use axum::{
    extract::{Path, Query, State},
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use sqlx::PgPool;

use crate::common::responses::{SuccessResponse, ErrorResponse};
use crate::common::error_codes;

use super::kpis_dto::*;
use super::kpis_services::ProductKpisService;
use super::services::ProductService;

/// GET /api/products/:id/kpis/pricing-margin - Obtenir les KPIs de prix et marge
#[utoipa::path(
    get,
    path = "/products/{id}/kpis/pricing-margin",
    tag = "product-kpis",
    params(
        ("id" = i32, Path, description = "Product ID"),
        KpiPeriodParams
    ),
    responses(
        (status = 200, description = "KPIs retrieved successfully", body = PricingMarginKpis),
        (status = 404, description = "Product not found", body = ErrorResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn get_pricing_margin_kpis(
    Path(id): Path<i32>,
    Query(params): Query<KpiPeriodParams>,
    State(pool): State<PgPool>,
) -> Response {
    // Vérifier que le produit existe
    match ProductService::product_exists(&pool, id).await {
        Ok(false) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    error_codes::NOT_FOUND.to_string(),
                    format!("Produit avec ID {} non trouvé", id)
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
        _ => {}
    }

    match ProductKpisService::get_pricing_margin_kpis(&pool, id, &params).await {
        Ok(kpis) => (
            StatusCode::OK,
            Json(SuccessResponse::new(kpis, "KPIs prix et marge récupérés avec succès".to_string()))
        ).into_response(),
        Err(e) => {
            eprintln!("Database error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    error_codes::DATABASE_ERROR.to_string(),
                    "Erreur lors de la récupération des KPIs".to_string()
                ))
            ).into_response()
        }
    }
}

/// GET /api/products/:id/kpis/stock-availability - Obtenir les KPIs de stock
#[utoipa::path(
    get,
    path = "/products/{id}/kpis/stock-availability",
    tag = "product-kpis",
    params(
        ("id" = i32, Path, description = "Product ID"),
        KpiPeriodParams
    ),
    responses(
        (status = 200, description = "KPIs retrieved successfully", body = StockAvailabilityKpis),
        (status = 404, description = "Product not found", body = ErrorResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn get_stock_availability_kpis(
    Path(id): Path<i32>,
    Query(params): Query<KpiPeriodParams>,
    State(pool): State<PgPool>,
) -> Response {
    match ProductService::product_exists(&pool, id).await {
        Ok(false) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    error_codes::NOT_FOUND.to_string(),
                    format!("Produit avec ID {} non trouvé", id)
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
        _ => {}
    }

    match ProductKpisService::get_stock_availability_kpis(&pool, id, &params).await {
        Ok(kpis) => (
            StatusCode::OK,
            Json(SuccessResponse::new(kpis, "KPIs stock et disponibilité récupérés avec succès".to_string()))
        ).into_response(),
        Err(e) => {
            eprintln!("Database error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    error_codes::DATABASE_ERROR.to_string(),
                    "Erreur lors de la récupération des KPIs".to_string()
                ))
            ).into_response()
        }
    }
}

/// GET /api/products/:id/kpis/sales-rotation - Obtenir les KPIs de ventes et rotation
#[utoipa::path(
    get,
    path = "/products/{id}/kpis/sales-rotation",
    tag = "product-kpis",
    params(
        ("id" = i32, Path, description = "Product ID"),
        KpiPeriodParams
    ),
    responses(
        (status = 200, description = "KPIs retrieved successfully", body = SalesRotationKpis),
        (status = 404, description = "Product not found", body = ErrorResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn get_sales_rotation_kpis(
    Path(id): Path<i32>,
    Query(params): Query<KpiPeriodParams>,
    State(pool): State<PgPool>,
) -> Response {
    match ProductService::product_exists(&pool, id).await {
        Ok(false) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    error_codes::NOT_FOUND.to_string(),
                    format!("Produit avec ID {} non trouvé", id)
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
        _ => {}
    }

    match ProductKpisService::get_sales_rotation_kpis(&pool, id, &params).await {
        Ok(kpis) => (
            StatusCode::OK,
            Json(SuccessResponse::new(kpis, "KPIs ventes et rotation récupérés avec succès".to_string()))
        ).into_response(),
        Err(e) => {
            eprintln!("Database error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    error_codes::DATABASE_ERROR.to_string(),
                    "Erreur lors de la récupération des KPIs".to_string()
                ))
            ).into_response()
        }
    }
}

/// GET /api/products/:id/kpis/profitability - Obtenir les KPIs de rentabilité
#[utoipa::path(
    get,
    path = "/products/{id}/kpis/profitability",
    tag = "product-kpis",
    params(
        ("id" = i32, Path, description = "Product ID"),
        KpiPeriodParams
    ),
    responses(
        (status = 200, description = "KPIs retrieved successfully", body = ProfitabilityKpis),
        (status = 404, description = "Product not found", body = ErrorResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn get_profitability_kpis(
    Path(id): Path<i32>,
    Query(params): Query<KpiPeriodParams>,
    State(pool): State<PgPool>,
) -> Response {
    match ProductService::product_exists(&pool, id).await {
        Ok(false) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    error_codes::NOT_FOUND.to_string(),
                    format!("Produit avec ID {} non trouvé", id)
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
        _ => {}
    }

    match ProductKpisService::get_profitability_kpis(&pool, id, &params).await {
        Ok(kpis) => (
            StatusCode::OK,
            Json(SuccessResponse::new(kpis, "KPIs rentabilité récupérés avec succès".to_string()))
        ).into_response(),
        Err(e) => {
            eprintln!("Database error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    error_codes::DATABASE_ERROR.to_string(),
                    "Erreur lors de la récupération des KPIs".to_string()
                ))
            ).into_response()
        }
    }
}

/// GET /api/products/:id/kpis/restock - Obtenir les KPIs de réapprovisionnement
#[utoipa::path(
    get,
    path = "/products/{id}/kpis/restock",
    tag = "product-kpis",
    params(
        ("id" = i32, Path, description = "Product ID"),
        KpiPeriodParams
    ),
    responses(
        (status = 200, description = "KPIs retrieved successfully", body = RestockKpis),
        (status = 404, description = "Product not found", body = ErrorResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn get_restock_kpis(
    Path(id): Path<i32>,
    Query(params): Query<KpiPeriodParams>,
    State(pool): State<PgPool>,
) -> Response {
    match ProductService::product_exists(&pool, id).await {
        Ok(false) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    error_codes::NOT_FOUND.to_string(),
                    format!("Produit avec ID {} non trouvé", id)
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
        _ => {}
    }

    match ProductKpisService::get_restock_kpis(&pool, id, &params).await {
        Ok(kpis) => (
            StatusCode::OK,
            Json(SuccessResponse::new(kpis, "KPIs réapprovisionnement récupérés avec succès".to_string()))
        ).into_response(),
        Err(e) => {
            eprintln!("Database error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    error_codes::DATABASE_ERROR.to_string(),
                    "Erreur lors de la récupération des KPIs".to_string()
                ))
            ).into_response()
        }
    }
}

/// GET /api/products/:id/kpis/predictions-alerts - Obtenir les KPIs de prédictions et alertes
#[utoipa::path(
    get,
    path = "/products/{id}/kpis/predictions-alerts",
    tag = "product-kpis",
    params(
        ("id" = i32, Path, description = "Product ID"),
        KpiPeriodParams
    ),
    responses(
        (status = 200, description = "KPIs prédictifs récupérés avec succès. Inclut date estimée de rupture, quantités optimales et alertes.", body = PredictionsAlertsKpis),
        (status = 404, description = "Product not found", body = ErrorResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn get_predictions_alerts_kpis(
    Path(id): Path<i32>,
    Query(params): Query<KpiPeriodParams>,
    State(pool): State<PgPool>,
) -> Response {
    match ProductService::product_exists(&pool, id).await {
        Ok(false) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    error_codes::NOT_FOUND.to_string(),
                    format!("Produit avec ID {} non trouvé", id)
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
        _ => {}
    }

    match ProductKpisService::get_predictions_alerts_kpis(&pool, id, &params).await {
        Ok(kpis) => (
            StatusCode::OK,
            Json(SuccessResponse::new(kpis, "KPIs prédictions et alertes récupérés avec succès".to_string()))
        ).into_response(),
        Err(e) => {
            eprintln!("Database error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    error_codes::DATABASE_ERROR.to_string(),
                    "Erreur lors de la récupération des KPIs".to_string()
                ))
            ).into_response()
        }
    }
}

/// GET /api/products/:id/kpis/scoring-classification - Obtenir les KPIs de scoring et classification
#[utoipa::path(
    get,
    path = "/products/{id}/kpis/scoring-classification",
    tag = "product-kpis",
    params(
        ("id" = i32, Path, description = "Product ID"),
        KpiPeriodParams
    ),
    responses(
        (status = 200, description = "KPIs retrieved successfully", body = ScoringClassificationKpis),
        (status = 404, description = "Product not found", body = ErrorResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn get_scoring_classification_kpis(
    Path(id): Path<i32>,
    Query(params): Query<KpiPeriodParams>,
    State(pool): State<PgPool>,
) -> Response {
    match ProductService::product_exists(&pool, id).await {
        Ok(false) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    error_codes::NOT_FOUND.to_string(),
                    format!("Produit avec ID {} non trouvé", id)
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
        _ => {}
    }

    match ProductKpisService::get_scoring_classification_kpis(&pool, id, &params).await {
        Ok(kpis) => (
            StatusCode::OK,
            Json(SuccessResponse::new(kpis, "KPIs scoring et classification récupérés avec succès".to_string()))
        ).into_response(),
        Err(e) => {
            eprintln!("Database error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    error_codes::DATABASE_ERROR.to_string(),
                    "Erreur lors de la récupération des KPIs".to_string()
                ))
            ).into_response()
        }
    }
}

/// GET /api/products/:id/kpis/comparative - Obtenir les KPIs comparatifs
#[utoipa::path(
    get,
    path = "/products/{id}/kpis/comparative",
    tag = "product-kpis",
    params(
        ("id" = i32, Path, description = "Product ID"),
        KpiPeriodParams
    ),
    responses(
        (status = 200, description = "KPIs comparatifs récupérés avec succès. Inclut performances relatives, rang et part de marché.", body = ComparativeKpis),
        (status = 404, description = "Product not found", body = ErrorResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn get_comparative_kpis(
    Path(id): Path<i32>,
    Query(params): Query<KpiPeriodParams>,
    State(pool): State<PgPool>,
) -> Response {
    match ProductService::product_exists(&pool, id).await {
        Ok(false) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    error_codes::NOT_FOUND.to_string(),
                    format!("Produit avec ID {} non trouvé", id)
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
        _ => {}
    }

    match ProductKpisService::get_comparative_kpis(&pool, id, &params).await {
        Ok(kpis) => (
            StatusCode::OK,
            Json(SuccessResponse::new(kpis, "KPIs comparatifs récupérés avec succès".to_string()))
        ).into_response(),
        Err(e) => {
            eprintln!("Database error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    error_codes::DATABASE_ERROR.to_string(),
                    "Erreur lors de la récupération des KPIs".to_string()
                ))
            ).into_response()
        }
    }
}

/// GET /api/products/:id/kpis/price-evolution - Obtenir l'évolution des prix pour graphiques
#[utoipa::path(
    get,
    path = "/products/{id}/kpis/price-evolution",
    tag = "product-kpis",
    params(
        ("id" = i32, Path, description = "Product ID"),
        KpiPeriodParams
    ),
    responses(
        (status = 200, description = "Évolution des prix récupérée avec succès. Données structurées pour affichage en graphique ligne.", body = PriceEvolution),
        (status = 404, description = "Product not found", body = ErrorResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn get_price_evolution(
    Path(id): Path<i32>,
    Query(params): Query<KpiPeriodParams>,
    State(pool): State<PgPool>,
) -> Response {
    match ProductService::product_exists(&pool, id).await {
        Ok(false) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    error_codes::NOT_FOUND.to_string(),
                    format!("Produit avec ID {} non trouvé", id)
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
        _ => {}
    }

    match ProductKpisService::get_price_evolution(&pool, id, &params).await {
        Ok(evolution) => (
            StatusCode::OK,
            Json(SuccessResponse::new(evolution, "Évolution des prix récupérée avec succès".to_string()))
        ).into_response(),
        Err(e) => {
            eprintln!("Database error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    error_codes::DATABASE_ERROR.to_string(),
                    "Erreur lors de la récupération de l'évolution des prix".to_string()
                ))
            ).into_response()
        }
    }
}

/// GET /api/products/:id/kpis/all - Obtenir tous les KPIs du produit
#[utoipa::path(
    get,
    path = "/products/{id}/kpis/all",
    tag = "product-kpis",
    params(
        ("id" = i32, Path, description = "Product ID"),
        KpiPeriodParams
    ),
    responses(
        (status = 200, description = "Tous les KPIs récupérés avec succès.", body = AllProductKpis),
        (status = 404, description = "Product not found", body = ErrorResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn get_all_product_kpis(
    Path(id): Path<i32>,
    Query(params): Query<KpiPeriodParams>,
    State(pool): State<PgPool>,
) -> Response {
    match ProductService::product_exists(&pool, id).await {
        Ok(false) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    error_codes::NOT_FOUND.to_string(),
                    format!("Produit avec ID {} non trouvé", id)
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
        _ => {}
    }

    match ProductKpisService::get_all_product_kpis(&pool, id, &params).await {
        Ok(kpis) => (
            StatusCode::OK,
            Json(SuccessResponse::new(kpis, "Tous les KPIs récupérés avec succès".to_string()))
        ).into_response(),
        Err(e) => {
            eprintln!("Database error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    error_codes::DATABASE_ERROR.to_string(),
                    "Erreur lors de la récupération des KPIs".to_string()
                ))
            ).into_response()
        }
    }
}
