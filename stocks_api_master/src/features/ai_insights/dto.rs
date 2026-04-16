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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ai_insights_params_default() {
        let params = AiInsightsParams::default();
        assert!(params.start_date.is_some());
        assert!(params.end_date.is_some());

        let start = params.start_date.unwrap();
        let end = params.end_date.unwrap();
        let diff = end - start;
        assert_eq!(diff.num_days(), 30);
    }

    #[test]
    fn test_ai_insights_params_deserialization() {
        let json = r#"{"start_date": "2024-01-01", "end_date": "2024-01-31"}"#;
        let params: Result<AiInsightsParams, _> = serde_json::from_str(json);
        assert!(params.is_ok());

        let params = params.unwrap();
        assert_eq!(
            params.start_date,
            Some(chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap())
        );
        assert_eq!(
            params.end_date,
            Some(chrono::NaiveDate::from_ymd_opt(2024, 1, 31).unwrap())
        );
    }

    #[test]
    fn test_ai_insights_params_empty() {
        let json = r#"{}"#;
        let params: Result<AiInsightsParams, _> = serde_json::from_str(json);
        assert!(params.is_ok());

        let params = params.unwrap();
        assert_eq!(params.start_date, None);
        assert_eq!(params.end_date, None);
    }

    #[test]
    fn test_period_info_serialization() {
        let period = PeriodInfo {
            start_date: chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            end_date: chrono::NaiveDate::from_ymd_opt(2024, 1, 31).unwrap(),
            days_count: 30,
        };

        let json = serde_json::to_string(&period).unwrap();
        assert!(json.contains("\"start_date\":\"2024-01-01\""));
        assert!(json.contains("\"end_date\":\"2024-01-31\""));
        assert!(json.contains("\"days_count\":30"));
    }

    #[test]
    fn test_ai_insights_data_serialization() {
        let period = PeriodInfo {
            start_date: chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            end_date: chrono::NaiveDate::from_ymd_opt(2024, 1, 31).unwrap(),
            days_count: 30,
        };

        let data = AiInsightsData {
            global_performance: GlobalPerformanceKpis {
                total_revenue: 100000.0,
                total_profit: 30000.0,
                avg_margin_rate: Some(30.0),
                total_products_count: 500,
                active_products_count: 450,
                inactive_products_count: 30,
                discontinued_products_count: 20,
                total_orders_count: 1200,
                global_avg_basket_value: Some(83.33),
                total_stock_value_cost: 200000.0,
                total_stock_value_potential: 350000.0,
            },
            top_flop: TopFlopKpis {
                top_10_by_revenue: vec![],
                top_10_by_profit: vec![],
                top_10_by_volume: vec![],
                top_10_by_turnover: vec![],
                flop_10_by_sales: vec![],
                flop_10_by_profit: vec![],
                at_risk_products: vec![],
            },
            stock_alerts: vec![],
            stock_summary: StockSummary {
                total_products: 500,
                out_of_stock_count: 10,
                low_stock_count: 25,
                overstock_count: 5,
                total_stock_value: rust_decimal::Decimal::from(200000),
                categories_affected: vec!["Electronics".to_string()],
            },
            trends: TrendsKpis {
                revenue_growth_percent: Some(5.0),
                profit_growth_percent: Some(3.0),
                orders_growth_percent: Some(8.0),
                basket_value_growth_percent: Some(-2.0),
                global_trend: "increasing".to_string(),
                seasonality_detected: false,
            },
            category_analysis: CategoryAnalysisKpis {
                by_category: vec![],
                top_5_by_revenue: vec![],
                top_5_by_profit: vec![],
                top_5_by_volume: vec![],
            },
            forecast: ForecastKpis {
                forecasted_revenue_next_month: Some(105000.0),
                forecasted_revenue_next_3_months: Some(320000.0),
                cash_needed_for_restocks: 50000.0,
                predicted_stockouts_count: 3,
                optimization_opportunities_count: 7,
            },
            catalog_health: CatalogHealthKpis {
                availability_rate: Some(90.0),
                stockout_products_count: 10,
                discontinued_products_count: 20,
                catalog_renewal_rate: Some(5.0),
                low_rotation_products_percent: Some(15.0),
                obsolete_products_percent: Some(4.0),
                overstocked_products_percent: Some(1.0),
            },
            period,
        };

        let json = serde_json::to_string(&data).unwrap();
        assert!(json.contains("\"total_revenue\":100000.0"));
        assert!(json.contains("\"global_trend\":\"increasing\""));
        assert!(json.contains("\"days_count\":30"));
        assert!(json.contains("\"total_products\":500"));
        assert!(json.contains("Electronics"));
    }
}
