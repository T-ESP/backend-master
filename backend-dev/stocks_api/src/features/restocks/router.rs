use axum::{
    Router,
    routing::{get, post, put}
};
use sqlx::PgPool;
use super::handlers;

pub fn restock_routes(pool: PgPool) -> Router {
    Router::new()
        .route("/", get(handlers::get_restocks).post(handlers::create_restock))
        .route("/with-supplier", get(handlers::get_restocks_with_supplier))
        .route("/product/:product_id", get(handlers::get_restocks_by_product))
        .route("/stats/global", get(handlers::get_restock_stats))
        .route("/stats/product/:product_id", get(handlers::get_restock_stats_by_product))
        .route("/:id", get(handlers::get_restock_by_id).put(handlers::update_restock))
        .with_state(pool)
}
