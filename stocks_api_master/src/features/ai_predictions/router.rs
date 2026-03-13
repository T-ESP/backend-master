use axum::{Router, routing::get};
use super::handlers;

pub fn ai_prediction_routes() -> Router {
    Router::new()
        .route("/forecasts", get(handlers::get_forecasts))
        .route("/forecasts/:product_id", get(handlers::get_forecast_by_product))
        .route("/classifications", get(handlers::get_classifications))
        .route("/classifications/:product_id", get(handlers::get_classification_by_product))
        .route("/clusters", get(handlers::get_clusters))
        .route("/clusters/:product_id", get(handlers::get_cluster_by_product))
        .route("/supplier-scores", get(handlers::get_supplier_scores))
        .route("/supplier-scores/:supplier_id", get(handlers::get_supplier_score))
        .route("/price-suggestions", get(handlers::get_price_suggestions))
        .route("/price-anomalies", get(handlers::get_price_anomalies))
        .route("/sales-anomalies", get(handlers::get_sales_anomalies))
        .route("/urgent-restocks", get(handlers::get_urgent_restocks))
}
