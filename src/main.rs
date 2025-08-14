use axum::{routing::get, Router, Json};
use serde_json::{json, Value};
use tower_http::cors::CorsLayer;

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/health", get(health))
        .route("/compare/products", get(compare_products))
        .route("/compare/prices", get(compare_prices))
        .route("/compare/vendors", get(compare_vendors))
        .layer(CorsLayer::permissive());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3007").await.unwrap();
    println!("⚖️ Comparison Service running on port 3007");
    axum::serve(listener, app).await.unwrap();
}

async fn health() -> Json<Value> {
    Json(json!({"status": "healthy", "service": "comparison-service", "features": ["price_comparison", "vendor_comparison"]}))
}

async fn compare_products() -> Json<Value> {
    Json(json!({
        "comparisons": [
            {"product": "Laptop", "vendor": "Amazon", "price": 999.99, "rating": 4.5},
            {"product": "Laptop", "vendor": "Best Buy", "price": 1099.99, "rating": 4.3}
        ],
        "service": "comparison-service"
    }))
}

async fn compare_prices() -> Json<Value> {
    Json(json!({
        "price_comparison": {
            "lowest": 999.99,
            "highest": 1299.99,
            "average": 1149.99,
            "vendors": 5
        },
        "service": "comparison-service"
    }))
}

async fn compare_vendors() -> Json<Value> {
    Json(json!({
        "vendor_comparison": [
            {"vendor": "Amazon", "rating": 4.5, "delivery": "2 days", "price_rank": 1},
            {"vendor": "Best Buy", "rating": 4.3, "delivery": "3 days", "price_rank": 2}
        ],
        "service": "comparison-service"
    }))
}
