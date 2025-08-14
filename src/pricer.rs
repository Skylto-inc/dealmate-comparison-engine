use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct PricePredictionRequest {
    pub product_name: String,
    pub current_price: f64,
    pub category: String,
    pub historical_prices: Option<Vec<HashMap<String, serde_json::Value>>>,
}

#[derive(Debug, Serialize)]
pub struct PricePredictionResponse {
    pub predicted_price_trend: String,
    pub confidence: f64,
    pub price_forecast: HashMap<String, f64>,
    pub recommendation: String,
}

pub fn predict_price(request: PricePredictionRequest) -> PricePredictionResponse {
    let category_trends: HashMap<String, (f64, bool)> = [
        ("electronics".to_string(), (0.3, true)),
        ("fashion".to_string(), (0.5, true)),
        ("books".to_string(), (0.1, false)),
        ("home".to_string(), (0.2, false)),
    ]
    .iter()
    .cloned()
    .collect();

    let category_info = category_trends
        .get(&request.category.to_lowercase())
        .unwrap_or(&(0.25, false));

    let mut trend_direction = "stable".to_string();
    if let Some(historical_prices) = &request.historical_prices {
        if historical_prices.len() > 1 {
            let prices: Vec<f64> = historical_prices
                .iter()
                .filter_map(|p| p.get("price").and_then(|v| v.as_f64()))
                .collect();
            if prices.len() > 1 {
                if *prices.last().unwrap() > *prices.first().unwrap() * 1.1 {
                    trend_direction = "increasing".to_string();
                } else if *prices.last().unwrap() < *prices.first().unwrap() * 0.9 {
                    trend_direction = "decreasing".to_string();
                }
            }
        }
    }

    let base_price = request.current_price;
    let (_volatility, _seasonal) = category_info;

    let multipliers = if trend_direction == "increasing" {
        [1.02, 1.05, 1.1]
    } else if trend_direction == "decreasing" {
        [0.98, 0.95, 0.9]
    } else {
        [1.0, 1.0, 1.0]
    };

    let price_forecast: HashMap<String, f64> = [
        ("1_week".to_string(), (base_price * multipliers[0]).round()),
        ("1_month".to_string(), (base_price * multipliers[1]).round()),
        ("3_months".to_string(), (base_price * multipliers[2]).round()),
    ]
    .iter()
    .cloned()
    .collect();

    let recommendation = if trend_direction == "decreasing" {
        "Wait for better deals - price likely to drop".to_string()
    } else if trend_direction == "increasing" {
        "Good time to buy - price may increase".to_string()
    } else {
        "Stable pricing - buy when needed".to_string()
    };

    let confidence: f64 = 0.7 + if request.historical_prices.is_some() { 0.2 } else { 0.0 };

    PricePredictionResponse {
        predicted_price_trend: trend_direction,
        confidence: confidence.min(1.0),
        price_forecast,
        recommendation,
    }
}
