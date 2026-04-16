use axum::{Json, extract::{Query, State}, response::{IntoResponse, Response}};
use axum::http::StatusCode;
use sqlx::PgPool;
use crate::common::{responses::{SuccessResponse, ErrorResponse}, error_codes};
use super::{dto::{StockParams, OverstockParams, StockResponse, StockAlert, StockSummary}, services};

#[utoipa::path(
    get,
    path = "/stocks/out-of-stock",
    tag = "stocks",
    params(StockParams),
    responses(
        (status = 200, description = "Out of stock products retrieved successfully", body = inline(SuccessResponse<Vec<StockResponse>>)),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn get_out_of_stock(
    Query(params): Query<StockParams>,
    State(pool): State<PgPool>,
) -> Response {
    let limit = params.limit.unwrap_or(50);

    match services::get_out_of_stock_products(&pool, limit).await {
        Ok(products) => (
            StatusCode::OK,
            Json(SuccessResponse::new(products, "Out of stock products retrieved successfully".to_string()))
        ).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                error_codes::DATABASE_ERROR.to_string(),
                "Failed to retrieve out of stock products".to_string()
            ))
        ).into_response()
    }
}

#[utoipa::path(
    get,
    path = "/stocks/low-stock",
    tag = "stocks",
    params(StockParams),
    responses(
        (status = 200, description = "Low stock products retrieved successfully", body = inline(SuccessResponse<Vec<StockResponse>>)),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn get_low_stock(
    Query(params): Query<StockParams>,
    State(pool): State<PgPool>,
) -> Response {
    let limit = params.limit.unwrap_or(50);
    let threshold = params.threshold.unwrap_or(10);

    match services::get_low_stock_products(&pool, limit, threshold).await {
        Ok(products) => (
            StatusCode::OK,
            Json(SuccessResponse::new(products, "Low stock products retrieved successfully".to_string()))
        ).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                error_codes::DATABASE_ERROR.to_string(),
                "Failed to retrieve low stock products".to_string()
            ))
        ).into_response()
    }
}

#[utoipa::path(
    get,
    path = "/stocks/soon-out-of-stock",
    tag = "stocks",
    params(StockParams),
    responses(
        (status = 200, description = "Soon out of stock products retrieved successfully", body = inline(SuccessResponse<Vec<StockResponse>>)),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn get_soon_out_of_stock(
    Query(params): Query<StockParams>,
    State(pool): State<PgPool>,
) -> Response {
    let limit = params.limit.unwrap_or(20);

    match services::get_soon_out_of_stock_products(&pool, limit).await {
        Ok(products) => (
            StatusCode::OK,
            Json(SuccessResponse::new(products, "Soon out of stock products retrieved successfully".to_string()))
        ).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                error_codes::DATABASE_ERROR.to_string(),
                "Failed to retrieve soon out of stock products".to_string()
            ))
        ).into_response()
    }
}

#[utoipa::path(
    get,
    path = "/stocks/overstock",
    tag = "stocks",
    params(OverstockParams),
    responses(
        (status = 200, description = "Overstock products retrieved successfully", body = inline(SuccessResponse<Vec<StockResponse>>)),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn get_overstock(
    Query(params): Query<OverstockParams>,
    State(pool): State<PgPool>,
) -> Response {
    let limit = params.limit.unwrap_or(30);
    let multiplier = params.multiplier.unwrap_or(3.0);

    match services::get_overstock_products(&pool, limit, multiplier).await {
        Ok(products) => (
            StatusCode::OK,
            Json(SuccessResponse::new(products, "Overstock products retrieved successfully".to_string()))
        ).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                error_codes::DATABASE_ERROR.to_string(),
                "Failed to retrieve overstock products".to_string()
            ))
        ).into_response()
    }
}

#[utoipa::path(
    get,
    path = "/stocks/alerts",
    tag = "stocks",
    params(StockParams),
    responses(
        (status = 200, description = "Stock alerts retrieved successfully", body = inline(SuccessResponse<Vec<StockAlert>>)),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn get_stock_alerts(
    Query(params): Query<StockParams>,
    State(pool): State<PgPool>,
) -> Response {
    let limit = params.limit.unwrap_or(25);

    match services::get_stock_alerts_data(&pool, limit).await {
        Ok(alerts) => (
            StatusCode::OK,
            Json(SuccessResponse::new(alerts, "Stock alerts retrieved successfully".to_string()))
        ).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                error_codes::DATABASE_ERROR.to_string(),
                "Failed to retrieve stock alerts".to_string()
            ))
        ).into_response()
    }
}

#[utoipa::path(
    get,
    path = "/stocks/summary",
    tag = "stocks",
    responses(
        (status = 200, description = "Stock summary retrieved successfully", body = inline(SuccessResponse<StockSummary>)),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn get_stock_summary(
    State(pool): State<PgPool>
) -> Response {
    match services::get_stock_summary_data(&pool).await {
        Ok(summary) => (
            StatusCode::OK,
            Json(SuccessResponse::new(summary, "Stock summary retrieved successfully".to_string()))
        ).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                error_codes::DATABASE_ERROR.to_string(),
                "Failed to retrieve stock summary".to_string()
            ))
        ).into_response()
    }
}