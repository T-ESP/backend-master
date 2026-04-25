use axum::{
    extract::{Path, Extension},
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use sqlx::PgPool;
use rust_decimal::Decimal;

use crate::common::responses::{SuccessResponse, ErrorResponse};
use crate::common::error_codes;

use super::dto::{UpdateLoyaltyConfigRequest, AdjustPointsRequest};
use super::services::{LoyaltyService, AdjustPointsError};

/// GET /api/:commerce_id/loyalty/config
pub async fn get_loyalty_config(
    Extension(pool): Extension<PgPool>,
) -> Response {
    match LoyaltyService::get_or_create_config(&pool).await {
        Ok(config) => (
            StatusCode::OK,
            Json(SuccessResponse::new(config, "Configuration de fidélité récupérée".to_string()))
        ).into_response(),
        Err(e) => {
            eprintln!("Database error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    error_codes::DATABASE_ERROR.to_string(),
                    "Erreur lors de la récupération de la configuration".to_string()
                ))
            ).into_response()
        }
    }
}

/// PUT /api/:commerce_id/loyalty/config
pub async fn update_loyalty_config(
    Extension(pool): Extension<PgPool>,
    Json(request): Json<UpdateLoyaltyConfigRequest>,
) -> Response {
    if let Some(v) = request.euros_per_point {
        if v <= Decimal::ZERO {
            return (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(ErrorResponse::new(
                    "INVALID_VALUE".to_string(),
                    "euros_per_point doit être strictement positif".to_string()
                ))
            ).into_response();
        }
    }
    if let Some(v) = request.points_required {
        if v <= 0 {
            return (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(ErrorResponse::new(
                    "INVALID_VALUE".to_string(),
                    "points_required doit être strictement positif".to_string()
                ))
            ).into_response();
        }
    }
    if let Some(v) = request.discount_percent {
        if v <= Decimal::ZERO || v > Decimal::from(100) {
            return (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(ErrorResponse::new(
                    "INVALID_VALUE".to_string(),
                    "discount_percent doit être entre 0 (exclu) et 100".to_string()
                ))
            ).into_response();
        }
    }

    match LoyaltyService::update_config(
        &pool,
        request.euros_per_point,
        request.points_required,
        request.discount_percent,
    ).await {
        Ok(config) => (
            StatusCode::OK,
            Json(SuccessResponse::new(config, "Configuration de fidélité mise à jour".to_string()))
        ).into_response(),
        Err(e) => {
            eprintln!("Database error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    error_codes::DATABASE_ERROR.to_string(),
                    "Erreur lors de la mise à jour de la configuration".to_string()
                ))
            ).into_response()
        }
    }
}

/// GET /api/:commerce_id/loyalty/users/:user_id
pub async fn get_user_loyalty(
    Path((_commerce_id, user_id)): Path<(String, i32)>,
    Extension(pool): Extension<PgPool>,
) -> Response {
    if let Err(resp) = ensure_user_exists(&pool, user_id).await {
        return resp;
    }

    match LoyaltyService::get_user_loyalty(&pool, user_id).await {
        Ok(loyalty) => (
            StatusCode::OK,
            Json(SuccessResponse::new(loyalty, "Points de fidélité récupérés".to_string()))
        ).into_response(),
        Err(e) => {
            eprintln!("Database error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    error_codes::DATABASE_ERROR.to_string(),
                    "Erreur lors de la récupération des points".to_string()
                ))
            ).into_response()
        }
    }
}

/// GET /api/:commerce_id/loyalty/users/:user_id/discount
pub async fn get_user_discount(
    Path((_commerce_id, user_id)): Path<(String, i32)>,
    Extension(pool): Extension<PgPool>,
) -> Response {
    if let Err(resp) = ensure_user_exists(&pool, user_id).await {
        return resp;
    }

    match LoyaltyService::get_user_discount(&pool, user_id).await {
        Ok(discount) => (
            StatusCode::OK,
            Json(SuccessResponse::new(discount, "Réduction disponible calculée".to_string()))
        ).into_response(),
        Err(e) => {
            eprintln!("Database error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    error_codes::DATABASE_ERROR.to_string(),
                    "Erreur lors du calcul de la réduction".to_string()
                ))
            ).into_response()
        }
    }
}

/// POST /api/:commerce_id/loyalty/users/:user_id/points
pub async fn adjust_user_points(
    Path((_commerce_id, user_id)): Path<(String, i32)>,
    Extension(pool): Extension<PgPool>,
    Json(request): Json<AdjustPointsRequest>,
) -> Response {
    if let Err(resp) = ensure_user_exists(&pool, user_id).await {
        return resp;
    }

    match LoyaltyService::adjust_points(&pool, user_id, request.points, request.reason).await {
        Ok(resp) => {
            let msg = if resp.adjustment > 0 {
                format!("{} points ajoutés avec succès", resp.adjustment)
            } else {
                format!("{} points retirés avec succès", resp.adjustment.abs())
            };
            (
                StatusCode::CREATED,
                Json(SuccessResponse::new(resp, msg))
            ).into_response()
        }
        Err(AdjustPointsError::ZeroAdjustment) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ErrorResponse::new(
                "INVALID_VALUE".to_string(),
                "L'ajustement de points ne peut pas être zéro".to_string()
            ))
        ).into_response(),
        Err(AdjustPointsError::InsufficientBalance { current, requested }) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ErrorResponse::new(
                "INSUFFICIENT_POINTS".to_string(),
                format!(
                    "Solde insuffisant : l'utilisateur a {} points, impossible de retirer {}",
                    current, requested.abs()
                )
            ))
        ).into_response(),
        Err(AdjustPointsError::Db(e)) => {
            eprintln!("Database error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    error_codes::DATABASE_ERROR.to_string(),
                    "Erreur lors de l'ajustement des points".to_string()
                ))
            ).into_response()
        }
    }
}

async fn ensure_user_exists(pool: &PgPool, user_id: i32) -> Result<(), Response> {
    match sqlx::query("SELECT id_usr FROM users_usr WHERE id_usr = $1")
        .bind(user_id)
        .fetch_optional(pool)
        .await
    {
        Ok(Some(_)) => Ok(()),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                error_codes::NOT_FOUND.to_string(),
                format!("Utilisateur {} non trouvé", user_id)
            ))
        ).into_response()),
        Err(e) => {
            eprintln!("Database error: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    error_codes::DATABASE_ERROR.to_string(),
                    "Erreur de base de données".to_string()
                ))
            ).into_response())
        }
    }
}
