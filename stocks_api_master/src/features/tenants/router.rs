use axum::{routing::{delete, get, post}, Router};
use sqlx::PgPool;

use super::handler::{create_tenant, delete_tenant, get_all_tenants, get_tenant_by_id, verify_tenant};

pub fn tenant_admin_routes(pool: PgPool) -> Router {
    Router::new()
        .route("/", post(create_tenant).get(get_all_tenants))
        .route("/:id", get(get_tenant_by_id).delete(delete_tenant))
        .with_state(pool)
}

pub fn tenant_public_routes(pool: PgPool) -> Router {
    Router::new()
        .route("/tenants/verify/:slug", get(verify_tenant))
        .with_state(pool)
}
