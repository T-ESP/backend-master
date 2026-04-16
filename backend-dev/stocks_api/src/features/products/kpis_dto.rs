use serde::{Deserialize, Serialize};
use utoipa::{ToSchema, IntoParams};

// ====================== 1. PRIX & MARGE ======================

/// KPIs relatifs aux prix et marges du produit
#[derive(Debug, Serialize, ToSchema)]
pub struct PricingMarginKpis {
    // Prix actuels
    pub current_buying_price: f64,
    pub current_selling_price: Option<f64>,

    // Marge
    pub gross_margin: Option<f64>,
    pub margin_rate: Option<f64>, // En pourcentage

    // Historique prix d'achat
    pub buying_price_min: Option<f64>,
    pub buying_price_max: Option<f64>,
    pub buying_price_avg: Option<f64>,
    pub buying_price_variation: Option<f64>, // En pourcentage
    pub buying_price_volatility: Option<f64>, // Écart-type
    pub buying_price_changes_count: i32,

    // Historique prix de vente
    pub selling_price_min: Option<f64>,
    pub selling_price_max: Option<f64>,
    pub selling_price_avg: Option<f64>,
    pub selling_price_variation: Option<f64>, // En pourcentage
    pub selling_price_volatility: Option<f64>, // Écart-type
    pub selling_price_changes_count: i32,

    // Analyses croisées
    pub repercussion_rate: Option<f64>, // Taux de répercussion des hausses de coût
    pub repercussion_delay_days: Option<i32>, // Délai moyen de répercussion
}

// ====================== 2. STOCK & DISPONIBILITÉ ======================

/// KPIs relatifs au stock et à la disponibilité du produit
#[derive(Debug, Serialize, ToSchema)]
pub struct StockAvailabilityKpis {
    pub current_stock: i32,
    pub product_status: String, // in_stock, out_of_stock, discontinued

    // Ruptures de stock
    pub stockout_rate: Option<f64>, // % temps en rupture
    pub stockout_count: i32,
    pub avg_stockout_duration_days: Option<f64>,

    // Recommandations
    pub safety_stock_recommended: Option<i32>,
    pub days_since_last_restock: Option<i32>,
    pub last_restock_date: Option<chrono::DateTime<chrono::Utc>>,
}

// ====================== 3. VENTES & ROTATION ======================

/// KPIs relatifs aux ventes et à la rotation du stock
#[derive(Debug, Serialize, ToSchema)]
pub struct SalesRotationKpis {
    // Volume de ventes
    pub quantity_sold: i32,
    pub revenue: f64,
    pub order_count: i32,
    pub avg_quantity_per_order: Option<f64>,
    pub avg_basket_value: Option<f64>,

    // Rotation
    pub stock_turnover_rate: Option<f64>,
    pub avg_storage_duration_days: Option<f64>,
    pub sales_velocity_per_day: Option<f64>,

    // Tendances
    pub sales_trend: String, // "increasing", "decreasing", "stable"
    pub sales_variation_percent: Option<f64>, // vs période précédente
}

// ====================== 4. RENTABILITÉ ======================

/// KPIs relatifs à la rentabilité du produit
#[derive(Debug, Serialize, ToSchema)]
pub struct ProfitabilityKpis {
    pub total_profit: f64,
    pub avg_profit_per_sale: Option<f64>,
    pub roi: Option<f64>, // Return on Investment en %
    pub contribution_to_total_revenue_percent: Option<f64>,
    pub contribution_to_total_profit_percent: Option<f64>,
}

// ====================== 5. RÉAPPROVISIONNEMENT ======================

/// KPIs relatifs aux réapprovisionnements du produit
#[derive(Debug, Serialize, ToSchema)]
pub struct RestockKpis {
    pub restock_count: i32,
    pub total_restocked_quantity: i32,
    pub avg_quantity_per_restock: Option<f64>,
    pub restock_frequency_days: Option<f64>,

    // Coûts
    pub total_restock_cost: f64,
    pub avg_restock_cost: Option<f64>,

    // Fiabilité
    pub reception_rate: Option<f64>, // % restocks reçus
    pub cancellation_rate: Option<f64>, // % restocks annulés
    pub avg_delivery_delay_days: Option<f64>,
}

// ====================== 6. PRÉDICTIONS & ALERTES ======================

/// KPIs prédictifs et alertes pour la gestion du stock
#[derive(Debug, Serialize, ToSchema)]
pub struct PredictionsAlertsKpis {
    pub estimated_stockout_date: Option<chrono::DateTime<chrono::Utc>>,
    pub optimal_reorder_quantity: Option<i32>,
    pub optimal_reorder_point: Option<i32>,
    pub days_of_coverage: Option<f64>,
    pub alert_status: String,

    // AI-powered predictions (from demand_forecasts table, populated by ai-service)
    pub ai_forecast_available: bool,
    pub ai_total_predicted_demand: Option<f64>,
    pub ai_avg_daily_demand: Option<f64>,
    pub ai_recommended_stock: Option<i32>,
    pub ai_reorder_quantity: Option<i32>,
    pub ai_days_until_stockout: Option<i32>,
    pub ai_urgency: Option<String>,
    pub ai_forecast_accuracy_mape: Option<f64>,
}

// ====================== 7. SCORING & CLASSIFICATION ======================

/// Scores et classifications du produit pour analyse stratégique
#[derive(Debug, Serialize, ToSchema)]
pub struct ScoringClassificationKpis {
    pub popularity_score: f64,
    pub profitability_score: f64,
    pub reliability_score: f64,
    pub global_score: f64,
    pub abc_classification: String,
    pub performance_category: String,

    // AI-powered classification (from product_classifications table, populated by ai-service)
    pub ai_classification_available: bool,
    pub ai_abc_class: Option<String>,
    pub ai_xyz_class: Option<String>,
    pub ai_combined_class: Option<String>,
    pub ai_strategy: Option<String>,
    pub ai_priority: Option<String>,

    // AI-powered clustering (from product_clusters table)
    pub ai_cluster_name: Option<String>,
    pub ai_cluster_id: Option<i32>,
}

// ====================== 8. COMPARAISONS RELATIVES ======================

/// KPIs comparatifs par rapport à la catégorie et au fournisseur
#[derive(Debug, Serialize, ToSchema)]
pub struct ComparativeKpis {
    pub performance_vs_category_percent: Option<f64>,
    pub performance_vs_supplier_percent: Option<f64>,
    pub rank_in_category: Option<i32>,
    pub share_in_category_percent: Option<f64>,
}

// ====================== 9. TOUS LES KPIS (AGRÉGÉ) ======================

/// Tous les KPIs du produit agrégés en une seule réponse
#[derive(Debug, Serialize, ToSchema)]
pub struct AllProductKpis {
    pub pricing_margin: PricingMarginKpis,
    pub stock_availability: StockAvailabilityKpis,
    pub sales_rotation: SalesRotationKpis,
    pub profitability: ProfitabilityKpis,
    pub restock: RestockKpis,
    pub predictions_alerts: PredictionsAlertsKpis,
    pub scoring_classification: ScoringClassificationKpis,
    pub comparative: ComparativeKpis,
    pub price_evolution: PriceEvolution,
}

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

// ====================== ÉVOLUTION DES PRIX (POUR GRAPHIQUES) ======================

/// Point de prix à une date donnée pour affichage graphique
#[derive(Debug, Serialize, ToSchema)]
pub struct PricePoint {
    pub date: chrono::DateTime<chrono::Utc>,
    pub price: f64,
}

/// Évolution des prix d'achat et de vente sur la période pour affichage en graphique
#[derive(Debug, Serialize, ToSchema)]
pub struct PriceEvolution {
    pub buying_price_history: Vec<PricePoint>,
    pub selling_price_history: Vec<PricePoint>,
    pub margin_history: Vec<MarginPoint>,
}

/// Point de marge à une date donnée
#[derive(Debug, Serialize, ToSchema)]
pub struct MarginPoint {
    pub date: chrono::DateTime<chrono::Utc>,
    pub margin_amount: Option<f64>,
    pub margin_rate: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kpi_period_params_default() {
        let default_params = KpiPeriodParams::default();

        assert!(default_params.start_date.is_some());
        assert!(default_params.end_date.is_some());

        // Vérifier que la période par défaut est de 30 jours
        let start = default_params.start_date.unwrap();
        let end = default_params.end_date.unwrap();
        let days_diff = (end - start).num_days();

        assert_eq!(days_diff, 30);
        assert!(start < end);
    }

    #[test]
    fn test_kpi_period_params_deserialization() {
        let json = r#"{
            "start_date": "2024-01-01",
            "end_date": "2024-01-31"
        }"#;

        let params: Result<KpiPeriodParams, _> = serde_json::from_str(json);
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
    fn test_pricing_margin_kpis_serialization() {
        let kpis = PricingMarginKpis {
            current_buying_price: 100.0,
            current_selling_price: Some(150.0),
            gross_margin: Some(50.0),
            margin_rate: Some(33.33),
            buying_price_min: Some(90.0),
            buying_price_max: Some(110.0),
            buying_price_avg: Some(100.0),
            buying_price_variation: Some(5.0),
            buying_price_volatility: Some(2.5),
            buying_price_changes_count: 3,
            selling_price_min: Some(140.0),
            selling_price_max: Some(160.0),
            selling_price_avg: Some(150.0),
            selling_price_variation: Some(3.0),
            selling_price_volatility: Some(1.5),
            selling_price_changes_count: 2,
            repercussion_rate: Some(80.0),
            repercussion_delay_days: Some(7),
        };

        let json = serde_json::to_string(&kpis);
        assert!(json.is_ok());

        let json_str = json.unwrap();
        assert!(json_str.contains("100"));
        assert!(json_str.contains("150"));
    }

    #[test]
    fn test_stock_availability_kpis_serialization() {
        let kpis = StockAvailabilityKpis {
            current_stock: 50,
            product_status: "in_stock".to_string(),
            stockout_rate: Some(10.5),
            stockout_count: 3,
            avg_stockout_duration_days: Some(5.2),
            safety_stock_recommended: Some(20),
            days_since_last_restock: Some(15),
            last_restock_date: Some(chrono::Utc::now()),
        };

        let json = serde_json::to_string(&kpis);
        assert!(json.is_ok());
        assert!(json.unwrap().contains("in_stock"));
    }

    #[test]
    fn test_sales_rotation_kpis_serialization() {
        let kpis = SalesRotationKpis {
            quantity_sold: 100,
            revenue: 5000.0,
            order_count: 25,
            avg_quantity_per_order: Some(4.0),
            avg_basket_value: Some(200.0),
            stock_turnover_rate: Some(8.5),
            avg_storage_duration_days: Some(42.0),
            sales_velocity_per_day: Some(3.33),
            sales_trend: "increasing".to_string(),
            sales_variation_percent: Some(15.5),
        };

        let json = serde_json::to_string(&kpis);
        assert!(json.is_ok());
        assert!(json.unwrap().contains("increasing"));
    }

    #[test]
    fn test_profitability_kpis_serialization() {
        let kpis = ProfitabilityKpis {
            total_profit: 2500.0,
            avg_profit_per_sale: Some(25.0),
            roi: Some(50.0),
            contribution_to_total_revenue_percent: Some(10.0),
            contribution_to_total_profit_percent: Some(12.5),
        };

        let json = serde_json::to_string(&kpis);
        assert!(json.is_ok());

        let json_str = json.unwrap();
        assert!(json_str.contains("2500"));
        assert!(json_str.contains("50"));
    }

    #[test]
    fn test_restock_kpis_serialization() {
        let kpis = RestockKpis {
            restock_count: 10,
            total_restocked_quantity: 500,
            avg_quantity_per_restock: Some(50.0),
            restock_frequency_days: Some(30.0),
            total_restock_cost: 10000.0,
            avg_restock_cost: Some(1000.0),
            reception_rate: Some(95.0),
            cancellation_rate: Some(5.0),
            avg_delivery_delay_days: Some(3.5),
        };

        let json = serde_json::to_string(&kpis);
        assert!(json.is_ok());
        assert!(json.unwrap().contains("10000"));
    }

    #[test]
    fn test_predictions_alerts_kpis_serialization() {
        let kpis = PredictionsAlertsKpis {
            estimated_stockout_date: Some(chrono::Utc::now()),
            optimal_reorder_quantity: Some(100),
            optimal_reorder_point: Some(20),
            days_of_coverage: Some(15.5),
            alert_status: "normal".to_string(),
            ai_forecast_available: false,
            ai_total_predicted_demand: None,
            ai_avg_daily_demand: None,
            ai_recommended_stock: None,
            ai_reorder_quantity: None,
            ai_days_until_stockout: None,
            ai_urgency: None,
            ai_forecast_accuracy_mape: None,
        };

        let json = serde_json::to_string(&kpis);
        assert!(json.is_ok());
        assert!(json.unwrap().contains("normal"));
    }

    #[test]
    fn test_scoring_classification_kpis_serialization() {
        let kpis = ScoringClassificationKpis {
            popularity_score: 85.5,
            profitability_score: 90.0,
            reliability_score: 88.0,
            global_score: 87.83,
            abc_classification: "A".to_string(),
            performance_category: "star".to_string(),
            ai_classification_available: false,
            ai_abc_class: None,
            ai_xyz_class: None,
            ai_combined_class: None,
            ai_strategy: None,
            ai_priority: None,
            ai_cluster_name: None,
            ai_cluster_id: None,
        };

        let json = serde_json::to_string(&kpis);
        assert!(json.is_ok());

        let json_str = json.unwrap();
        assert!(json_str.contains("star"));
        assert!(json_str.contains("\"A\"") || json_str.contains("A"));
    }

    #[test]
    fn test_comparative_kpis_serialization() {
        let kpis = ComparativeKpis {
            performance_vs_category_percent: Some(120.0),
            performance_vs_supplier_percent: Some(110.0),
            rank_in_category: Some(1),
            share_in_category_percent: Some(25.5),
        };

        let json = serde_json::to_string(&kpis);
        assert!(json.is_ok());
        assert!(json.unwrap().contains("120"));
    }

    #[test]
    fn test_price_point_serialization() {
        let now = chrono::Utc::now();
        let point = PricePoint {
            date: now,
            price: 99.99,
        };

        let json = serde_json::to_string(&point);
        assert!(json.is_ok());
        assert!(json.unwrap().contains("99.99"));
    }

    #[test]
    fn test_margin_point_serialization() {
        let now = chrono::Utc::now();
        let point = MarginPoint {
            date: now,
            margin_amount: Some(25.0),
            margin_rate: Some(25.0),
        };

        let json = serde_json::to_string(&point);
        assert!(json.is_ok());

        let json_str = json.unwrap();
        assert!(json_str.contains("25"));
    }

    #[test]
    fn test_price_evolution_serialization() {
        let now = chrono::Utc::now();
        let evolution = PriceEvolution {
            buying_price_history: vec![
                PricePoint {
                    date: now,
                    price: 100.0,
                },
                PricePoint {
                    date: now + chrono::Duration::days(1),
                    price: 105.0,
                },
            ],
            selling_price_history: vec![
                PricePoint {
                    date: now,
                    price: 150.0,
                },
                PricePoint {
                    date: now + chrono::Duration::days(1),
                    price: 155.0,
                },
            ],
            margin_history: vec![
                MarginPoint {
                    date: now,
                    margin_amount: Some(50.0),
                    margin_rate: Some(33.33),
                },
            ],
        };

        let json = serde_json::to_string(&evolution);
        assert!(json.is_ok());
        assert!(json.unwrap().contains("150"));
    }

    #[test]
    fn test_kpi_period_params_with_none_values() {
        let json = r#"{}"#;
        let params: Result<KpiPeriodParams, _> = serde_json::from_str(json);
        assert!(params.is_ok());

        let params = params.unwrap();
        assert_eq!(params.start_date, None);
        assert_eq!(params.end_date, None);
    }

    #[test]
    fn test_alert_status_values() {
        let normal = PredictionsAlertsKpis {
            estimated_stockout_date: None,
            optimal_reorder_quantity: Some(50),
            optimal_reorder_point: Some(10),
            days_of_coverage: Some(30.0),
            alert_status: "normal".to_string(),
            ai_forecast_available: false,
            ai_total_predicted_demand: None,
            ai_avg_daily_demand: None,
            ai_recommended_stock: None,
            ai_reorder_quantity: None,
            ai_days_until_stockout: None,
            ai_urgency: None,
            ai_forecast_accuracy_mape: None,
        };

        let imminent = PredictionsAlertsKpis {
            estimated_stockout_date: Some(chrono::Utc::now()),
            optimal_reorder_quantity: Some(100),
            optimal_reorder_point: Some(20),
            days_of_coverage: Some(3.0),
            alert_status: "imminent_stockout".to_string(),
            ai_forecast_available: false,
            ai_total_predicted_demand: None,
            ai_avg_daily_demand: None,
            ai_recommended_stock: None,
            ai_reorder_quantity: None,
            ai_days_until_stockout: None,
            ai_urgency: None,
            ai_forecast_accuracy_mape: None,
        };

        let overstock = PredictionsAlertsKpis {
            estimated_stockout_date: None,
            optimal_reorder_quantity: Some(0),
            optimal_reorder_point: Some(50),
            days_of_coverage: Some(90.0),
            alert_status: "overstock".to_string(),
            ai_forecast_available: false,
            ai_total_predicted_demand: None,
            ai_avg_daily_demand: None,
            ai_recommended_stock: None,
            ai_reorder_quantity: None,
            ai_days_until_stockout: None,
            ai_urgency: None,
            ai_forecast_accuracy_mape: None,
        };

        assert_eq!(normal.alert_status, "normal");
        assert_eq!(imminent.alert_status, "imminent_stockout");
        assert_eq!(overstock.alert_status, "overstock");
    }

    #[test]
    fn test_sales_trend_values() {
        let trends = vec!["increasing", "decreasing", "stable"];

        for trend in trends {
            let kpis = SalesRotationKpis {
                quantity_sold: 100,
                revenue: 5000.0,
                order_count: 25,
                avg_quantity_per_order: Some(4.0),
                avg_basket_value: Some(200.0),
                stock_turnover_rate: Some(8.5),
                avg_storage_duration_days: Some(42.0),
                sales_velocity_per_day: Some(3.33),
                sales_trend: trend.to_string(),
                sales_variation_percent: Some(15.5),
            };

            assert_eq!(kpis.sales_trend, trend);
        }
    }

    #[test]
    fn test_abc_classification_values() {
        let classifications = vec!["A", "B", "C"];

        for classification in classifications {
            let kpis = ScoringClassificationKpis {
                popularity_score: 85.5,
                profitability_score: 90.0,
                reliability_score: 88.0,
                global_score: 87.83,
                abc_classification: classification.to_string(),
                performance_category: "star".to_string(),
                ai_classification_available: false,
                ai_abc_class: None,
                ai_xyz_class: None,
                ai_combined_class: None,
                ai_strategy: None,
                ai_priority: None,
                ai_cluster_name: None,
                ai_cluster_id: None,
            };

            assert_eq!(kpis.abc_classification, classification);
        }
    }

    #[test]
    fn test_performance_category_values() {
        let categories = vec!["star", "rising", "declining", "dead"];

        for category in categories {
            let kpis = ScoringClassificationKpis {
                popularity_score: 85.5,
                profitability_score: 90.0,
                reliability_score: 88.0,
                global_score: 87.83,
                abc_classification: "A".to_string(),
                performance_category: category.to_string(),
                ai_classification_available: false,
                ai_abc_class: None,
                ai_xyz_class: None,
                ai_combined_class: None,
                ai_strategy: None,
                ai_priority: None,
                ai_cluster_name: None,
                ai_cluster_id: None,
            };

            assert_eq!(kpis.performance_category, category);
        }
    }
}