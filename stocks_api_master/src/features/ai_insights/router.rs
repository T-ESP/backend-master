use axum::{Router, routing::get};
use super::handlers;

pub fn ai_insights_routes() -> Router {
    Router::new()
        .route("/insights", get(handlers::get_ai_insights))
}
