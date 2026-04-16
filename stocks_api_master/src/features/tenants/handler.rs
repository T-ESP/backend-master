use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Extension,
    Json,
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{
    error_codes,
    responses::{ErrorResponse, SuccessResponse},
};
use crate::common::security::Claims;

use super::{
    dto::{CreateTenantRequest, UpdateTenantRequest, TenantResponse},
    model::Tenant,
    service::{self, TenantServiceError},
};

fn tenant_to_response(t: Tenant) -> TenantResponse {
    TenantResponse {
        id: t.id,
        name: t.name,
        slug: t.slug,
        email: t.email,
        phone: t.phone,
        address: t.address,
        siret: t.siret,
        status: t.status,
    }
}

fn map_tenant_error(error: TenantServiceError) -> Response {
    match error {
        TenantServiceError::NotFound => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                error_codes::NOT_FOUND.to_string(),
                "Tenant not found".to_string(),
            )),
        ).into_response(),

        TenantServiceError::SlugAlreadyExists => (
            StatusCode::CONFLICT,
            Json(ErrorResponse::new(
                error_codes::ALREADY_EXISTS.to_string(),
                "Slug already exists".to_string(),
            )),
        ).into_response(),

        TenantServiceError::EmailAlreadyExists => (
            StatusCode::CONFLICT,
            Json(ErrorResponse::new(
                error_codes::ALREADY_EXISTS.to_string(),
                "Email already exists".to_string(),
            )),
        ).into_response(),

        TenantServiceError::Database(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                error_codes::DATABASE_ERROR.to_string(),
                format!("Database error: {}", err),
            )),
        ).into_response(),

        TenantServiceError::Security(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                error_codes::INTERNAL_SERVER_ERROR.to_string(),
                format!("Security error: {}", err),
            )),
        ).into_response(),

        TenantServiceError::DatabaseProvisioning(msg) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                error_codes::INTERNAL_SERVER_ERROR.to_string(),
                format!("Database provisioning error: {}", msg),
            )),
        ).into_response(),
    }
}

#[utoipa::path(
    post,
    path = "/admin/tenants",
    tag = "tenants",
    request_body = CreateTenantRequest,
    responses(
        (status = 201, description = "Tenant created successfully", body = inline(SuccessResponse<TenantResponse>)),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 409, description = "Conflict", body = ErrorResponse),
        (status = 500, description = "Internal error", body = ErrorResponse)
    )
)]
pub async fn create_tenant(
    State(pool): State<PgPool>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<CreateTenantRequest>,
) -> Response {
    if claims.role != "platform_admin" {
        return (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                error_codes::FORBIDDEN.to_string(),
                "Only platform admins can create tenants".to_string(),
            )),
        ).into_response();
    }

    let database_url = std::env::var("DATABASE_URL").unwrap_or_default();

    match service::create_tenant(
        &pool,
        &database_url,
        payload.name.clone(),
        payload.slug.clone(),
        payload.email.clone(),
        payload.phone.clone(),
        payload.address.clone(),
        payload.siret.clone(),
    ).await {
        Ok(created) => {
            let response = TenantResponse {
                id: created.id,
                name: payload.name,
                slug: payload.slug,
                email: created.email,
                phone: payload.phone,
                address: payload.address,
                siret: payload.siret,
                status: "active".to_string(),
            };
            (
                StatusCode::CREATED,
                Json(SuccessResponse::new(
                    response,
                    format!("Tenant created. Generated password: {}", created.generated_password),
                )),
            ).into_response()
        }
        Err(err) => map_tenant_error(err),
    }
}

#[utoipa::path(
    get,
    path = "/admin/tenants",
    tag = "tenants",
    responses(
        (status = 200, description = "List of tenants", body = inline(SuccessResponse<Vec<TenantResponse>>)),
        (status = 500, description = "Internal error", body = ErrorResponse)
    )
)]
pub async fn get_all_tenants(
    State(pool): State<PgPool>,
) -> Response {
    match service::get_all_tenants(&pool).await {
        Ok(tenants) => {
            let response: Vec<TenantResponse> = tenants.into_iter().map(tenant_to_response).collect();
            (
                StatusCode::OK,
                Json(SuccessResponse::new(response, "Tenants retrieved".to_string())),
            ).into_response()
        }
        Err(err) => map_tenant_error(err),
    }
}

#[utoipa::path(
    get,
    path = "/admin/tenants/{id}",
    tag = "tenants",
    params(("id" = Uuid, Path, description = "Tenant ID")),
    responses(
        (status = 200, description = "Tenant found", body = inline(SuccessResponse<TenantResponse>)),
        (status = 404, description = "Not found", body = ErrorResponse),
        (status = 500, description = "Internal error", body = ErrorResponse)
    )
)]
pub async fn get_tenant_by_id(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> Response {
    match service::get_tenant_by_id(&pool, id).await {
        Ok(tenant) => (
            StatusCode::OK,
            Json(SuccessResponse::new(tenant_to_response(tenant), "Tenant retrieved".to_string())),
        ).into_response(),
        Err(err) => map_tenant_error(err),
    }
}

#[utoipa::path(
    put,
    path = "/admin/tenants/{id}",
    tag = "tenants",
    request_body = UpdateTenantRequest,
    params(("id" = Uuid, Path, description = "Tenant ID")),
    responses(
        (status = 200, description = "Tenant updated", body = inline(SuccessResponse<TenantResponse>)),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "Not found", body = ErrorResponse),
        (status = 500, description = "Internal error", body = ErrorResponse)
    )
)]
pub async fn update_tenant(
    State(pool): State<PgPool>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateTenantRequest>,
) -> Response {
    if claims.role != "platform_admin" {
        return (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                error_codes::FORBIDDEN.to_string(),
                "Only platform admins can update tenants".to_string(),
            )),
        ).into_response();
    }

    match service::update_tenant(
        &pool, id,
        payload.name, payload.email, payload.phone,
        payload.address, payload.siret, payload.status,
    ).await {
        Ok(tenant) => (
            StatusCode::OK,
            Json(SuccessResponse::new(tenant_to_response(tenant), "Tenant updated".to_string())),
        ).into_response(),
        Err(err) => map_tenant_error(err),
    }
}

#[utoipa::path(
    delete,
    path = "/admin/tenants/{id}",
    tag = "tenants",
    params(("id" = Uuid, Path, description = "Tenant ID")),
    responses(
        (status = 200, description = "Tenant deleted", body = ErrorResponse),
        (status = 404, description = "Not found", body = ErrorResponse),
        (status = 500, description = "Internal error", body = ErrorResponse)
    )
)]
pub async fn delete_tenant(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> Response {
    match service::delete_tenant(&pool, id).await {
        Ok(()) => (
            StatusCode::OK,
            Json(SuccessResponse::new(
                serde_json::Value::Null,
                "Tenant deleted".to_string(),
            )),
        ).into_response(),
        Err(err) => map_tenant_error(err),
    }
}

#[utoipa::path(
    post,
    path = "/admin/tenants/{id}/seed",
    tag = "tenants",
    params(("id" = Uuid, Path, description = "Tenant ID")),
    responses(
        (status = 200, description = "Seed started successfully"),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "Tenant not found", body = ErrorResponse),
        (status = 500, description = "Internal error", body = ErrorResponse)
    )
)]
pub async fn seed_tenant(
    State(pool): State<PgPool>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<Uuid>,
) -> Response {
    if claims.role != "platform_admin" {
        return (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                error_codes::FORBIDDEN.to_string(),
                "Only platform admins can seed tenants".to_string(),
            )),
        ).into_response();
    }

    let database_url = std::env::var("DATABASE_URL").unwrap_or_default();

    match service::seed_tenant(&pool, &database_url, id).await {
        Ok(()) => (
            StatusCode::OK,
            Json(SuccessResponse::new(
                serde_json::Value::Null,
                "Seed completed successfully".to_string(),
            )),
        ).into_response(),
        Err(err) => map_tenant_error(err),
    }
}

#[utoipa::path(
    get,
    path = "/tenants/verify/{slug}",
    tag = "tenants",
    params(("slug" = String, Path, description = "Tenant slug")),
    responses(
        (status = 200, description = "Tenant exists", body = inline(SuccessResponse<TenantResponse>)),
        (status = 404, description = "Tenant not found", body = ErrorResponse),
        (status = 500, description = "Internal error", body = ErrorResponse)
    )
)]
pub async fn verify_tenant(
    State(pool): State<PgPool>,
    Path(slug): Path<String>,
) -> Response {
    match service::get_tenant_by_slug(&pool, &slug).await {
        Ok(tenant) => (
            StatusCode::OK,
            Json(SuccessResponse::new(tenant_to_response(tenant), "Tenant verified".to_string())),
        ).into_response(),
        Err(TenantServiceError::NotFound) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                error_codes::NOT_FOUND.to_string(),
                "Tenant not found".to_string(),
            )),
        ).into_response(),
        Err(err) => map_tenant_error(err),
    }
}
