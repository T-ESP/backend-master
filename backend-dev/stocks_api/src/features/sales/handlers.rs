use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
    response::{IntoResponse, Response},
};
use chrono::{NaiveDate, Duration};
use sqlx::PgPool;

use crate::common::{responses::{SuccessResponse, ErrorResponse}, error_codes};
use super::{
    dto::{
        AverageBasketByClientTypeResponse, AverageBasketResponse, ComparisonResponse,
        EvolutionResponse, EvolutionDataPoint, EvolutionQuery, PeriodQuery, TotalRevenueResponse,
    },
    services,
};

fn parse_period(q: &PeriodQuery) -> Result<(NaiveDate, NaiveDate), StatusCode> {
    let start = NaiveDate::parse_from_str(&q.start_date, "%Y-%m-%d")
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let end = NaiveDate::parse_from_str(&q.end_date, "%Y-%m-%d")
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    if end < start {
        return Err(StatusCode::BAD_REQUEST);
    }
    Ok((start, end))
}

#[utoipa::path(
    get,
    path = "/sales/total-revenue",
    tag = "sales",
    params(PeriodQuery),
    responses(
        (status = 200, description = "Total revenue calculated successfully", body = SuccessResponse<TotalRevenueResponse>),
        (status = 400, description = "Invalid date format or range", body = ErrorResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn get_total_revenue(
    State(pool): State<PgPool>,
    Query(q): Query<PeriodQuery>,
) -> Response {
    let (start, end) = match parse_period(&q) {
        Ok(v) => v,
        Err(code) => {
            return (
                code,
                Json(ErrorResponse::new(
                    error_codes::INVALID_INPUT.to_string(),
                    "Invalid date format or range".to_string(),
                )),
            ).into_response()
        }
    };

    match services::get_total_revenue(&pool, start, end).await {
        Ok(total) => (
            StatusCode::OK,
            Json(SuccessResponse::new(
                TotalRevenueResponse { total_revenue: total },
                "Total revenue calculated".into(),
            )),
        ).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                error_codes::DATABASE_ERROR.to_string(),
                "Failed to calculate total revenue".to_string(),
            )),
        ).into_response(),
    }
}

#[utoipa::path(
    get,
    path = "/sales/evolution",
    tag = "sales",
    params(EvolutionQuery),
    responses(
        (status = 200, description = "Revenue evolution calculated successfully", body = SuccessResponse<EvolutionResponse>),
        (status = 400, description = "Invalid date format or range", body = ErrorResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn get_evolution(
    State(pool): State<PgPool>,
    Query(q): Query<EvolutionQuery>,
) -> Response {
    let start = match NaiveDate::parse_from_str(&q.start_date, "%Y-%m-%d") {
        Ok(d) => d,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    error_codes::INVALID_INPUT.to_string(),
                    "Invalid start_date format".to_string(),
                )),
            ).into_response()
        }
    };

    let end = match NaiveDate::parse_from_str(&q.end_date, "%Y-%m-%d") {
        Ok(d) => d,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    error_codes::INVALID_INPUT.to_string(),
                    "Invalid end_date format".to_string(),
                )),
            ).into_response()
        }
    };

    if end < start {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                error_codes::INVALID_INPUT.to_string(),
                "end_date must be after start_date".to_string(),
            )),
        ).into_response();
    }

    // Valide et normalise le grain
    let grain = q.grain.as_deref().unwrap_or("day");
    let normalized_grain = match grain {
        "day" | "d" => "day",
        "week" | "w" => "week",
        "month" | "m" => "month",
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    error_codes::INVALID_INPUT.to_string(),
                    "Invalid grain. Must be 'day', 'week', or 'month'".to_string(),
                )),
            ).into_response();
        }
    };

    match services::get_revenue_evolution_by_grain(&pool, start, end, normalized_grain).await {
        Ok(points) => {
            let data: Vec<EvolutionDataPoint> = points
                .into_iter()
                .map(|p| EvolutionDataPoint {
                    date: p.date,
                    revenue: p.revenue,
                })
                .collect();

            (
                StatusCode::OK,
                Json(SuccessResponse::new(
                    EvolutionResponse {
                        grain: normalized_grain.to_string(),
                        data,
                    },
                    "Evolution calculated".into(),
                )),
            ).into_response()
        }
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                error_codes::DATABASE_ERROR.to_string(),
                "Failed to calculate evolution".to_string(),
            )),
        ).into_response(),
    }
}

#[utoipa::path(
    get,
    path = "/sales/comparison",
    tag = "sales",
    params(PeriodQuery),
    responses(
        (status = 200, description = "Forecast vs actual comparison calculated", body = SuccessResponse<ComparisonResponse>),
        (status = 400, description = "Invalid date format or range", body = ErrorResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn get_comparison(
    State(pool): State<PgPool>,
    Query(q): Query<PeriodQuery>,
) -> Response {
    let (start, end) = match parse_period(&q) {
        Ok(v) => v,
        Err(code) => {
            return (
                code,
                Json(ErrorResponse::new(
                    error_codes::INVALID_INPUT.to_string(),
                    "Invalid date format or range".to_string(),
                )),
            ).into_response()
        }
    };

    match services::get_forecast_vs_actual(&pool, start, end).await {
        Ok((forecast, actual)) => (
            StatusCode::OK,
            Json(SuccessResponse::new(
                ComparisonResponse { forecast, actual },
                "Forecast vs actual".into(),
            )),
        ).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                error_codes::DATABASE_ERROR.to_string(),
                "Failed to calculate comparison".to_string(),
            )),
        ).into_response(),
    }
}

#[utoipa::path(
    get,
    path = "/sales/average-basket",
    tag = "sales",
    params(PeriodQuery),
    responses(
        (status = 200, description = "Average basket calculated successfully", body = SuccessResponse<AverageBasketResponse>),
        (status = 400, description = "Invalid date format or range", body = ErrorResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn get_average_basket(
    State(pool): State<PgPool>,
    Query(q): Query<PeriodQuery>,
) -> Response {
    let (start, end) = match parse_period(&q) {
        Ok(v) => v,
        Err(code) => {
            return (
                code,
                Json(ErrorResponse::new(
                    error_codes::INVALID_INPUT.to_string(),
                    "Invalid date format or range".to_string(),
                )),
            ).into_response()
        }
    };

    match services::get_average_basket(&pool, start, end).await {
        Ok((avg, evo)) => (
            StatusCode::OK,
            Json(SuccessResponse::new(
                AverageBasketResponse {
                    average_basket: avg,
                    evolution_percentage: Some(evo),
                },
                "Average basket calculated".into(),
            )),
        ).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                error_codes::DATABASE_ERROR.to_string(),
                "Failed to calculate average basket".to_string(),
            )),
        ).into_response(),
    }
}

#[utoipa::path(
    get,
    path = "/sales/average-basket-by-client-type",
    tag = "sales",
    params(PeriodQuery),
    responses(
        (status = 200, description = "Average basket by client type calculated", body = SuccessResponse<AverageBasketByClientTypeResponse>),
        (status = 400, description = "Invalid date format or range", body = ErrorResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn get_average_basket_by_client_type(
    State(pool): State<PgPool>,
    Query(q): Query<PeriodQuery>,
) -> Response {
    let (start, end) = match parse_period(&q) {
        Ok(v) => v,
        Err(code) => {
            return (
                code,
                Json(ErrorResponse::new(
                    error_codes::INVALID_INPUT.to_string(),
                    "Invalid date format or range".to_string(),
                )),
            ).into_response()
        }
    };

    match services::get_average_basket_by_client_type(&pool, start, end).await {
        Ok((new, loyal)) => (
            StatusCode::OK,
            Json(SuccessResponse::new(
                AverageBasketByClientTypeResponse {
                    new_clients: new,
                    loyal_clients: loyal,
                },
                "Average basket by client type".into(),
            )),
        ).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                error_codes::DATABASE_ERROR.to_string(),
                "Failed to calculate average basket by client type".to_string(),
            )),
        ).into_response(),
    }
}
