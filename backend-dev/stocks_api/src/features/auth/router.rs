use axum::{routing::post, Router};
use sqlx::PgPool;

use super::handlers;

pub fn auth_routes(pool: PgPool) -> Router {
    Router::new()
        .route("/login", post(handlers::login))
        .route("/register", post(handlers::register))
        .with_state(pool)
}

