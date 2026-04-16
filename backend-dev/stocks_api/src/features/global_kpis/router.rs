use axum::{
    Router,
    routing::get
};
use sqlx::PgPool;
use super::handlers;

pub fn global_kpis_routes(pool: PgPool) -> Router {
    Router::new()
        .route("/global-performance", get(handlers::get_global_performance_kpis))
        .route("/category-analysis", get(handlers::get_category_analysis_kpis))
        .route("/supplier-analysis", get(handlers::get_supplier_analysis_kpis))
        .route("/catalog-health", get(handlers::get_catalog_health_kpis))
        .route("/abc-distribution", get(handlers::get_abc_distribution_kpis))
        .route("/trends", get(handlers::get_trends_kpis))
        .route("/operational-efficiency", get(handlers::get_operational_efficiency_kpis))
        .route("/price-analysis", get(handlers::get_price_analysis_kpis))
        .route("/top-flop", get(handlers::get_top_flop_kpis))
        .route("/forecast", get(handlers::get_forecast_kpis))
        .route("/time-series", get(handlers::get_time_series_kpis))
        .with_state(pool)
}
