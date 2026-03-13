use serde::Serialize;
use utoipa::{ToSchema, IntoParams};
use serde::Deserialize;
use rust_decimal::Decimal;

// ==================== Demand Forecasts ====================

#[derive(Debug, Serialize, ToSchema)]
pub struct ForecastResponse {
    pub id: i32,
    pub product_id: i32,
    pub product_name: Option<String>,
    pub forecast_date: chrono::DateTime<chrono::Utc>,
    pub forecast_days: i32,
    pub total_predicted_demand: Decimal,
    pub avg_daily_demand: Decimal,
    pub current_stock: Option<i32>,
    pub recommended_stock: Option<i32>,
    pub reorder_quantity: Option<i32>,
    pub days_until_stockout: Option<i32>,
    pub urgency: Option<String>,
    pub mape: Option<Decimal>,
    pub rmse: Option<Decimal>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

// ==================== Product Classifications ====================

#[derive(Debug, Serialize, ToSchema)]
pub struct ClassificationResponse {
    pub id: i32,
    pub product_id: i32,
    pub product_name: Option<String>,
    pub abc_class: String,
    pub xyz_class: String,
    pub combined_class: String,
    pub total_revenue: Option<Decimal>,
    pub revenue_contribution_pct: Option<Decimal>,
    pub total_units_sold: Option<i32>,
    pub coefficient_of_variation: Option<Decimal>,
    pub strategy: Option<String>,
    pub priority: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

// ==================== Product Clusters ====================

#[derive(Debug, Serialize, ToSchema)]
pub struct ClusterResponse {
    pub id: i32,
    pub product_id: i32,
    pub product_name: Option<String>,
    pub cluster_id: i32,
    pub cluster_name: Option<String>,
    pub revenue_score: Option<Decimal>,
    pub variability_score: Option<Decimal>,
    pub trend_score: Option<Decimal>,
    pub seasonality_score: Option<Decimal>,
    pub frequency_score: Option<Decimal>,
    pub margin_score: Option<Decimal>,
    pub distance_to_centroid: Option<Decimal>,
    pub n_clusters: Option<i32>,
    pub silhouette_score: Option<Decimal>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

// ==================== Supplier Scores ====================

#[derive(Debug, Serialize, ToSchema)]
pub struct SupplierScoreResponse {
    pub id: i32,
    pub supplier_id: i32,
    pub supplier_name: Option<String>,
    pub overall_score: Decimal,
    pub delivery_score: Option<Decimal>,
    pub quality_score: Option<Decimal>,
    pub lead_time_score: Option<Decimal>,
    pub fulfillment_score: Option<Decimal>,
    pub rating: Option<String>,
    pub total_restocks: Option<i32>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

// ==================== Price Suggestions ====================

#[derive(Debug, Serialize, ToSchema)]
pub struct PriceSuggestionResponse {
    pub id: i32,
    pub product_id: i32,
    pub product_name: Option<String>,
    pub suggested_price: Decimal,
    pub current_price: Decimal,
    pub reason: Option<String>,
    pub confidence: Option<f64>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

// ==================== Price Anomalies ====================

#[derive(Debug, Serialize, ToSchema)]
pub struct PriceAnomalyResponse {
    pub id: i32,
    pub product_id: i32,
    pub product_name: Option<String>,
    pub current_price: Decimal,
    pub expected_price: Decimal,
    pub anomaly_score: Option<f64>,
    pub is_anomaly: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

// ==================== Sales Anomalies ====================

#[derive(Debug, Serialize, ToSchema)]
pub struct SalesAnomalyResponse {
    pub id: i32,
    pub product_id: i32,
    pub product_name: Option<String>,
    pub sales_volume: i32,
    pub expected_sales: Option<f64>,
    pub anomaly_score: Option<f64>,
    pub is_anomaly: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

// ==================== Urgent Restocks ====================

#[derive(Debug, Serialize, ToSchema)]
pub struct UrgentRestockResponse {
    pub product_id: i32,
    pub product_name: Option<String>,
    pub current_stock: Option<i32>,
    pub recommended_stock: Option<i32>,
    pub reorder_quantity: Option<i32>,
    pub days_until_stockout: Option<i32>,
    pub urgency: Option<String>,
    pub avg_daily_demand: Option<Decimal>,
    pub forecast_date: chrono::DateTime<chrono::Utc>,
}

// ==================== Query Params ====================

#[derive(Debug, Deserialize, IntoParams)]
pub struct PaginationParams {
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct AnomalyParams {
    pub limit: Option<i32>,
    pub only_anomalies: Option<bool>,
}
