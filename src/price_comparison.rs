use axum::{
    extract::Extension,
    http::StatusCode,
    response::Json,
    routing::post,
    Router,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;

use crate::optimization::{
    bloom_filter::BloomFilter,
    concurrent_trie::ConcurrentTrie,
};
use crate::services::deal_search::DealSearchService;

#[derive(Debug, Deserialize)]
pub struct PriceComparisonRequest {
    pub product_name: String,
    pub current_price: f64,
    pub current_platform: String,
    pub product_category: Option<String>,
    pub user_id: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct PlatformPrice {
    pub platform: String,
    pub price: f64,
    pub url: String,
    pub discount_percentage: f64,
    pub availability: bool,
    pub rating: Option<f64>,
    pub delivery_time: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PriceComparisonResponse {
    pub current_platform: PlatformPrice,
    pub alternatives: Vec<PlatformPrice>,
    pub best_deal: PlatformPrice,
    pub potential_savings: f64,
    pub recommendations: Vec<String>,
}

pub struct PriceComparisonService {
    pool: PgPool,
    search_service: Arc<DealSearchService>,
    bloom_filter: Arc<BloomFilter>,
    trie: Arc<ConcurrentTrie>,
}

impl PriceComparisonService {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool: pool.clone(),
            search_service: Arc::new(DealSearchService::new(pool.clone())),
            bloom_filter: Arc::new(BloomFilter::new(10000, 0.01)),
            trie: Arc::new(ConcurrentTrie::new()),
        }
    }

    pub async fn compare_prices(&self, req: PriceComparisonRequest) -> Result<PriceComparisonResponse, StatusCode> {
        // Use bloom filter for quick duplicate detection
        let _product_hash = format!("{}-{}", req.product_name, req.current_platform);
        
        // Search for the product across platforms using concurrent trie
        let search_results = self.search_service
            .search_product_across_platforms(&req.product_name)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        // Collect and sort results by price
        let mut sorted_results = Vec::new();
        
        // Convert search results to platform prices
        let mut alternatives = Vec::new();
        for result in search_results {
            let platform_price = PlatformPrice {
                platform: result.platform.clone(),
                price: result.price,
                url: result.url.clone(),
                discount_percentage: ((req.current_price - result.price) / req.current_price * 100.0).max(0.0),
                availability: result.in_stock,
                rating: result.rating,
                delivery_time: result.estimated_delivery,
            };
            
            sorted_results.push(platform_price.clone());
            alternatives.push(platform_price);
        }

        // Sort by price and get the best deal
        sorted_results.sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap_or(std::cmp::Ordering::Equal));
        
        let best_deal = sorted_results.into_iter().next()
            .unwrap_or_else(|| PlatformPrice {
                platform: req.current_platform.clone(),
                price: req.current_price,
                url: String::new(),
                discount_percentage: 0.0,
                availability: true,
                rating: None,
                delivery_time: None,
            });

        let current_platform = PlatformPrice {
            platform: req.current_platform.clone(),
            price: req.current_price,
            url: String::new(),
            discount_percentage: 0.0,
            availability: true,
            rating: None,
            delivery_time: None,
        };

        let potential_savings = req.current_price - best_deal.price;

        // Generate smart recommendations
        let mut recommendations = Vec::new();
        if potential_savings > 0.0 {
            recommendations.push(format!(
                "You can save ₹{:.2} by buying from {}",
                potential_savings, best_deal.platform
            ));
        }
        
        if best_deal.rating.unwrap_or(0.0) > 4.0 {
            recommendations.push(format!(
                "{} has excellent ratings ({:.1}/5)",
                best_deal.platform, best_deal.rating.unwrap()
            ));
        }

        // Note: In production, we'd use a thread-safe mutable bloom filter
        // For now, we'll skip caching in the bloom filter

        Ok(PriceComparisonResponse {
            current_platform,
            alternatives,
            best_deal,
            potential_savings,
            recommendations,
        })
    }
}

pub fn price_comparison_routes(pool: PgPool) -> Router {
    let service = Arc::new(PriceComparisonService::new(pool));
    
    Router::new()
        .route("/compare", post(compare_prices))
        .route("/bulk-compare", post(bulk_compare))
        .layer(Extension(service))
}

async fn compare_prices(
    Extension(service): Extension<Arc<PriceComparisonService>>,
    Json(payload): Json<PriceComparisonRequest>,
) -> Result<Json<PriceComparisonResponse>, StatusCode> {
    let response = service.compare_prices(payload).await?;
    Ok(Json(response))
}

#[derive(Debug, Deserialize)]
pub struct BulkComparisonRequest {
    pub products: Vec<PriceComparisonRequest>,
}

async fn bulk_compare(
    Extension(service): Extension<Arc<PriceComparisonService>>,
    Json(payload): Json<BulkComparisonRequest>,
) -> Result<Json<Vec<PriceComparisonResponse>>, StatusCode> {
    let mut results = Vec::new();
    
    // Process in parallel for better performance
    let futures: Vec<_> = payload.products
        .into_iter()
        .map(|product| {
            let service = service.clone();
            async move {
                service.compare_prices(product).await
            }
        })
        .collect();
    
    for future in futures {
        match future.await {
            Ok(result) => results.push(result),
            Err(_) => continue,
        }
    }
    
    Ok(Json(results))
}
