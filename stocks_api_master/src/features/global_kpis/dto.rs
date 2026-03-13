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
    pub avg_margin_rate: Option<f64>, // % moyen tous produits

    // Nombre de produits
    pub total_products_count: i32,
    pub active_products_count: i32,
    pub inactive_products_count: i32,
    pub discontinued_products_count: i32,

    // Commandes
    pub total_orders_count: i32,
    pub global_avg_basket_value: Option<f64>,

    // Stock
    pub total_stock_value_cost: f64, // stock × prix_achat
    pub total_stock_value_potential: f64, // stock × prix_vente
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
    pub reliability_rate: Option<f64>, // % restocks reçus
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
    pub availability_rate: Option<f64>, // % produits en stock
    pub stockout_products_count: i32,
    pub discontinued_products_count: i32,
    pub catalog_renewal_rate: Option<f64>, // nouveaux / total
    pub low_rotation_products_percent: Option<f64>, // < seuil ventes/mois
    pub obsolete_products_percent: Option<f64>, // pas de vente depuis X jours
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
    // Produits A
    pub products_a_count: i32,
    pub products_a_revenue_percent: Option<f64>,
    pub products_a_list: Vec<AbcProductInfo>,

    // Produits B
    pub products_b_count: i32,
    pub products_b_revenue_percent: Option<f64>,
    pub products_b_list: Vec<AbcProductInfo>,

    // Produits C
    pub products_c_count: i32,
    pub products_c_revenue_percent: Option<f64>,
    pub products_c_list: Vec<AbcProductInfo>,

    // Concentration
    pub top_20_percent_revenue_concentration: Option<f64>,
}

// ====================== 6. ÉVOLUTIONS & TENDANCES ======================

/// KPIs d'évolutions et tendances
#[derive(Debug, Serialize, ToSchema)]
pub struct TrendsKpis {
    pub revenue_growth_percent: Option<f64>, // vs période précédente
    pub profit_growth_percent: Option<f64>,
    pub orders_growth_percent: Option<f64>,
    pub basket_value_growth_percent: Option<f64>,
    pub global_trend: String, // "increasing", "decreasing", "stable"
    pub seasonality_detected: bool,
}

// ====================== 7. EFFICACITÉ OPÉRATIONNELLE ======================

/// KPIs d'efficacité opérationnelle
#[derive(Debug, Serialize, ToSchema)]
pub struct OperationalEfficiencyKpis {
    pub avg_catalog_turnover_rate: Option<f64>,
    pub avg_storage_duration_days: Option<f64>,
    pub estimated_storage_cost: f64,
    pub service_rate: Option<f64>, // % commandes sans rupture
    pub avg_fill_rate: Option<f64>, // stock / capacité max
    pub avg_restock_frequency_days: Option<f64>,
}

// ====================== 8. ANALYSES DE PRIX ======================

/// KPIs d'analyse de prix
#[derive(Debug, Serialize, ToSchema)]
pub struct PriceAnalysisKpis {
    pub avg_buying_price: Option<f64>,
    pub avg_selling_price: Option<f64>,
    pub weighted_avg_margin: Option<f64>, // pondéré par CA
    pub price_changes_count: i32,
    pub buying_price_inflation_percent: Option<f64>,
    pub selling_price_evolution_percent: Option<f64>,
}

/// Distribution des marges (histogramme)
#[derive(Debug, Serialize, ToSchema)]
pub struct MarginDistribution {
    pub range: String, // ex: "0-10%", "10-20%", etc.
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
    pub value: f64, // revenue, profit, volume, ou rotation selon le contexte
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
    pub at_risk_products: Vec<RankingProductInfo>, // faible rotation + stock élevé
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
    pub opportunity_type: String, // "promote", "reduce_stock", "discontinue"
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
