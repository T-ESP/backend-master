use axum::{routing::{delete, get, post, put}, Router};
use sqlx::PgPool;

use crate::common::tenant_pool::TenantPoolManager;

use super::handler::{
    create_tenant, delete_tenant, delete_tenant_staff, get_all_tenants, get_tenant_by_id,
    get_tenant_staff, list_all_staff, seed_tenant, update_tenant, update_tenant_staff,
    verify_tenant,
};

#[derive(Clone)]
pub struct AdminState {
    pub pool: PgPool,
    pub tenant_pool_manager: TenantPoolManager,
}

pub fn tenant_admin_routes(pool: PgPool, tenant_pool_manager: TenantPoolManager) -> Router {
    Router::new()
        .route("/", post(create_tenant).get(get_all_tenants))
        .route("/staff", get(list_all_staff))
        .route("/:id", get(get_tenant_by_id).put(update_tenant).delete(delete_tenant))
        .route("/:id/seed", post(seed_tenant))
        .route("/:id/staff", get(get_tenant_staff))
        .route("/:id/staff/:staff_id", put(update_tenant_staff).delete(delete_tenant_staff))
        .with_state(AdminState { pool, tenant_pool_manager })
}

pub fn tenant_public_routes(pool: PgPool) -> Router {
    Router::new()
        .route("/tenants/verify/:slug", get(verify_tenant))
        .with_state(pool)
}
