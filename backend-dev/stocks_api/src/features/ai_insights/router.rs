use axum::{Router, routing::get};
use sqlx::PgPool;
use super::handlers;

pub fn ai_insights_routes(pool: PgPool) -> Router {
    Router::new()
        .route("/insights", get(handlers::get_ai_insights))
        .with_state(pool)
}
