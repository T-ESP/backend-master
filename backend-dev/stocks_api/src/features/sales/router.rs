use axum::{routing::get, Router};
use sqlx::PgPool;
use super::handlers;

pub fn sales_routes(pool: PgPool) -> Router {
    Router::new()
        .route("/total", get(handlers::get_total_revenue))
        .route("/evolution", get(handlers::get_evolution))
        .route("/average-basket", get(handlers::get_average_basket))
        .route("/average-basket-by-client-type",get(handlers::get_average_basket_by_client_type),)
        .with_state(pool)
}
