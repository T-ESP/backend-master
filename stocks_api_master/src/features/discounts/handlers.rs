use axum::{
    extract::{Extension, Path},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use sqlx::PgPool;

use crate::common::{error_codes, responses::{ErrorResponse, SuccessResponse}};
use super::dto::{CheckDiscountRequest, CreateDiscountRequest, DiscountResponse, UpdateDiscountRequest};
use super::services::DiscountService;

pub async fn get_discounts(Extension(pool): Extension<PgPool>) -> Response {
    match DiscountService::get_all(&pool).await {
        Ok(list) => (
            StatusCode::OK,
            Json(SuccessResponse::new(list, "Remises récupérées".to_string())),
        )
            .into_response(),
        Err(e) => {
            eprintln!("DB error: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    error_codes::DATABASE_ERROR.to_string(),
                    "Erreur base de données".to_string(),
                )),
            )
                .into_response()
        }
    }
}

pub async fn get_discount_by_id(
    Path((_commerce_id, id)): Path<(String, i32)>,
    Extension(pool): Extension<PgPool>,
) -> Response {
    match DiscountService::get_by_id(&pool, id).await {
        Ok(Some(d)) => (
            StatusCode::OK,
            Json(SuccessResponse::new(d, "Remise récupérée".to_string())),
        )
            .into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                error_codes::NOT_FOUND.to_string(),
                format!("Remise {} non trouvée", id),
            )),
        )
            .into_response(),
        Err(e) => {
            eprintln!("DB error: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    error_codes::DATABASE_ERROR.to_string(),
                    "Erreur base de données".to_string(),
                )),
            )
                .into_response()
        }
    }
}

pub async fn create_discount(
    Extension(pool): Extension<PgPool>,
    Json(req): Json<CreateDiscountRequest>,
) -> Response {
    match DiscountService::create(&pool, &req).await {
        Ok(d) => (
            StatusCode::CREATED,
            Json(SuccessResponse::new(d, "Remise créée".to_string())),
        )
            .into_response(),
        Err(e) => {
            eprintln!("DB error: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    error_codes::DATABASE_ERROR.to_string(),
                    "Erreur lors de la création de la remise".to_string(),
                )),
            )
                .into_response()
        }
    }
}

pub async fn update_discount(
    Path((_commerce_id, id)): Path<(String, i32)>,
    Extension(pool): Extension<PgPool>,
    Json(req): Json<UpdateDiscountRequest>,
) -> Response {
    match DiscountService::update(&pool, id, &req).await {
        Ok(Some(d)) => (
            StatusCode::OK,
            Json(SuccessResponse::new(d, "Remise mise à jour".to_string())),
        )
            .into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                error_codes::NOT_FOUND.to_string(),
                format!("Remise {} non trouvée", id),
            )),
        )
            .into_response(),
        Err(e) => {
            eprintln!("DB error: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    error_codes::DATABASE_ERROR.to_string(),
                    "Erreur lors de la mise à jour".to_string(),
                )),
            )
                .into_response()
        }
    }
}

pub async fn delete_discount(
    Path((_commerce_id, id)): Path<(String, i32)>,
    Extension(pool): Extension<PgPool>,
) -> Response {
    match DiscountService::delete(&pool, id).await {
        Ok(0) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                error_codes::NOT_FOUND.to_string(),
                format!("Remise {} non trouvée", id),
            )),
        )
            .into_response(),
        Ok(_) => (
            StatusCode::OK,
            Json(SuccessResponse::new((), format!("Remise {} supprimée", id))),
        )
            .into_response(),
        Err(e) => {
            eprintln!("DB error: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    error_codes::DATABASE_ERROR.to_string(),
                    "Erreur lors de la suppression".to_string(),
                )),
            )
                .into_response()
        }
    }
}

pub async fn check_discounts(
    Extension(pool): Extension<PgPool>,
    Json(req): Json<CheckDiscountRequest>,
) -> Response {
    match DiscountService::check_discounts(&pool, &req).await {
        Ok(result) => (
            StatusCode::OK,
            Json(SuccessResponse::new(result, "Remises applicables".to_string())),
        )
            .into_response(),
        Err(e) => {
            eprintln!("DB error: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    error_codes::DATABASE_ERROR.to_string(),
                    "Erreur lors du calcul des remises".to_string(),
                )),
            )
                .into_response()
        }
    }
}

pub async fn get_discount_orders(
    Path((_commerce_id, id)): Path<(String, i32)>,
    Extension(pool): Extension<PgPool>,
) -> Response {
    match DiscountService::get_usage_orders(&pool, id).await {
        Ok(orders) => (
            StatusCode::OK,
            Json(SuccessResponse::new(orders, "Commandes récupérées".to_string())),
        )
            .into_response(),
        Err(e) => {
            eprintln!("DB error: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    error_codes::DATABASE_ERROR.to_string(),
                    "Erreur base de données".to_string(),
                )),
            )
                .into_response()
        }
    }
}

pub async fn get_order_discounts(
    Path((_commerce_id, order_id)): Path<(String, i32)>,
    Extension(pool): Extension<PgPool>,
) -> Response {
    match DiscountService::get_order_discounts(&pool, order_id).await {
        Ok(discounts) => (
            StatusCode::OK,
            Json(SuccessResponse::new(discounts, "Remises de la commande".to_string())),
        )
            .into_response(),
        Err(e) => {
            eprintln!("DB error: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    error_codes::DATABASE_ERROR.to_string(),
                    "Erreur base de données".to_string(),
                )),
            )
                .into_response()
        }
    }
}
