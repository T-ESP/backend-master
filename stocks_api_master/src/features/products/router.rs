use axum::{
    Router,
    routing::{get, post, put, patch, delete}
};
use super::handlers;
use super::kpis_handlers;

pub fn product_routes() -> Router {
    Router::new()
        // Routes principales
        .route("/", get(handlers::get_products).post(handlers::create_product))
        .route("/light", get(handlers::search_products_light))
        .route("/:id/stock", get(handlers::get_product_stock).patch(handlers::update_stock))
        .route("/:id", get(handlers::get_product_by_id).put(handlers::update_product).delete(handlers::delete_product))
        .route("/reference/:reference", get(handlers::get_product_by_reference))
        // Routes KPIs
        .route("/:id/kpis/pricing-margin", get(kpis_handlers::get_pricing_margin_kpis))
        .route("/:id/kpis/stock-availability", get(kpis_handlers::get_stock_availability_kpis))
        .route("/:id/kpis/sales-rotation", get(kpis_handlers::get_sales_rotation_kpis))
        .route("/:id/kpis/profitability", get(kpis_handlers::get_profitability_kpis))
        .route("/:id/kpis/restock", get(kpis_handlers::get_restock_kpis))
        .route("/:id/kpis/predictions-alerts", get(kpis_handlers::get_predictions_alerts_kpis))
        .route("/:id/kpis/scoring-classification", get(kpis_handlers::get_scoring_classification_kpis))
        .route("/:id/kpis/comparative", get(kpis_handlers::get_comparative_kpis))
        .route("/:id/kpis/price-evolution", get(kpis_handlers::get_price_evolution))
        .route("/:id/kpis/all", get(kpis_handlers::get_all_product_kpis)) // ✅ ADD THIS LINE
}
