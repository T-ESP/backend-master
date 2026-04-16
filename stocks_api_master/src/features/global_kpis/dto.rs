use serde::{Deserialize, Serialize};
use utoipa::{ToSchema, IntoParams};

// ====================== PARAMÈTRES DE REQUÊTE ======================

#[derive(Debug, Deserialize, IntoParams)]
pub struct KpiPeriodParams {
    pub start_date: Option<chrono::NaiveDate>,
    pub end_date: Option<chrono::NaiveDate>,
}

impl Default for KpiPeriodParams {
    fn default() -> Self {
        let end_date = chrono::Utc::now().date_naive();
        let start_date = end_date - chrono::Duration::days(30);
        Self {
            start_date: Some(start_date),
            end_date: Some(end_date),
        }
    }
}

// ====================== 1. PERFORMANCE GLOBALE ======================

/// KPIs de performance globale du catalogue
#[derive(Debug, Serialize, ToSchema)]
pub struct GlobalPerformanceKpis {
    pub total_revenue: f64,
    pub total_profit: f64,
    pub avg_margin_rate: Option<f64>,

    pub total_products_count: i32,
    pub active_products_count: i32,
    pub inactive_products_count: i32,
    pub discontinued_products_count: i32,

    pub total_orders_count: i32,
    pub global_avg_basket_value: Option<f64>,

    pub total_stock_value_cost: f64,
    pub total_stock_value_potential: f64,
}

// ====================== 2. ANALYSES PAR CATÉGORIE ======================

/// Statistiques agrégées par catégorie
#[derive(Debug, Serialize, ToSchema, Clone)]
pub struct CategoryStats {
    pub category: String,
    pub revenue: f64,
    pub profit: f64,
    pub avg_margin_rate: Option<f64>,
    pub products_count: i32,
    pub avg_turnover_rate: Option<f64>,
    pub stock_distribution_percent: Option<f64>,
}

/// KPIs d'analyse par catégorie
#[derive(Debug, Serialize, ToSchema)]
pub struct CategoryAnalysisKpis {
    pub by_category: Vec<CategoryStats>,
    pub top_5_by_revenue: Vec<CategoryStats>,
    pub top_5_by_profit: Vec<CategoryStats>,
    pub top_5_by_volume: Vec<CategoryStats>,
}

// ====================== 3. ANALYSES PAR FOURNISSEUR ======================

/// Statistiques agrégées par fournisseur
#[derive(Debug, Serialize, ToSchema, Clone)]
pub struct SupplierStats {
    pub supplier_id: i32,
    pub supplier_name: String,
    pub revenue: f64,
    pub profit: f64,
    pub products_count: i32,
    pub restocks_count: i32,
    pub total_purchase_cost: f64,
    pub avg_delivery_delay_days: Option<f64>,
    pub reliability_rate: Option<f64>,
    pub cancellation_rate: Option<f64>,
}

/// KPIs d'analyse par fournisseur
#[derive(Debug, Serialize, ToSchema)]
pub struct SupplierAnalysisKpis {
    pub by_supplier: Vec<SupplierStats>,
    pub top_5_by_revenue: Vec<SupplierStats>,
    pub top_5_by_reliability: Vec<SupplierStats>,
    pub top_5_by_cost: Vec<SupplierStats>,
}

// ====================== 4. SANTÉ DU CATALOGUE ======================

/// KPIs de santé du catalogue
#[derive(Debug, Serialize, ToSchema)]
pub struct CatalogHealthKpis {
    pub availability_rate: Option<f64>,
    pub stockout_products_count: i32,
    pub discontinued_products_count: i32,
    pub catalog_renewal_rate: Option<f64>,
    pub low_rotation_products_percent: Option<f64>,
    pub obsolete_products_percent: Option<f64>,
    pub overstocked_products_percent: Option<f64>,
}

// ====================== 5. DISTRIBUTION ABC ======================

/// Informations sur un produit dans la classification ABC
#[derive(Debug, Serialize, ToSchema)]
pub struct AbcProductInfo {
    pub product_id: i32,
    pub product_name: String,
    pub revenue: f64,
}

/// KPIs de distribution ABC
#[derive(Debug, Serialize, ToSchema)]
pub struct AbcDistributionKpis {
    pub products_a_count: i32,
    pub products_a_revenue_percent: Option<f64>,
    pub products_a_list: Vec<AbcProductInfo>,

    pub products_b_count: i32,
    pub products_b_revenue_percent: Option<f64>,
    pub products_b_list: Vec<AbcProductInfo>,

    pub products_c_count: i32,
    pub products_c_revenue_percent: Option<f64>,
    pub products_c_list: Vec<AbcProductInfo>,

    pub top_20_percent_revenue_concentration: Option<f64>,
}

// ====================== 6. ÉVOLUTIONS & TENDANCES ======================

/// KPIs d'évolutions et tendances
#[derive(Debug, Serialize, ToSchema)]
pub struct TrendsKpis {
    pub revenue_growth_percent: Option<f64>,
    pub profit_growth_percent: Option<f64>,
    pub orders_growth_percent: Option<f64>,
    pub basket_value_growth_percent: Option<f64>,
    pub global_trend: String,
    pub seasonality_detected: bool,
}

// ====================== 7. EFFICACITÉ OPÉRATIONNELLE ======================

/// KPIs d'efficacité opérationnelle
#[derive(Debug, Serialize, ToSchema)]
pub struct OperationalEfficiencyKpis {
    pub avg_catalog_turnover_rate: Option<f64>,
    pub avg_storage_duration_days: Option<f64>,
    pub estimated_storage_cost: f64,
    pub service_rate: Option<f64>,
    pub avg_fill_rate: Option<f64>,
    pub avg_restock_frequency_days: Option<f64>,
}

// ====================== 8. ANALYSES DE PRIX ======================

/// KPIs d'analyse de prix
#[derive(Debug, Serialize, ToSchema)]
pub struct PriceAnalysisKpis {
    pub avg_buying_price: Option<f64>,
    pub avg_selling_price: Option<f64>,
    pub weighted_avg_margin: Option<f64>,
    pub price_changes_count: i32,
    pub buying_price_inflation_percent: Option<f64>,
    pub selling_price_evolution_percent: Option<f64>,
}

/// Distribution des marges (histogramme)
#[derive(Debug, Serialize, ToSchema)]
pub struct MarginDistribution {
    pub range: String,
    pub products_count: i32,
    pub percent_of_total: Option<f64>,
}

// ====================== 9. TOP & FLOP ======================

/// Informations sur un produit pour les classements
#[derive(Debug, Serialize, ToSchema)]
pub struct RankingProductInfo {
    pub product_id: i32,
    pub product_name: String,
    pub category: String,
    pub value: f64,
}

/// KPIs Top & Flop
#[derive(Debug, Serialize, ToSchema)]
pub struct TopFlopKpis {
    pub top_10_by_revenue: Vec<RankingProductInfo>,
    pub top_10_by_profit: Vec<RankingProductInfo>,
    pub top_10_by_volume: Vec<RankingProductInfo>,
    pub top_10_by_turnover: Vec<RankingProductInfo>,
    pub flop_10_by_sales: Vec<RankingProductInfo>,
    pub flop_10_by_profit: Vec<RankingProductInfo>,
    pub at_risk_products: Vec<RankingProductInfo>,
}

// ====================== 10. PRÉVISIONS GLOBALES ======================

/// KPIs de prévisions
#[derive(Debug, Serialize, ToSchema)]
pub struct ForecastKpis {
    pub forecasted_revenue_next_month: Option<f64>,
    pub forecasted_revenue_next_3_months: Option<f64>,
    pub cash_needed_for_restocks: f64,
    pub predicted_stockouts_count: i32,
    pub optimization_opportunities_count: i32,
}

/// Opportunité d'optimisation
#[derive(Debug, Serialize, ToSchema)]
pub struct OptimizationOpportunity {
    pub product_id: i32,
    pub product_name: String,
    pub opportunity_type: String,
    pub reason: String,
    pub potential_impact: Option<f64>,
}

// ====================== ÉVOLUTIONS TEMPORELLES (GRAPHIQUES) ======================

/// Point de données pour graphique temporel
#[derive(Debug, Serialize, ToSchema)]
pub struct TimeSeriesPoint {
    pub date: chrono::NaiveDate,
    pub value: f64,
}

/// Évolution temporelle des KPIs pour graphiques
#[derive(Debug, Serialize, ToSchema)]
pub struct TimeSeriesKpis {
    pub revenue_history: Vec<TimeSeriesPoint>,
    pub profit_history: Vec<TimeSeriesPoint>,
    pub orders_history: Vec<TimeSeriesPoint>,
    pub basket_value_history: Vec<TimeSeriesPoint>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kpi_period_params_default() {
        let params = KpiPeriodParams::default();
        assert!(params.start_date.is_some());
        assert!(params.end_date.is_some());

        let start = params.start_date.unwrap();
        let end = params.end_date.unwrap();
        let diff = end - start;
        assert_eq!(diff.num_days(), 30);
    }

    #[test]
    fn test_kpi_period_params_deserialization() {
        let json = r#"{"start_date": "2024-01-01", "end_date": "2024-06-30"}"#;
        let params: Result<KpiPeriodParams, _> = serde_json::from_str(json);
        assert!(params.is_ok());

        let params = params.unwrap();
        assert_eq!(
            params.start_date,
            Some(chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap())
        );
        assert_eq!(
            params.end_date,
            Some(chrono::NaiveDate::from_ymd_opt(2024, 6, 30).unwrap())
        );
    }

    #[test]
    fn test_kpi_period_params_empty() {
        let json = r#"{}"#;
        let params: Result<KpiPeriodParams, _> = serde_json::from_str(json);
        assert!(params.is_ok());

        let params = params.unwrap();
        assert_eq!(params.start_date, None);
        assert_eq!(params.end_date, None);
    }

    #[test]
    fn test_global_performance_kpis_serialization() {
        let kpis = GlobalPerformanceKpis {
            total_revenue: 500000.0,
            total_profit: 150000.0,
            avg_margin_rate: Some(30.0),
            total_products_count: 1000,
            active_products_count: 900,
            inactive_products_count: 70,
            discontinued_products_count: 30,
            total_orders_count: 5000,
            global_avg_basket_value: Some(100.0),
            total_stock_value_cost: 800000.0,
            total_stock_value_potential: 1200000.0,
        };

        let json = serde_json::to_string(&kpis).unwrap();
        assert!(json.contains("\"total_revenue\":500000.0"));
        assert!(json.contains("\"total_profit\":150000.0"));
        assert!(json.contains("\"avg_margin_rate\":30.0"));
        assert!(json.contains("\"total_products_count\":1000"));
        assert!(json.contains("\"active_products_count\":900"));
        assert!(json.contains("\"total_orders_count\":5000"));
    }

    #[test]
    fn test_global_performance_kpis_without_optionals() {
        let kpis = GlobalPerformanceKpis {
            total_revenue: 0.0,
            total_profit: 0.0,
            avg_margin_rate: None,
            total_products_count: 0,
            active_products_count: 0,
            inactive_products_count: 0,
            discontinued_products_count: 0,
            total_orders_count: 0,
            global_avg_basket_value: None,
            total_stock_value_cost: 0.0,
            total_stock_value_potential: 0.0,
        };

        let json = serde_json::to_string(&kpis).unwrap();
        assert!(json.contains("\"avg_margin_rate\":null"));
        assert!(json.contains("\"global_avg_basket_value\":null"));
    }

    #[test]
    fn test_category_stats_serialization() {
        let stats = CategoryStats {
            category: "Electronics".to_string(),
            revenue: 200000.0,
            profit: 60000.0,
            avg_margin_rate: Some(30.0),
            products_count: 150,
            avg_turnover_rate: Some(4.5),
            stock_distribution_percent: Some(25.0),
        };

        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("Electronics"));
        assert!(json.contains("\"revenue\":200000.0"));
        assert!(json.contains("\"products_count\":150"));
    }

    #[test]
    fn test_category_analysis_kpis_serialization() {
        let cat = CategoryStats {
            category: "Food".to_string(),
            revenue: 50000.0,
            profit: 15000.0,
            avg_margin_rate: Some(30.0),
            products_count: 80,
            avg_turnover_rate: Some(8.0),
            stock_distribution_percent: Some(10.0),
        };

        let kpis = CategoryAnalysisKpis {
            by_category: vec![cat.clone()],
            top_5_by_revenue: vec![cat.clone()],
            top_5_by_profit: vec![cat.clone()],
            top_5_by_volume: vec![cat],
        };

        let json = serde_json::to_string(&kpis).unwrap();
        assert!(json.contains("Food"));
        assert!(json.contains("\"by_category\""));
        assert!(json.contains("\"top_5_by_revenue\""));
    }

    #[test]
    fn test_trends_kpis_serialization() {
        let trends = TrendsKpis {
            revenue_growth_percent: Some(12.5),
            profit_growth_percent: Some(8.0),
            orders_growth_percent: Some(15.0),
            basket_value_growth_percent: Some(-3.0),
            global_trend: "increasing".to_string(),
            seasonality_detected: true,
        };

        let json = serde_json::to_string(&trends).unwrap();
        assert!(json.contains("\"revenue_growth_percent\":12.5"));
        assert!(json.contains("\"global_trend\":\"increasing\""));
        assert!(json.contains("\"seasonality_detected\":true"));
    }

    #[test]
    fn test_catalog_health_kpis_serialization() {
        let health = CatalogHealthKpis {
            availability_rate: Some(92.5),
            stockout_products_count: 15,
            discontinued_products_count: 30,
            catalog_renewal_rate: Some(8.0),
            low_rotation_products_percent: Some(12.0),
            obsolete_products_percent: Some(3.5),
            overstocked_products_percent: Some(5.0),
        };

        let json = serde_json::to_string(&health).unwrap();
        assert!(json.contains("\"availability_rate\":92.5"));
        assert!(json.contains("\"stockout_products_count\":15"));
        assert!(json.contains("\"discontinued_products_count\":30"));
    }

    #[test]
    fn test_top_flop_kpis_serialization() {
        let kpis = TopFlopKpis {
            top_10_by_revenue: vec![RankingProductInfo {
                product_id: 1,
                product_name: "Best Seller".to_string(),
                category: "Electronics".to_string(),
                value: 50000.0,
            }],
            top_10_by_profit: vec![],
            top_10_by_volume: vec![],
            top_10_by_turnover: vec![],
            flop_10_by_sales: vec![],
            flop_10_by_profit: vec![],
            at_risk_products: vec![],
        };

        let json = serde_json::to_string(&kpis).unwrap();
        assert!(json.contains("Best Seller"));
        assert!(json.contains("\"value\":50000.0"));
        assert!(json.contains("\"top_10_by_revenue\""));
    }

    #[test]
    fn test_forecast_kpis_serialization() {
        let forecast = ForecastKpis {
            forecasted_revenue_next_month: Some(120000.0),
            forecasted_revenue_next_3_months: Some(350000.0),
            cash_needed_for_restocks: 75000.0,
            predicted_stockouts_count: 5,
            optimization_opportunities_count: 12,
        };

        let json = serde_json::to_string(&forecast).unwrap();
        assert!(json.contains("\"forecasted_revenue_next_month\":120000.0"));
        assert!(json.contains("\"cash_needed_for_restocks\":75000.0"));
        assert!(json.contains("\"predicted_stockouts_count\":5"));
    }

    #[test]
    fn test_time_series_point_serialization() {
        let point = TimeSeriesPoint {
            date: chrono::NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
            value: 42000.0,
        };

        let json = serde_json::to_string(&point).unwrap();
        assert!(json.contains("\"date\":\"2024-06-15\""));
        assert!(json.contains("\"value\":42000.0"));
    }

    #[test]
    fn test_time_series_kpis_serialization() {
        let point = TimeSeriesPoint {
            date: chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            value: 10000.0,
        };

        let kpis = TimeSeriesKpis {
            revenue_history: vec![point],
            profit_history: vec![],
            orders_history: vec![],
            basket_value_history: vec![],
        };

        let json = serde_json::to_string(&kpis).unwrap();
        assert!(json.contains("\"revenue_history\""));
        assert!(json.contains("2024-01-01"));
        assert!(json.contains("10000.0"));
        assert!(json.contains("\"profit_history\":[]"));
    }

    #[test]
    fn test_supplier_stats_serialization() {
        let stats = SupplierStats {
            supplier_id: 3,
            supplier_name: "Acme Corp".to_string(),
            revenue: 80000.0,
            profit: 24000.0,
            products_count: 50,
            restocks_count: 25,
            total_purchase_cost: 56000.0,
            avg_delivery_delay_days: Some(3.5),
            reliability_rate: Some(95.0),
            cancellation_rate: Some(2.0),
        };

        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("Acme Corp"));
        assert!(json.contains("\"supplier_id\":3"));
        assert!(json.contains("\"restocks_count\":25"));
        assert!(json.contains("\"reliability_rate\":95.0"));
    }

    #[test]
    fn test_operational_efficiency_kpis_serialization() {
        let kpis = OperationalEfficiencyKpis {
            avg_catalog_turnover_rate: Some(6.0),
            avg_storage_duration_days: Some(45.0),
            estimated_storage_cost: 25000.0,
            service_rate: Some(98.5),
            avg_fill_rate: Some(75.0),
            avg_restock_frequency_days: Some(14.0),
        };

        let json = serde_json::to_string(&kpis).unwrap();
        assert!(json.contains("\"avg_catalog_turnover_rate\":6.0"));
        assert!(json.contains("\"estimated_storage_cost\":25000.0"));
        assert!(json.contains("\"service_rate\":98.5"));
    }

    #[test]
    fn test_margin_distribution_serialization() {
        let dist = MarginDistribution {
            range: "10-20%".to_string(),
            products_count: 45,
            percent_of_total: Some(22.5),
        };

        let json = serde_json::to_string(&dist).unwrap();
        assert!(json.contains("\"range\":\"10-20%\""));
        assert!(json.contains("\"products_count\":45"));
        assert!(json.contains("\"percent_of_total\":22.5"));
    }

    #[test]
    fn test_optimization_opportunity_serialization() {
        let opp = OptimizationOpportunity {
            product_id: 42,
            product_name: "Slow Mover".to_string(),
            opportunity_type: "reduce_stock".to_string(),
            reason: "Low rotation rate".to_string(),
            potential_impact: Some(5000.0),
        };

        let json = serde_json::to_string(&opp).unwrap();
        assert!(json.contains("Slow Mover"));
        assert!(json.contains("reduce_stock"));
        assert!(json.contains("Low rotation rate"));
        assert!(json.contains("\"potential_impact\":5000.0"));
    }
}
