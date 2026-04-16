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
use super::services::GlobalKpisService;

/// GET /api/kpis/global-performance - Obtenir les KPIs de performance globale
#[utoipa::path(
    get,
    path = "/kpis/global-performance",
    tag = "global-kpis",
    params(
        KpiPeriodParams
    ),
    responses(
        (status = 200, description = "KPIs retrieved successfully", body = GlobalPerformanceKpis),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn get_global_performance_kpis(
    Query(params): Query<KpiPeriodParams>,
    State(pool): State<PgPool>,
) -> Response {
    match GlobalKpisService::get_global_performance_kpis(&pool, &params).await {
        Ok(kpis) => (
            StatusCode::OK,
            Json(SuccessResponse::new(kpis, "KPIs de performance globale récupérés avec succès".to_string()))
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

/// GET /api/kpis/category-analysis - Obtenir les KPIs par catégorie
#[utoipa::path(
    get,
    path = "/kpis/category-analysis",
    tag = "global-kpis",
    params(
        KpiPeriodParams
    ),
    responses(
        (status = 200, description = "KPIs retrieved successfully", body = CategoryAnalysisKpis),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn get_category_analysis_kpis(
    Query(params): Query<KpiPeriodParams>,
    State(pool): State<PgPool>,
) -> Response {
    match GlobalKpisService::get_category_analysis_kpis(&pool, &params).await {
        Ok(kpis) => (
            StatusCode::OK,
            Json(SuccessResponse::new(kpis, "KPIs d'analyse par catégorie récupérés avec succès".to_string()))
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

/// GET /api/kpis/supplier-analysis - Obtenir les KPIs par fournisseur
#[utoipa::path(
    get,
    path = "/kpis/supplier-analysis",
    tag = "global-kpis",
    params(
        KpiPeriodParams
    ),
    responses(
        (status = 200, description = "KPIs retrieved successfully", body = SupplierAnalysisKpis),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn get_supplier_analysis_kpis(
    Query(params): Query<KpiPeriodParams>,
    State(pool): State<PgPool>,
) -> Response {
    match GlobalKpisService::get_supplier_analysis_kpis(&pool, &params).await {
        Ok(kpis) => (
            StatusCode::OK,
            Json(SuccessResponse::new(kpis, "KPIs d'analyse par fournisseur récupérés avec succès".to_string()))
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

/// GET /api/kpis/catalog-health - Obtenir les KPIs de santé du catalogue
#[utoipa::path(
    get,
    path = "/kpis/catalog-health",
    tag = "global-kpis",
    params(
        KpiPeriodParams
    ),
    responses(
        (status = 200, description = "KPIs retrieved successfully", body = CatalogHealthKpis),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn get_catalog_health_kpis(
    Query(params): Query<KpiPeriodParams>,
    State(pool): State<PgPool>,
) -> Response {
    match GlobalKpisService::get_catalog_health_kpis(&pool, &params).await {
        Ok(kpis) => (
            StatusCode::OK,
            Json(SuccessResponse::new(kpis, "KPIs de santé du catalogue récupérés avec succès".to_string()))
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

/// GET /api/kpis/abc-distribution - Obtenir la distribution ABC
#[utoipa::path(
    get,
    path = "/kpis/abc-distribution",
    tag = "global-kpis",
    params(
        KpiPeriodParams
    ),
    responses(
        (status = 200, description = "KPIs retrieved successfully", body = AbcDistributionKpis),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn get_abc_distribution_kpis(
    Query(params): Query<KpiPeriodParams>,
    State(pool): State<PgPool>,
) -> Response {
    match GlobalKpisService::get_abc_distribution_kpis(&pool, &params).await {
        Ok(kpis) => (
            StatusCode::OK,
            Json(SuccessResponse::new(kpis, "Distribution ABC récupérée avec succès".to_string()))
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

/// GET /api/kpis/trends - Obtenir les évolutions et tendances
#[utoipa::path(
    get,
    path = "/kpis/trends",
    tag = "global-kpis",
    params(
        KpiPeriodParams
    ),
    responses(
        (status = 200, description = "KPIs retrieved successfully", body = TrendsKpis),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn get_trends_kpis(
    Query(params): Query<KpiPeriodParams>,
    State(pool): State<PgPool>,
) -> Response {
    match GlobalKpisService::get_trends_kpis(&pool, &params).await {
        Ok(kpis) => (
            StatusCode::OK,
            Json(SuccessResponse::new(kpis, "KPIs d'évolutions et tendances récupérés avec succès".to_string()))
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

/// GET /api/kpis/operational-efficiency - Obtenir les KPIs d'efficacité opérationnelle
#[utoipa::path(
    get,
    path = "/kpis/operational-efficiency",
    tag = "global-kpis",
    params(
        KpiPeriodParams
    ),
    responses(
        (status = 200, description = "KPIs retrieved successfully", body = OperationalEfficiencyKpis),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn get_operational_efficiency_kpis(
    Query(params): Query<KpiPeriodParams>,
    State(pool): State<PgPool>,
) -> Response {
    match GlobalKpisService::get_operational_efficiency_kpis(&pool, &params).await {
        Ok(kpis) => (
            StatusCode::OK,
            Json(SuccessResponse::new(kpis, "KPIs d'efficacité opérationnelle récupérés avec succès".to_string()))
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

/// GET /api/kpis/price-analysis - Obtenir les KPIs d'analyse de prix
#[utoipa::path(
    get,
    path = "/kpis/price-analysis",
    tag = "global-kpis",
    params(
        KpiPeriodParams
    ),
    responses(
        (status = 200, description = "KPIs retrieved successfully", body = PriceAnalysisKpis),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn get_price_analysis_kpis(
    Query(params): Query<KpiPeriodParams>,
    State(pool): State<PgPool>,
) -> Response {
    match GlobalKpisService::get_price_analysis_kpis(&pool, &params).await {
        Ok(kpis) => (
            StatusCode::OK,
            Json(SuccessResponse::new(kpis, "KPIs d'analyse de prix récupérés avec succès".to_string()))
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

/// GET /api/kpis/top-flop - Obtenir les tops et flops
#[utoipa::path(
    get,
    path = "/kpis/top-flop",
    tag = "global-kpis",
    params(
        KpiPeriodParams
    ),
    responses(
        (status = 200, description = "KPIs retrieved successfully", body = TopFlopKpis),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn get_top_flop_kpis(
    Query(params): Query<KpiPeriodParams>,
    State(pool): State<PgPool>,
) -> Response {
    match GlobalKpisService::get_top_flop_kpis(&pool, &params).await {
        Ok(kpis) => (
            StatusCode::OK,
            Json(SuccessResponse::new(kpis, "KPIs top et flop récupérés avec succès".to_string()))
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

/// GET /api/kpis/forecast - Obtenir les prévisions globales
#[utoipa::path(
    get,
    path = "/kpis/forecast",
    tag = "global-kpis",
    params(
        KpiPeriodParams
    ),
    responses(
        (status = 200, description = "KPIs retrieved successfully", body = ForecastKpis),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn get_forecast_kpis(
    Query(params): Query<KpiPeriodParams>,
    State(pool): State<PgPool>,
) -> Response {
    match GlobalKpisService::get_forecast_kpis(&pool, &params).await {
        Ok(kpis) => (
            StatusCode::OK,
            Json(SuccessResponse::new(kpis, "KPIs de prévisions récupérés avec succès".to_string()))
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

/// GET /api/kpis/time-series - Obtenir les évolutions temporelles
#[utoipa::path(
    get,
    path = "/kpis/time-series",
    tag = "global-kpis",
    params(
        KpiPeriodParams
    ),
    responses(
        (status = 200, description = "KPIs retrieved successfully", body = TimeSeriesKpis),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn get_time_series_kpis(
    Query(params): Query<KpiPeriodParams>,
    State(pool): State<PgPool>,
) -> Response {
    match GlobalKpisService::get_time_series_kpis(&pool, &params).await {
        Ok(kpis) => (
            StatusCode::OK,
            Json(SuccessResponse::new(kpis, "Évolutions temporelles récupérées avec succès".to_string()))
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
