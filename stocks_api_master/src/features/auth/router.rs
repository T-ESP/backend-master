use axum::{routing::post, Router};
use sqlx::PgPool;

use crate::common::tenant_pool::TenantPoolManager;

use super::handlers;

#[derive(Clone)]
pub struct AuthState {
    pub pool: PgPool,
    pub tenant_pool_manager: TenantPoolManager,
}

pub fn auth_routes(pool: PgPool, tenant_pool_manager: TenantPoolManager) -> Router {
    Router::new()
        .route("/login", post(handlers::login))
        .route("/register", post(handlers::register))
        .with_state(AuthState { pool, tenant_pool_manager })
}
