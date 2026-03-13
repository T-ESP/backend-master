use axum::{Router, routing::{get, put, delete}};
use sqlx::PgPool;
use super::handlers;

pub fn alert_routes(pool: PgPool) -> Router {
    Router::new()
        .route("/", get(handlers::get_alerts))
        .route("/summary", get(handlers::get_alert_summary))
        .route("/bulk-status", put(handlers::bulk_update_status))
        .route("/cleanup", delete(handlers::cleanup_alerts))
        .route("/:id", get(handlers::get_alert_by_id))
        .route("/:id/status", put(handlers::update_alert_status))
        .with_state(pool)
}
