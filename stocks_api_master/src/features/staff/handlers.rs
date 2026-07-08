use axum::{
    extract::{Extension, Path},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use sqlx::PgPool;

use crate::common::{
    error_codes,
    responses::{ErrorResponse, SuccessResponse},
    security::Claims,
};

use super::{
    dto::{CreateStaffRequest, UpdateStaffRequest},
    services::{self, StaffServiceError},
};

/// Seul le compte commerce (le gérant) peut gérer ses employés.
fn require_commerce(claims: &Claims) -> Option<Response> {
    if claims.role != "commerce" {
        return Some(
            (
                StatusCode::FORBIDDEN,
                Json(ErrorResponse::new(
                    error_codes::FORBIDDEN.to_string(),
                    "Seul le compte du commerce peut gérer les employés".to_string(),
                )),
            )
                .into_response(),
        );
    }
    None
}

pub(crate) fn service_error_response(err: StaffServiceError) -> Response {
    match err {
        StaffServiceError::EmailAlreadyExists => (
            StatusCode::CONFLICT,
            Json(ErrorResponse::new(
                error_codes::ALREADY_EXISTS.to_string(),
                "Un employé avec cet email existe déjà".to_string(),
            )),
        )
            .into_response(),
        StaffServiceError::NotFound => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                error_codes::NOT_FOUND.to_string(),
                "Employé non trouvé".to_string(),
            )),
        )
            .into_response(),
        StaffServiceError::Database(e) => {
            eprintln!("Database error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    error_codes::DATABASE_ERROR.to_string(),
                    "Erreur de base de données".to_string(),
                )),
            )
                .into_response()
        }
        StaffServiceError::Security(e) => {
            eprintln!("Security error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    error_codes::INTERNAL_SERVER_ERROR.to_string(),
                    "Erreur de sécurité".to_string(),
                )),
            )
                .into_response()
        }
    }
}

pub async fn get_staff(Extension(claims): Extension<Claims>, Extension(pool): Extension<PgPool>) -> Response {
    if let Some(forbidden) = require_commerce(&claims) {
        return forbidden;
    }

    match services::get_all_staff(&pool).await {
        Ok(staff) => (
            StatusCode::OK,
            Json(SuccessResponse::new(staff, "Employés récupérés avec succès".to_string())),
        )
            .into_response(),
        Err(e) => service_error_response(e),
    }
}

pub async fn get_staff_by_id(
    Extension(claims): Extension<Claims>,
    Path((_commerce_id, id)): Path<(String, i32)>,
    Extension(pool): Extension<PgPool>,
) -> Response {
    if let Some(forbidden) = require_commerce(&claims) {
        return forbidden;
    }

    match services::get_staff_by_id(&pool, id).await {
        Ok(Some(staff)) => (
            StatusCode::OK,
            Json(SuccessResponse::new(staff, "Employé récupéré avec succès".to_string())),
        )
            .into_response(),
        Ok(None) => service_error_response(StaffServiceError::NotFound),
        Err(e) => service_error_response(e),
    }
}

pub async fn create_staff(
    Extension(claims): Extension<Claims>,
    Extension(pool): Extension<PgPool>,
    Json(payload): Json<CreateStaffRequest>,
) -> Response {
    if let Some(forbidden) = require_commerce(&claims) {
        return forbidden;
    }

    match services::create_staff(&pool, payload).await {
        Ok(staff) => (
            StatusCode::CREATED,
            Json(SuccessResponse::new(staff, "Employé créé avec succès".to_string())),
        )
            .into_response(),
        Err(e) => service_error_response(e),
    }
}

pub async fn update_staff(
    Extension(claims): Extension<Claims>,
    Path((_commerce_id, id)): Path<(String, i32)>,
    Extension(pool): Extension<PgPool>,
    Json(payload): Json<UpdateStaffRequest>,
) -> Response {
    if let Some(forbidden) = require_commerce(&claims) {
        return forbidden;
    }

    match services::update_staff(&pool, id, payload).await {
        Ok(staff) => (
            StatusCode::OK,
            Json(SuccessResponse::new(staff, "Employé mis à jour avec succès".to_string())),
        )
            .into_response(),
        Err(e) => service_error_response(e),
    }
}

pub async fn delete_staff(
    Extension(claims): Extension<Claims>,
    Path((_commerce_id, id)): Path<(String, i32)>,
    Extension(pool): Extension<PgPool>,
) -> Response {
    if let Some(forbidden) = require_commerce(&claims) {
        return forbidden;
    }

    match services::delete_staff(&pool, id).await {
        Ok(_) => (
            StatusCode::NO_CONTENT,
            Json(SuccessResponse::new(
                "Employé supprimé avec succès".to_string(),
                "Employé supprimé".to_string(),
            )),
        )
            .into_response(),
        Err(e) => service_error_response(e),
    }
}
