use sqlx::PgPool;
use super::dto::*;
use super::super::global_kpis::dto::KpiPeriodParams;
use super::super::global_kpis::services::GlobalKpisService;
use super::super::stocks::services::{get_stock_alerts_data, get_stock_summary_data};

pub struct AiInsightsService;

impl AiInsightsService {
    pub async fn get_ai_insights(
        pool: &PgPool,
        params: &AiInsightsParams,
    ) -> Result<AiInsightsData, sqlx::Error> {
        let start_date = params.start_date.unwrap_or_else(|| {
            (chrono::Utc::now() - chrono::Duration::days(30)).date_naive()
        });
        let end_date = params.end_date.unwrap_or_else(|| chrono::Utc::now().date_naive());

        let kpi_params = KpiPeriodParams {
            start_date: Some(start_date),
            end_date: Some(end_date),
        };

        // Récupérer toutes les données en parallèle pour optimiser les performances
        let (
            global_performance,
            top_flop,
            stock_alerts,
            stock_summary,
            trends,
            category_analysis,
            forecast,
            catalog_health,
        ) = tokio::try_join!(
            GlobalKpisService::get_global_performance_kpis(pool, &kpi_params),
            GlobalKpisService::get_top_flop_kpis(pool, &kpi_params),
            get_stock_alerts_data(pool, 100),
            get_stock_summary_data(pool),
            GlobalKpisService::get_trends_kpis(pool, &kpi_params),
            GlobalKpisService::get_category_analysis_kpis(pool, &kpi_params),
            GlobalKpisService::get_forecast_kpis(pool, &kpi_params),
            GlobalKpisService::get_catalog_health_kpis(pool, &kpi_params),
        )?;

        let days_count = (end_date - start_date).num_days();

        Ok(AiInsightsData {
            global_performance,
            top_flop,
            stock_alerts,
            stock_summary,
            trends,
            category_analysis,
            forecast,
            catalog_health,
            period: PeriodInfo {
                start_date,
                end_date,
                days_count,
            },
        })
    }
}
