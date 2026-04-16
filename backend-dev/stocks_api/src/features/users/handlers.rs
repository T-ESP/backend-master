use axum::{Json, extract::{Path, State}, response::{IntoResponse, Response}};
use axum::http::StatusCode;
use sqlx::PgPool;
use crate::common::{responses::{SuccessResponse, ErrorResponse}, error_codes};
use super::{dto::{CreateUserRequest, UpdateUserRequest, UserResponse}, services};

#[utoipa::path(
    get,
    path = "/users",
    tag = "users",
    responses(
        (status = 200, description = "Users retrieved successfully", body = inline(SuccessResponse<Vec<UserResponse>>)),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn get_users(State(pool): State<PgPool>) -> Response {
    match services::get_all_users(&pool).await {
        Ok(users) => (
            StatusCode::OK,
            Json(SuccessResponse::new(users, "Users retrieved successfully".to_string()))
        ).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                error_codes::DATABASE_ERROR.to_string(),
                "Failed to retrieve users".to_string()
            ))
        ).into_response()
    }
}

#[utoipa::path(
    post,
    path = "/users",
    tag = "users",
    request_body = CreateUserRequest,
    responses(
        (status = 201, description = "User created successfully", body = inline(SuccessResponse<String>)),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn create_user(State(pool): State<PgPool>, Json(payload): Json<CreateUserRequest>) -> Response {
    match services::create_user(&pool, payload).await {
        Ok(_) => (
            StatusCode::CREATED,
            Json(SuccessResponse::new("User created successfully".to_string(), "User created".to_string()))
        ).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                error_codes::DATABASE_ERROR.to_string(),
                "Failed to create user".to_string()
            ))
        ).into_response()
    }
}

#[utoipa::path(
    put,
    path = "/users/{id}",
    tag = "users",
    params(
        ("id" = i32, Path, description = "User ID")
    ),
    request_body = UpdateUserRequest,
    responses(
        (status = 200, description = "User updated successfully", body = inline(SuccessResponse<String>)),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn update_user(Path(id): Path<i32>, State(pool): State<PgPool>, Json(payload): Json<UpdateUserRequest>) -> Response {
    match services::update_user(&pool, id, payload).await {
        Ok(_) => (
            StatusCode::OK,
            Json(SuccessResponse::new("User updated successfully".to_string(), "User updated".to_string()))
        ).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                error_codes::DATABASE_ERROR.to_string(),
                "Failed to update user".to_string()
            ))
        ).into_response()
    }
}

#[utoipa::path(
    delete,
    path = "/users/{id}",
    tag = "users",
    params(
        ("id" = i32, Path, description = "User ID")
    ),
    responses(
        (status = 204, description = "User deleted successfully", body = inline(SuccessResponse<String>)),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn delete_user(Path(id): Path<i32>, State(pool): State<PgPool>) -> Response {
    match services::delete_user(&pool, id).await {
        Ok(_) => (
            StatusCode::NO_CONTENT,
            Json(SuccessResponse::new("User deleted successfully".to_string(), "User deleted".to_string()))
        ).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                error_codes::DATABASE_ERROR.to_string(),
                "Failed to delete user".to_string()
            ))
        ).into_response()
    }
}