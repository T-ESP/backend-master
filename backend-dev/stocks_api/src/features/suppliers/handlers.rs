use axum::{Json, extract::{Path, State, Query}, response::{IntoResponse, Response}};
use axum::http::StatusCode;
use sqlx::PgPool;
use validator::Validate;
use crate::common::{responses::{SuccessResponse, ErrorResponse}, error_codes};
use super::{dto::{SupplierResponse, CreateSupplierRequest, UpdateSupplierRequest}, services};

#[utoipa::path(
    get,
    path = "/suppliers",
    tag = "suppliers",
    responses(
        (status = 200, description = "Suppliers retrieved successfully", body = SuccessResponse<Vec<SupplierResponse>>),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn get_all_suppliers(
    State(pool): State<PgPool>,
) -> Response {
    match services::get_all_suppliers(&pool).await {
        Ok(suppliers) => (
            StatusCode::OK,
            Json(SuccessResponse::new(suppliers, "Suppliers retrieved successfully".to_string()))
        ).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                error_codes::DATABASE_ERROR.to_string(),
                "Failed to retrieve suppliers".to_string()
            ))
        ).into_response()
    }
}

#[utoipa::path(
    post,
    path = "/suppliers",
    tag = "suppliers",
    request_body = CreateSupplierRequest,
    responses(
        (status = 201, description = "Supplier created successfully", body = SuccessResponse<String>),
        (status = 400, description = "Validation error", body = ErrorResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn create_supplier(
    State(pool): State<PgPool>,
    Json(payload): Json<CreateSupplierRequest>,
) -> Response {
    if let Err(validation_errors) = payload.validate() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                error_codes::VALIDATION_ERROR.to_string(),
                format!("Validation failed: {}", validation_errors)
            ))
        ).into_response();
    }

    match services::create_supplier(&pool, payload).await {
        Ok(_) => (
            StatusCode::CREATED,
            Json(SuccessResponse::new("Supplier created".to_string(), "Supplier created successfully".to_string()))
        ).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                error_codes::DATABASE_ERROR.to_string(),
                "Failed to create supplier".to_string()
            ))
        ).into_response()
    }
}

#[utoipa::path(
    get,
    path = "/suppliers/{id}",
    tag = "suppliers",
    params(
        ("id" = i32, Path, description = "Supplier ID")
    ),
    responses(
        (status = 200, description = "Supplier retrieved successfully", body = SuccessResponse<SupplierResponse>),
        (status = 400, description = "Invalid supplier ID", body = ErrorResponse),
        (status = 404, description = "Supplier not found", body = ErrorResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn get_supplier_by_id(
    Path(id): Path<i32>,
    State(pool): State<PgPool>,
) -> Response {
    if id <= 0 {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                error_codes::INVALID_INPUT.to_string(),
                "Invalid supplier ID".to_string()
            ))
        ).into_response();
    }

    match services::get_supplier_by_id(&pool, id).await {
        Ok(Some(supplier)) => (
            StatusCode::OK,
            Json(SuccessResponse::new(Some(supplier), "Supplier retrieved successfully".to_string()))
        ).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                error_codes::NOT_FOUND.to_string(),
                "Supplier not found".to_string()
            ))
        ).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                error_codes::DATABASE_ERROR.to_string(),
                "Failed to retrieve supplier".to_string()
            ))
        ).into_response()
    }
}

#[utoipa::path(
    get,
    path = "/suppliers/by-email",
    tag = "suppliers",
    params(
        ("email" = String, Query, description = "Supplier email address")
    ),
    responses(
        (status = 200, description = "Supplier retrieved successfully", body = SuccessResponse<SupplierResponse>),
        (status = 400, description = "Email parameter required", body = ErrorResponse),
        (status = 404, description = "Supplier not found", body = ErrorResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn get_supplier_by_email(
    Query(params): Query<std::collections::HashMap<String, String>>,
    State(pool): State<PgPool>,
) -> Response {
    let email = match params.get("email") {
        Some(email) if !email.is_empty() => email,
        _ => return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                error_codes::INVALID_INPUT.to_string(),
                "Email parameter is required".to_string()
            ))
        ).into_response()
    };

    match services::get_supplier_by_email(&pool, email).await {
        Ok(Some(supplier)) => (
            StatusCode::OK,
            Json(SuccessResponse::new(Some(supplier), "Supplier retrieved successfully".to_string()))
        ).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                error_codes::NOT_FOUND.to_string(),
                "Supplier not found".to_string()
            ))
        ).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                error_codes::DATABASE_ERROR.to_string(),
                "Failed to retrieve supplier".to_string()
            ))
        ).into_response()
    }
}

#[utoipa::path(
    put,
    path = "/suppliers/{id}",
    tag = "suppliers",
    params(
        ("id" = i32, Path, description = "Supplier ID")
    ),
    request_body = UpdateSupplierRequest,
    responses(
        (status = 200, description = "Supplier updated successfully", body = SuccessResponse<String>),
        (status = 400, description = "Validation error or invalid ID", body = ErrorResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn update_supplier(
    Path(id): Path<i32>,
    State(pool): State<PgPool>,
    Json(payload): Json<UpdateSupplierRequest>,
) -> Response {
    if id <= 0 {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                error_codes::INVALID_INPUT.to_string(),
                "Invalid supplier ID".to_string()
            ))
        ).into_response();
    }

    if let Err(validation_errors) = payload.validate() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                error_codes::VALIDATION_ERROR.to_string(),
                format!("Validation failed: {}", validation_errors)
            ))
        ).into_response();
    }

    match services::update_supplier(&pool, id, payload).await {
        Ok(_) => (
            StatusCode::OK,
            Json(SuccessResponse::new("Supplier updated".to_string(), "Supplier updated successfully".to_string()))
        ).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                error_codes::DATABASE_ERROR.to_string(),
                "Failed to update supplier".to_string()
            ))
        ).into_response()
    }
}

#[utoipa::path(
    delete,
    path = "/suppliers/{id}",
    tag = "suppliers",
    params(
        ("id" = i32, Path, description = "Supplier ID")
    ),
    responses(
        (status = 200, description = "Supplier deleted successfully", body = SuccessResponse<String>),
        (status = 400, description = "Invalid supplier ID", body = ErrorResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn delete_supplier(
    Path(id): Path<i32>,
    State(pool): State<PgPool>,
) -> Response {
    if id <= 0 {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                error_codes::INVALID_INPUT.to_string(),
                "Invalid supplier ID".to_string()
            ))
        ).into_response();
    }

    match services::delete_supplier(&pool, id).await {
        Ok(_) => (
            StatusCode::OK,
            Json(SuccessResponse::new("Supplier deleted".to_string(), "Supplier deleted successfully".to_string()))
        ).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                error_codes::DATABASE_ERROR.to_string(),
                "Failed to delete supplier".to_string()
            ))
        ).into_response()
    }
}
