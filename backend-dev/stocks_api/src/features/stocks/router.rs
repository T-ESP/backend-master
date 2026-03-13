use axum::{Router, routing::get};
use sqlx::PgPool;
use super::handlers;

pub fn stock_routes(pool: PgPool) -> Router {
    Router::new()
        .route("/out-of-stock", get(handlers::get_out_of_stock))
        .route("/low-stock", get(handlers::get_low_stock))
        .route("/soon-out-of-stock", get(handlers::get_soon_out_of_stock))
        .route("/overstock", get(handlers::get_overstock))
        .route("/alerts", get(handlers::get_stock_alerts))
        .route("/summary", get(handlers::get_stock_summary))
        .with_state(pool)
}