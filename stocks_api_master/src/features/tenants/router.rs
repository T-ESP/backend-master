use axum::{routing::post, Router};
use sqlx::PgPool;

use super::handler::create_tenant;

pub fn tenant_routes(pool: PgPool) -> Router {
    Router::new()
        .route("/", post(create_tenant))
        .with_state(pool)
}
