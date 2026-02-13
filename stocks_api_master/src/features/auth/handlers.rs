use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use sqlx::PgPool;

use crate::common::{
    error_codes,
    responses::{ErrorResponse, SuccessResponse},
    security,
};

use super::{
    dto::{LoginRequest, LoginResponse, RegisterRequest},
    services::{self, AuthServiceError},
};

#[utoipa::path(
    post,
    path = "/auth/login",
    tag = "auth",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login successful", body = inline(SuccessResponse<LoginResponse>)),
        (status = 401, description = "Invalid credentials", body = ErrorResponse),
        (status = 500, description = "Authentication service error", body = ErrorResponse)
    )
)]
pub async fn login(
    State(pool): State<PgPool>,
    Json(payload): Json<LoginRequest>,
) -> Response {

    match services::authenticate_user(&pool, &payload.email, &payload.password).await {
        Ok(user) => {
            match security::generate_jwt(&user.id, &user.email, &user.role) {
                Ok(token) => (
                    StatusCode::OK,
                    Json(SuccessResponse::new(
                        LoginResponse { token },
                        "Login successful".to_string(),
                    )),
                ).into_response(),

                Err(err) => {
                    eprintln!("JWT generation error for {}: {}", user.email, err);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse::new(
                            error_codes::INTERNAL_SERVER_ERROR.to_string(),
                            "Failed to generate token".to_string(),
                        )),
                    ).into_response()
                }
            }
        }

        Err(error) => {
            eprintln!("Authentication error for {}: {:?}", payload.email, error);

            match error {
                AuthServiceError::InvalidCredentials => (
                    StatusCode::UNAUTHORIZED,
                    Json(ErrorResponse::new(
                        error_codes::UNAUTHORIZED.to_string(),
                        "Invalid email or password".to_string(),
                    )),
                ).into_response(),

                AuthServiceError::Database(err) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new(
                        error_codes::DATABASE_ERROR.to_string(),
                        format!("Database error: {}", err),
                    )),
                ).into_response(),

                AuthServiceError::Security(err) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new(
                        error_codes::INTERNAL_SERVER_ERROR.to_string(),
                        format!("Security error: {}", err),
                    )),
                ).into_response(),

                _ => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new(
                        error_codes::INTERNAL_SERVER_ERROR.to_string(),
                        "Authentication failed".to_string(),
                    )),
                ).into_response(),
            }
        }
    }
}

#[utoipa::path(
    post,
    path = "/auth/register",
    tag = "auth",
    request_body = RegisterRequest,
    responses(
        (status = 201, description = "Platform admin registered successfully", body = inline(SuccessResponse<String>)),
        (status = 409, description = "Email already exists", body = ErrorResponse),
        (status = 500, description = "Registration error", body = ErrorResponse)
    )
)]
pub async fn register(
    State(pool): State<PgPool>,
    Json(payload): Json<RegisterRequest>,
) -> Response {

    match services::register_platform_admin(
        &pool,
        &payload.email,
        &payload.password,
    ).await {

        Ok(_) => (
            StatusCode::CREATED,
            Json(SuccessResponse::new(
                "Platform admin registered successfully".to_string(),
                "Registration successful".to_string(),
            )),
        ).into_response(),

        Err(error) => {
            eprintln!("Registration error for {}: {:?}", payload.email, error);

            match error {
                AuthServiceError::EmailAlreadyExists => (
                    StatusCode::CONFLICT,
                    Json(ErrorResponse::new(
                        error_codes::ALREADY_EXISTS.to_string(),
                        "Email already exists".to_string(),
                    )),
                ).into_response(),

                AuthServiceError::Database(err) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new(
                        error_codes::DATABASE_ERROR.to_string(),
                        format!("Database error: {}", err),
                    )),
                ).into_response(),

                AuthServiceError::Security(err) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new(
                        error_codes::INTERNAL_SERVER_ERROR.to_string(),
                        format!("Security error: {}", err),
                    )),
                ).into_response(),

                _ => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new(
                        error_codes::INTERNAL_SERVER_ERROR.to_string(),
                        "Registration failed".to_string(),
                    )),
                ).into_response(),
            }
        }
    }
}
