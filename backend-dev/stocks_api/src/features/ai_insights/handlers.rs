use axum::{
    extract::{Query, State},
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use sqlx::PgPool;

use crate::common::responses::{SuccessResponse, ErrorResponse};
use crate::common::error_codes;

use super::dto::*;
use super::services::AiInsightsService;

/// GET /ai/insights - Obtenir toutes les données agrégées pour l'analyse IA
#[utoipa::path(
    get,
    path = "/ai/insights",
    tag = "ai-insights",
    params(
        AiInsightsParams
    ),
    responses(
        (status = 200, description = "AI insights retrieved successfully", body = AiInsightsData),
        (status = 500, description = "Database error", body = ErrorResponse)
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_ai_insights(
    Query(params): Query<AiInsightsParams>,
    State(pool): State<PgPool>,
) -> Response {
    match AiInsightsService::get_ai_insights(&pool, &params).await {
        Ok(data) => (
            StatusCode::OK,
            Json(SuccessResponse::new(
                data,
                "Données pour analyse IA récupérées avec succès".to_string()
            ))
        ).into_response(),
        Err(e) => {
            eprintln!("Database error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    error_codes::DATABASE_ERROR.to_string(),
                    format!("Erreur lors de la récupération des données: {}", e)
                ))
            ).into_response()
        }
    }
}
