use axum::{
    extract::{Path, Query, State},
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use sqlx::PgPool;

use crate::common::responses::{SuccessResponse, ErrorResponse};
use crate::common::error_codes;

use super::dto::{CreateRestockRequest, UpdateRestockRequest};
use super::dto::RestockSearchParams;
use super::services::RestockService;

/// GET /api/restocks - Get all restocks with pagination and filters
#[utoipa::path(
    get,
    path = "/restocks",
    tag = "restocks",
    params(RestockSearchParams),
    responses(
        (status = 200, description = "Restocks retrieved successfully", body = Vec<RestockResponse>),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn get_restocks(
    Query(params): Query<RestockSearchParams>,
    State(pool): State<PgPool>,
) -> Response {
    match RestockService::get_restocks(&pool, &params).await {
        Ok(restocks) => (
            StatusCode::OK,
            Json(SuccessResponse::new(restocks, "Restocks retrieved successfully".to_string()))
        ).into_response(),
        Err(e) => {
            eprintln!("Database error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    error_codes::DATABASE_ERROR.to_string(),
                    "Erreur de base de données".to_string()
                ))
            ).into_response()
        }
    }
}

/// GET /api/restocks/:id - Get restock by ID
#[utoipa::path(
    get,
    path = "/restocks/{id}",
    tag = "restocks",
    params(
        ("id" = i32, Path, description = "Restock ID")
    ),
    responses(
        (status = 200, description = "Restock retrieved successfully", body = RestockResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn get_restock_by_id(
    Path(id): Path<i32>,
    State(pool): State<PgPool>,
) -> Response {
    match RestockService::get_restock_by_id(&pool, id).await {
        Ok(restock) => (
            StatusCode::OK,
            Json(SuccessResponse::new(restock, "Restock retrieved successfully".to_string()))
        ).into_response(),
        Err(e) => {
            eprintln!("Database error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    error_codes::DATABASE_ERROR.to_string(),
                    "Erreur de base de données".to_string()
                ))
            ).into_response()
        }
    }
}

/// GET /api/restocks/with-supplier - Get restocks with supplier information
#[utoipa::path(
    get,
    path = "/restocks/with-supplier",
    tag = "restocks",
    params(RestockSearchParams),
    responses(
        (status = 200, description = "Restocks with supplier info retrieved successfully", body = Vec<RestockWithSupplierResponse>),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn get_restocks_with_supplier(
    Query(params): Query<RestockSearchParams>,
    State(pool): State<PgPool>,
) -> Response {
    match RestockService::get_restocks_with_supplier(&pool, &params).await {
        Ok(restocks) => (
            StatusCode::OK,
            Json(SuccessResponse::new(restocks, "Restocks with supplier info retrieved successfully".to_string()))
        ).into_response(),
        Err(e) => {
            eprintln!("Database error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    error_codes::DATABASE_ERROR.to_string(),
                    "Erreur de base de données".to_string()
                ))
            ).into_response()
        }
    }
}

/// POST /api/restocks - Create a new restock
#[utoipa::path(
    post,
    path = "/restocks",
    tag = "restocks",
    request_body = CreateRestockRequest,
    responses(
        (status = 201, description = "Restock created successfully", body = RestockResponse),
        (status = 400, description = "Invalid input", body = ErrorResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn create_restock(
    State(pool): State<PgPool>,
    Json(req): Json<CreateRestockRequest>,
) -> Response {
    // Validate that there is at least one line
    if req.lines.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_REQUEST".to_string(),
                "Un restock doit contenir au moins une ligne de produit".to_string()
            ))
        ).into_response();
    }

    // Validate each line
    for line in &req.lines {
        if line.quantity <= 0 {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "INVALID_QUANTITY".to_string(),
                    "La quantité doit être strictement positive".to_string()
                ))
            ).into_response();
        }
        if line.unit_price < rust_decimal::Decimal::ZERO {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "INVALID_PRICE".to_string(),
                    "Le prix unitaire doit être positif ou zéro".to_string()
                ))
            ).into_response();
        }
    }

    match RestockService::create_restock(&pool, &req).await {
        Ok(restock) => (
            StatusCode::CREATED,
            Json(SuccessResponse::new(restock, "Restock created successfully".to_string()))
        ).into_response(),
        Err(e) => {
            eprintln!("Database error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    error_codes::DATABASE_ERROR.to_string(),
                    "Erreur de base de données".to_string()
                ))
            ).into_response()
        }
    }
}

/// PUT /api/restocks/:id - Update a restock
#[utoipa::path(
    put,
    path = "/restocks/{id}",
    tag = "restocks",
    params(
        ("id" = i32, Path, description = "Restock ID")
    ),
    request_body = UpdateRestockRequest,
    responses(
        (status = 200, description = "Restock updated successfully", body = RestockResponse),
        (status = 404, description = "Restock not found", body = ErrorResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn update_restock(
    Path(id): Path<i32>,
    State(pool): State<PgPool>,
    Json(req): Json<UpdateRestockRequest>,
) -> Response {
    match RestockService::update_restock(&pool, id, &req).await {
        Ok(restock) => (
            StatusCode::OK,
            Json(SuccessResponse::new(restock, "Restock updated successfully".to_string()))
        ).into_response(),
        Err(sqlx::Error::RowNotFound) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "RESTOCK_NOT_FOUND".to_string(),
                format!("Restock with ID {} not found", id)
            ))
        ).into_response(),
        Err(e) => {
            eprintln!("Database error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    error_codes::DATABASE_ERROR.to_string(),
                    "Erreur de base de données".to_string()
                ))
            ).into_response()
        }
    }
}

/// GET /api/restocks/stats/global - Get restock statistics
#[utoipa::path(
    get,
    path = "/restocks/stats/global",
    tag = "restocks",
    responses(
        (status = 200, description = "Statistics retrieved successfully", body = RestockStatsResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn get_restock_stats(
    State(pool): State<PgPool>,
) -> Response {
    match RestockService::get_restock_stats(&pool, None).await {
        Ok(stats) => (
            StatusCode::OK,
            Json(SuccessResponse::new(stats, "Statistics retrieved successfully".to_string()))
        ).into_response(),
        Err(e) => {
            eprintln!("Database error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    error_codes::DATABASE_ERROR.to_string(),
                    "Erreur de base de données".to_string()
                ))
            ).into_response()
        }
    }
}

/// GET /api/restocks/stats/product/:product_id - Get restock statistics for a product
#[utoipa::path(
    get,
    path = "/restocks/stats/product/{product_id}",
    tag = "restocks",
    params(
        ("product_id" = i32, Path, description = "Product ID")
    ),
    responses(
        (status = 200, description = "Statistics retrieved successfully", body = RestockStatsResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn get_restock_stats_by_product(
    Path(product_id): Path<i32>,
    State(pool): State<PgPool>,
) -> Response {
    match RestockService::get_restock_stats(&pool, Some(product_id)).await {
        Ok(stats) => (
            StatusCode::OK,
            Json(SuccessResponse::new(stats, "Statistics retrieved successfully".to_string()))
        ).into_response(),
        Err(e) => {
            eprintln!("Database error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    error_codes::DATABASE_ERROR.to_string(),
                    "Erreur de base de données".to_string()
                ))
            ).into_response()
        }
    }
}

/// GET /api/restocks/product/:product_id - Get all restocks for a specific product
#[utoipa::path(
    get,
    path = "/restocks/product/{product_id}",
    tag = "restocks",
    params(
        ("product_id" = i32, Path, description = "Product ID")
    ),
    responses(
        (status = 200, description = "Restocks retrieved successfully", body = Vec<RestockResponse>),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn get_restocks_by_product(
    Path(product_id): Path<i32>,
    Query(params): Query<RestockSearchParams>,
    State(pool): State<PgPool>,
) -> Response {
    match RestockService::get_restocks_by_product(&pool, product_id, params.limit, params.offset).await {
        Ok(restocks) => (
            StatusCode::OK,
            Json(SuccessResponse::new(restocks, "Restocks retrieved successfully".to_string()))
        ).into_response(),
        Err(e) => {
            eprintln!("Database error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    error_codes::DATABASE_ERROR.to_string(),
                    "Erreur de base de données".to_string()
                ))
            ).into_response()
        }
    }
}
