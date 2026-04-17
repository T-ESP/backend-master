use axum::{Json, extract::{Path, Query, Extension}, response::{IntoResponse, Response}};
use axum::http::StatusCode;
use sqlx::PgPool;
use crate::common::{responses::{SuccessResponse, ErrorResponse}, error_codes};
use super::{dto::{PaginationParams, AnomalyParams}, services};

// ==================== Demand Forecasts ====================

#[utoipa::path(
    get,
    path = "/ai/forecasts",
    tag = "ai_predictions",
    params(PaginationParams),
    responses(
        (status = 200, description = "Forecasts retrieved successfully", body = inline(SuccessResponse<Vec<super::dto::ForecastResponse>>)),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn get_forecasts(
    Query(params): Query<PaginationParams>,
    Extension(pool): Extension<PgPool>,
) -> Response {
    let limit = params.limit.unwrap_or(50);
    let offset = params.offset.unwrap_or(0);

    match services::get_latest_forecasts(&pool, limit, offset).await {
        Ok(data) => (
            StatusCode::OK,
            Json(SuccessResponse::new(data, "Demand forecasts retrieved successfully".to_string()))
        ).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(error_codes::DATABASE_ERROR.to_string(), "Failed to retrieve forecasts".to_string()))
        ).into_response()
    }
}

#[utoipa::path(
    get,
    path = "/ai/forecasts/{product_id}",
    tag = "ai_predictions",
    params(("product_id" = i32, Path, description = "Product ID")),
    responses(
        (status = 200, description = "Forecast retrieved successfully", body = inline(SuccessResponse<super::dto::ForecastResponse>)),
        (status = 404, description = "No forecast found", body = ErrorResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn get_forecast_by_product(
    Path((_commerce_id, product_id)): Path<(String, i32)>,
    Extension(pool): Extension<PgPool>,
) -> Response {
    match services::get_forecast_by_product(&pool, product_id).await {
        Ok(Some(data)) => (
            StatusCode::OK,
            Json(SuccessResponse::new(data, "Forecast retrieved successfully".to_string()))
        ).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(error_codes::NOT_FOUND.to_string(), format!("No forecast found for product {}", product_id)))
        ).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(error_codes::DATABASE_ERROR.to_string(), "Failed to retrieve forecast".to_string()))
        ).into_response()
    }
}

// ==================== Classifications ====================

#[utoipa::path(
    get,
    path = "/ai/classifications",
    tag = "ai_predictions",
    params(PaginationParams),
    responses(
        (status = 200, description = "Classifications retrieved successfully", body = inline(SuccessResponse<Vec<super::dto::ClassificationResponse>>)),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn get_classifications(
    Query(params): Query<PaginationParams>,
    Extension(pool): Extension<PgPool>,
) -> Response {
    let limit = params.limit.unwrap_or(50);
    let offset = params.offset.unwrap_or(0);

    match services::get_latest_classifications(&pool, limit, offset).await {
        Ok(data) => (
            StatusCode::OK,
            Json(SuccessResponse::new(data, "Classifications retrieved successfully".to_string()))
        ).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(error_codes::DATABASE_ERROR.to_string(), "Failed to retrieve classifications".to_string()))
        ).into_response()
    }
}

#[utoipa::path(
    get,
    path = "/ai/classifications/{product_id}",
    tag = "ai_predictions",
    params(("product_id" = i32, Path, description = "Product ID")),
    responses(
        (status = 200, description = "Classification retrieved successfully", body = inline(SuccessResponse<super::dto::ClassificationResponse>)),
        (status = 404, description = "No classification found", body = ErrorResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn get_classification_by_product(
    Path((_commerce_id, product_id)): Path<(String, i32)>,
    Extension(pool): Extension<PgPool>,
) -> Response {
    match services::get_classification_by_product(&pool, product_id).await {
        Ok(Some(data)) => (
            StatusCode::OK,
            Json(SuccessResponse::new(data, "Classification retrieved successfully".to_string()))
        ).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(error_codes::NOT_FOUND.to_string(), format!("No classification found for product {}", product_id)))
        ).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(error_codes::DATABASE_ERROR.to_string(), "Failed to retrieve classification".to_string()))
        ).into_response()
    }
}

// ==================== Clusters ====================

#[utoipa::path(
    get,
    path = "/ai/clusters",
    tag = "ai_predictions",
    params(PaginationParams),
    responses(
        (status = 200, description = "Clusters retrieved successfully", body = inline(SuccessResponse<Vec<super::dto::ClusterResponse>>)),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn get_clusters(
    Query(params): Query<PaginationParams>,
    Extension(pool): Extension<PgPool>,
) -> Response {
    let limit = params.limit.unwrap_or(50);
    let offset = params.offset.unwrap_or(0);

    match services::get_latest_clusters(&pool, limit, offset).await {
        Ok(data) => (
            StatusCode::OK,
            Json(SuccessResponse::new(data, "Product clusters retrieved successfully".to_string()))
        ).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(error_codes::DATABASE_ERROR.to_string(), "Failed to retrieve clusters".to_string()))
        ).into_response()
    }
}

#[utoipa::path(
    get,
    path = "/ai/clusters/{product_id}",
    tag = "ai_predictions",
    params(("product_id" = i32, Path, description = "Product ID")),
    responses(
        (status = 200, description = "Cluster retrieved successfully", body = inline(SuccessResponse<super::dto::ClusterResponse>)),
        (status = 404, description = "No cluster found", body = ErrorResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn get_cluster_by_product(
    Path((_commerce_id, product_id)): Path<(String, i32)>,
    Extension(pool): Extension<PgPool>,
) -> Response {
    match services::get_cluster_by_product(&pool, product_id).await {
        Ok(Some(data)) => (
            StatusCode::OK,
            Json(SuccessResponse::new(data, "Cluster retrieved successfully".to_string()))
        ).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(error_codes::NOT_FOUND.to_string(), format!("No cluster found for product {}", product_id)))
        ).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(error_codes::DATABASE_ERROR.to_string(), "Failed to retrieve cluster".to_string()))
        ).into_response()
    }
}

// ==================== Supplier Scores ====================

#[utoipa::path(
    get,
    path = "/ai/supplier-scores",
    tag = "ai_predictions",
    params(PaginationParams),
    responses(
        (status = 200, description = "Supplier scores retrieved successfully", body = inline(SuccessResponse<Vec<super::dto::SupplierScoreResponse>>)),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn get_supplier_scores(
    Query(params): Query<PaginationParams>,
    Extension(pool): Extension<PgPool>,
) -> Response {
    let limit = params.limit.unwrap_or(50);
    let offset = params.offset.unwrap_or(0);

    match services::get_latest_supplier_scores(&pool, limit, offset).await {
        Ok(data) => (
            StatusCode::OK,
            Json(SuccessResponse::new(data, "Supplier scores retrieved successfully".to_string()))
        ).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(error_codes::DATABASE_ERROR.to_string(), "Failed to retrieve supplier scores".to_string()))
        ).into_response()
    }
}

#[utoipa::path(
    get,
    path = "/ai/supplier-scores/{supplier_id}",
    tag = "ai_predictions",
    params(("supplier_id" = i32, Path, description = "Supplier ID")),
    responses(
        (status = 200, description = "Supplier score retrieved successfully", body = inline(SuccessResponse<super::dto::SupplierScoreResponse>)),
        (status = 404, description = "No score found", body = ErrorResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn get_supplier_score(
    Path((_commerce_id, supplier_id)): Path<(String, i32)>,
    Extension(pool): Extension<PgPool>,
) -> Response {
    match services::get_supplier_score(&pool, supplier_id).await {
        Ok(Some(data)) => (
            StatusCode::OK,
            Json(SuccessResponse::new(data, "Supplier score retrieved successfully".to_string()))
        ).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(error_codes::NOT_FOUND.to_string(), format!("No score found for supplier {}", supplier_id)))
        ).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(error_codes::DATABASE_ERROR.to_string(), "Failed to retrieve supplier score".to_string()))
        ).into_response()
    }
}

// ==================== Price Suggestions ====================

#[utoipa::path(
    get,
    path = "/ai/price-suggestions",
    tag = "ai_predictions",
    params(PaginationParams),
    responses(
        (status = 200, description = "Price suggestions retrieved successfully", body = inline(SuccessResponse<Vec<super::dto::PriceSuggestionResponse>>)),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn get_price_suggestions(
    Query(params): Query<PaginationParams>,
    Extension(pool): Extension<PgPool>,
) -> Response {
    let limit = params.limit.unwrap_or(50);

    match services::get_price_suggestions(&pool, limit).await {
        Ok(data) => (
            StatusCode::OK,
            Json(SuccessResponse::new(data, "Price suggestions retrieved successfully".to_string()))
        ).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(error_codes::DATABASE_ERROR.to_string(), "Failed to retrieve price suggestions".to_string()))
        ).into_response()
    }
}

// ==================== Price Anomalies ====================

#[utoipa::path(
    get,
    path = "/ai/price-anomalies",
    tag = "ai_predictions",
    params(AnomalyParams),
    responses(
        (status = 200, description = "Price anomalies retrieved successfully", body = inline(SuccessResponse<Vec<super::dto::PriceAnomalyResponse>>)),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn get_price_anomalies(
    Query(params): Query<AnomalyParams>,
    Extension(pool): Extension<PgPool>,
) -> Response {
    let limit = params.limit.unwrap_or(50);
    let only_anomalies = params.only_anomalies.unwrap_or(false);

    match services::get_price_anomalies(&pool, limit, only_anomalies).await {
        Ok(data) => (
            StatusCode::OK,
            Json(SuccessResponse::new(data, "Price anomalies retrieved successfully".to_string()))
        ).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(error_codes::DATABASE_ERROR.to_string(), "Failed to retrieve price anomalies".to_string()))
        ).into_response()
    }
}

// ==================== Sales Anomalies ====================

#[utoipa::path(
    get,
    path = "/ai/sales-anomalies",
    tag = "ai_predictions",
    params(AnomalyParams),
    responses(
        (status = 200, description = "Sales anomalies retrieved successfully", body = inline(SuccessResponse<Vec<super::dto::SalesAnomalyResponse>>)),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn get_sales_anomalies(
    Query(params): Query<AnomalyParams>,
    Extension(pool): Extension<PgPool>,
) -> Response {
    let limit = params.limit.unwrap_or(50);
    let only_anomalies = params.only_anomalies.unwrap_or(false);

    match services::get_sales_anomalies(&pool, limit, only_anomalies).await {
        Ok(data) => (
            StatusCode::OK,
            Json(SuccessResponse::new(data, "Sales anomalies retrieved successfully".to_string()))
        ).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(error_codes::DATABASE_ERROR.to_string(), "Failed to retrieve sales anomalies".to_string()))
        ).into_response()
    }
}

// ==================== Urgent Restocks ====================

#[utoipa::path(
    get,
    path = "/ai/urgent-restocks",
    tag = "ai_predictions",
    params(PaginationParams),
    responses(
        (status = 200, description = "Urgent restocks retrieved successfully", body = inline(SuccessResponse<Vec<super::dto::UrgentRestockResponse>>)),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn get_urgent_restocks(
    Query(params): Query<PaginationParams>,
    Extension(pool): Extension<PgPool>,
) -> Response {
    let limit = params.limit.unwrap_or(50);

    match services::get_urgent_restocks(&pool, limit).await {
        Ok(data) => (
            StatusCode::OK,
            Json(SuccessResponse::new(data, "Urgent restocks retrieved successfully".to_string()))
        ).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(error_codes::DATABASE_ERROR.to_string(), "Failed to retrieve urgent restocks".to_string()))
        ).into_response()
    }
}
