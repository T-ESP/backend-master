use axum::{
    extract::{State, Request},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use sqlx::PgPool;

use crate::common::{
    error_codes,
    responses::{ErrorResponse, SuccessResponse},
};
use crate::features::auth::middleware::get_claims;

use super::{
    dto::{CreateTenantRequest, TenantResponse},
    services::{self, TenantServiceError},
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
    req: Request,
    Json(payload): Json<CreateTenantRequest>,
) -> Response {

    // 🔐 Vérifie rôle
    let claims = match get_claims(&req) {
        Some(c) => c,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse::new(
                    error_codes::UNAUTHORIZED.to_string(),
                    "Unauthorized".to_string(),
                )),
            ).into_response()
        }
    };

    if claims.role != "platform_admin" {
        return (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                error_codes::FORBIDDEN.to_string(),
                "Only platform admins can create tenants".to_string(),
            )),
        ).into_response();
    }

    match services::create_tenant(
        &pool,
        payload.name,
        payload.slug,
        payload.email,
        payload.phone,
        payload.address,
        payload.siret,
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
