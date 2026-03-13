use serde::{Deserialize, Serialize};
use utoipa::{ToSchema, IntoParams};

use super::super::global_kpis::dto::{
    GlobalPerformanceKpis, TopFlopKpis, TrendsKpis, 
    CategoryAnalysisKpis, ForecastKpis, CatalogHealthKpis
};
use super::super::stocks::dto::{StockAlert, StockSummary};

// ====================== PARAMÈTRES DE REQUÊTE ======================

#[derive(Debug, Deserialize, IntoParams)]
pub struct AiInsightsParams {
    pub start_date: Option<chrono::NaiveDate>,
    pub end_date: Option<chrono::NaiveDate>,
}

impl Default for AiInsightsParams {
    fn default() -> Self {
        let end_date = chrono::Utc::now().date_naive();
        let start_date = end_date - chrono::Duration::days(30);
        Self {
            start_date: Some(start_date),
            end_date: Some(end_date),
        }
    }
}

// ====================== RÉPONSE AGRÉGÉE ======================

/// Données agrégées pour l'analyse IA
#[derive(Debug, Serialize, ToSchema)]
pub struct AiInsightsData {
    /// Performance globale du catalogue
    pub global_performance: GlobalPerformanceKpis,
    
    /// Top et flop produits (les plus/moins vendus)
    pub top_flop: TopFlopKpis,
    
    /// Alertes sur les stocks
    pub stock_alerts: Vec<StockAlert>,
    
    /// Résumé des stocks
    pub stock_summary: StockSummary,
    
    /// Tendances et évolutions
    pub trends: TrendsKpis,
    
    /// Analyse par catégorie
    pub category_analysis: CategoryAnalysisKpis,
    
    /// Prévisions
    pub forecast: ForecastKpis,
    
    /// Santé du catalogue
    pub catalog_health: CatalogHealthKpis,
    
    /// Période analysée
    pub period: PeriodInfo,
}

/// Informations sur la période analysée
#[derive(Debug, Serialize, ToSchema)]
pub struct PeriodInfo {
    pub start_date: chrono::NaiveDate,
    pub end_date: chrono::NaiveDate,
    pub days_count: i64,
}
