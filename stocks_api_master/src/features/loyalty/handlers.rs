use axum::{
    extract::{Path, Extension},
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use sqlx::PgPool;

use crate::common::responses::{SuccessResponse, ErrorResponse};
use crate::common::error_codes;

use super::dto::UpdateLoyaltyConfigRequest;
use super::services::LoyaltyService;

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
    if request.euros_per_point <= rust_decimal::Decimal::ZERO {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ErrorResponse::new(
                "INVALID_VALUE".to_string(),
                "Le ratio euros/point doit être strictement positif".to_string()
            ))
        ).into_response();
    }

    match LoyaltyService::update_config(&pool, request.euros_per_point).await {
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
    let user_exists = sqlx::query("SELECT id_usr FROM users_usr WHERE id_usr = $1")
        .bind(user_id)
        .fetch_optional(&pool)
        .await;

    match user_exists {
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    error_codes::NOT_FOUND.to_string(),
                    format!("Utilisateur {} non trouvé", user_id)
                ))
            ).into_response();
        }
        Err(e) => {
            eprintln!("Database error: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    error_codes::DATABASE_ERROR.to_string(),
                    "Erreur de base de données".to_string()
                ))
            ).into_response();
        }
        Ok(Some(_)) => {}
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
