use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Extension,
    Json,
};
use sqlx::PgPool;

use crate::common::{
    error_codes,
    responses::{ErrorResponse, SuccessResponse},
};
use crate::common::security::Claims;

use super::{
    dto::{CreateTenantRequest, TenantResponse},
    service::{self, TenantServiceError},
};

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

    match service::create_tenant(
        &pool,
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

        Err(error) => {
            match error {
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
            }
        }
    }
}
