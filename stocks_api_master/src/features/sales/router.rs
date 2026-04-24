use axum::{routing::get, Router};
use super::handlers;

pub fn sales_routes() -> Router {
    Router::new()
        .route("/total", get(handlers::get_total_revenue))
        .route("/evolution", get(handlers::get_evolution))
        .route("/comparison", get(handlers::get_comparison))
        .route("/average-basket", get(handlers::get_average_basket))
        .route("/average-basket-by-client-type", get(handlers::get_average_basket_by_client_type))
}
