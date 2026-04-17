use axum::{Json, extract::{Path, Query, Extension}, response::{IntoResponse, Response}};
use axum::http::StatusCode;
use sqlx::PgPool;
use crate::common::{responses::{SuccessResponse, ErrorResponse}, error_codes};
use super::{dto::{AlertFilters, CleanupParams, UpdateStatusRequest, BulkUpdateStatusRequest}, services};

#[utoipa::path(
    get,
    path = "/alerts",
    tag = "alerts",
    params(AlertFilters),
    responses(
        (status = 200, description = "Alerts retrieved successfully", body = inline(SuccessResponse<Vec<super::dto::AlertResponse>>)),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn get_alerts(
    Query(filters): Query<AlertFilters>,
    Extension(pool): Extension<PgPool>,
) -> Response {
    match services::get_alerts(&pool, &filters).await {
        Ok(alerts) => (
            StatusCode::OK,
            Json(SuccessResponse::new(alerts, "Alerts retrieved successfully".to_string()))
        ).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                error_codes::DATABASE_ERROR.to_string(),
                "Failed to retrieve alerts".to_string()
            ))
        ).into_response()
    }
}

#[utoipa::path(
    get,
    path = "/alerts/summary",
    tag = "alerts",
    responses(
        (status = 200, description = "Alert summary retrieved successfully", body = inline(SuccessResponse<super::dto::AlertSummary>)),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn get_alert_summary(
    Extension(pool): Extension<PgPool>,
) -> Response {
    match services::get_alert_summary(&pool).await {
        Ok(summary) => (
            StatusCode::OK,
            Json(SuccessResponse::new(summary, "Alert summary retrieved successfully".to_string()))
        ).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                error_codes::DATABASE_ERROR.to_string(),
                "Failed to retrieve alert summary".to_string()
            ))
        ).into_response()
    }
}

#[utoipa::path(
    get,
    path = "/alerts/{id}",
    tag = "alerts",
    params(("id" = i32, Path, description = "Alert ID")),
    responses(
        (status = 200, description = "Alert retrieved successfully", body = inline(SuccessResponse<super::dto::AlertResponse>)),
        (status = 404, description = "Alert not found", body = ErrorResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn get_alert_by_id(
    Path((_commerce_id, id)): Path<(String, i32)>,
    Extension(pool): Extension<PgPool>,
) -> Response {
    match services::get_alert_by_id(&pool, id).await {
        Ok(Some(alert)) => (
            StatusCode::OK,
            Json(SuccessResponse::new(alert, "Alert retrieved successfully".to_string()))
        ).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                error_codes::NOT_FOUND.to_string(),
                format!("Alert with id {} not found", id)
            ))
        ).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                error_codes::DATABASE_ERROR.to_string(),
                "Failed to retrieve alert".to_string()
            ))
        ).into_response()
    }
}

#[utoipa::path(
    put,
    path = "/alerts/{id}/status",
    tag = "alerts",
    params(("id" = i32, Path, description = "Alert ID")),
    request_body = UpdateStatusRequest,
    responses(
        (status = 200, description = "Alert status updated successfully"),
        (status = 400, description = "Invalid status", body = ErrorResponse),
        (status = 404, description = "Alert not found", body = ErrorResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn update_alert_status(
    Path((_commerce_id, id)): Path<(String, i32)>,
    Extension(pool): Extension<PgPool>,
    Json(body): Json<UpdateStatusRequest>,
) -> Response {
    if !services::is_valid_status(&body.status) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                error_codes::VALIDATION_ERROR.to_string(),
                format!("Invalid status '{}'. Valid values: new, acknowledged, in_progress, resolved, dismissed", body.status)
            ))
        ).into_response();
    }

    match services::update_alert_status(&pool, id, &body.status).await {
        Ok(true) => (
            StatusCode::OK,
            Json(SuccessResponse::new(
                serde_json::json!({"id": id, "status": body.status}),
                "Alert status updated successfully".to_string()
            ))
        ).into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                error_codes::NOT_FOUND.to_string(),
                format!("Alert with id {} not found", id)
            ))
        ).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                error_codes::DATABASE_ERROR.to_string(),
                "Failed to update alert status".to_string()
            ))
        ).into_response()
    }
}

#[utoipa::path(
    put,
    path = "/alerts/bulk-status",
    tag = "alerts",
    request_body = BulkUpdateStatusRequest,
    responses(
        (status = 200, description = "Alert statuses updated successfully"),
        (status = 400, description = "Invalid status", body = ErrorResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn bulk_update_status(
    Extension(pool): Extension<PgPool>,
    Json(body): Json<BulkUpdateStatusRequest>,
) -> Response {
    if !services::is_valid_status(&body.status) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                error_codes::VALIDATION_ERROR.to_string(),
                format!("Invalid status '{}'. Valid values: new, acknowledged, in_progress, resolved, dismissed", body.status)
            ))
        ).into_response();
    }

    match services::bulk_update_status(&pool, &body.ids, &body.status).await {
        Ok(affected) => (
            StatusCode::OK,
            Json(SuccessResponse::new(
                serde_json::json!({"updated_count": affected}),
                format!("{} alerts updated to '{}'", affected, body.status)
            ))
        ).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                error_codes::DATABASE_ERROR.to_string(),
                "Failed to bulk update alert statuses".to_string()
            ))
        ).into_response()
    }
}

#[utoipa::path(
    delete,
    path = "/alerts/cleanup",
    tag = "alerts",
    params(CleanupParams),
    responses(
        (status = 200, description = "Old alerts cleaned up successfully"),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn cleanup_alerts(
    Query(params): Query<CleanupParams>,
    Extension(pool): Extension<PgPool>,
) -> Response {
    let older_than_days = params.older_than_days.unwrap_or(30);

    match services::cleanup_old_alerts(&pool, older_than_days).await {
        Ok(deleted) => (
            StatusCode::OK,
            Json(SuccessResponse::new(
                serde_json::json!({"deleted_count": deleted}),
                format!("{} old resolved/dismissed alerts cleaned up", deleted)
            ))
        ).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                error_codes::DATABASE_ERROR.to_string(),
                "Failed to cleanup old alerts".to_string()
            ))
        ).into_response()
    }
}
