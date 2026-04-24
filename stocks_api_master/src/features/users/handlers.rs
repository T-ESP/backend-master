use axum::{Json, extract::{Path, Extension}, response::{IntoResponse, Response}};
use axum::http::StatusCode;
use sqlx::PgPool;
use crate::common::{responses::{SuccessResponse, ErrorResponse}, error_codes};
use super::{dto::{CreateUserRequest, UpdateUserRequest}, services};

pub async fn get_users(Extension(pool): Extension<PgPool>) -> Response {
    match services::get_all_users(&pool).await {
        Ok(users) => (
            StatusCode::OK,
            Json(SuccessResponse::new(users, "Users retrieved successfully".to_string()))
        ).into_response(),
        Err(e) => {
            eprintln!("Database error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    error_codes::DATABASE_ERROR.to_string(),
                    "Failed to retrieve users".to_string()
                ))
            ).into_response()
        }
    }
}

pub async fn get_user_by_id(
    Path((_commerce_id, id)): Path<(String, i32)>,
    Extension(pool): Extension<PgPool>,
) -> Response {
    match services::get_user_by_id(&pool, id).await {
        Ok(Some(user)) => (
            StatusCode::OK,
            Json(SuccessResponse::new(user, "User retrieved successfully".to_string()))
        ).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                error_codes::NOT_FOUND.to_string(),
                format!("Utilisateur {} non trouvé", id)
            ))
        ).into_response(),
        Err(e) => {
            eprintln!("Database error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    error_codes::DATABASE_ERROR.to_string(),
                    "Failed to retrieve user".to_string()
                ))
            ).into_response()
        }
    }
}

pub async fn get_user_by_fidelity_code(
    Path((_commerce_id, code)): Path<(String, String)>,
    Extension(pool): Extension<PgPool>,
) -> Response {
    match services::get_user_by_fidelity_code(&pool, &code).await {
        Ok(Some(user)) => (
            StatusCode::OK,
            Json(SuccessResponse::new(user, "User retrieved successfully".to_string()))
        ).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                error_codes::NOT_FOUND.to_string(),
                format!("Aucun utilisateur avec le code de fidélité {}", code)
            ))
        ).into_response(),
        Err(e) => {
            eprintln!("Database error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    error_codes::DATABASE_ERROR.to_string(),
                    "Failed to retrieve user".to_string()
                ))
            ).into_response()
        }
    }
}

pub async fn create_user(Extension(pool): Extension<PgPool>, Json(payload): Json<CreateUserRequest>) -> Response {
    match services::create_user(&pool, payload).await {
        Ok(user) => (
            StatusCode::CREATED,
            Json(SuccessResponse::new(user, "User created successfully".to_string()))
        ).into_response(),
        Err(e) => {
            eprintln!("Database error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    error_codes::DATABASE_ERROR.to_string(),
                    "Failed to create user".to_string()
                ))
            ).into_response()
        }
    }
}

pub async fn update_user(Path((_commerce_id, id)): Path<(String, i32)>, Extension(pool): Extension<PgPool>, Json(payload): Json<UpdateUserRequest>) -> Response {
    match services::update_user(&pool, id, payload).await {
        Ok(_) => (
            StatusCode::OK,
            Json(SuccessResponse::new("User updated successfully".to_string(), "User updated".to_string()))
        ).into_response(),
        Err(e) => {
            eprintln!("Database error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    error_codes::DATABASE_ERROR.to_string(),
                    "Failed to update user".to_string()
                ))
            ).into_response()
        }
    }
}

pub async fn delete_user(Path((_commerce_id, id)): Path<(String, i32)>, Extension(pool): Extension<PgPool>) -> Response {
    match services::delete_user(&pool, id).await {
        Ok(_) => (
            StatusCode::NO_CONTENT,
            Json(SuccessResponse::new("User deleted successfully".to_string(), "User deleted".to_string()))
        ).into_response(),
        Err(e) => {
            eprintln!("Database error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    error_codes::DATABASE_ERROR.to_string(),
                    "Failed to delete user".to_string()
                ))
            ).into_response()
        }
    }
}
