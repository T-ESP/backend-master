use serde::{Deserialize, Serialize};
use utoipa::{ToSchema, IntoParams};

#[derive(Deserialize, IntoParams)]
pub struct PeriodQuery {
    pub start_date: String, // "YYYY-MM-DD"
    pub end_date: String,   // "YYYY-MM-DD"
}

/// Grain de temps pour l'évolution des ventes
#[derive(Deserialize, Serialize, ToSchema, Clone, Copy, Debug)]
#[serde(rename_all = "lowercase")]
pub enum TimeGrain {
    Day,
    Week,
    Month,
}

#[derive(Deserialize, IntoParams)]
pub struct EvolutionQuery {
    pub start_date: String, // "YYYY-MM-DD"
    pub end_date: String,   // "YYYY-MM-DD"
    #[serde(default = "default_grain")]
    pub grain: Option<String>, // "day", "week", "month"
}

fn default_grain() -> Option<String> {
    Some("day".to_string())
}

#[derive(Serialize, ToSchema)]
pub struct TotalRevenueResponse {
    pub total_revenue: f64,
}

/// Point de données pour l'évolution temporelle
#[derive(Serialize, ToSchema, Clone)]
pub struct EvolutionDataPoint {
    /// Date au format "YYYY-MM-DD"
    pub date: String,
    /// Chiffre d'affaires pour cette période
    pub revenue: f64,
}

#[derive(Serialize, ToSchema)]
pub struct EvolutionResponse {
    /// Grain de temps utilisé (day, week, month)
    pub grain: String,
    /// Série temporelle des revenus
    pub data: Vec<EvolutionDataPoint>,
}

#[derive(Serialize, ToSchema)]
pub struct ComparisonResponse {
    /// Baseline simple: prévision = total de la période précédente // A modifier lorsqu'on aura les IAs
    pub forecast: f64,
    pub actual: f64,
}

#[derive(Serialize, ToSchema)]
pub struct AverageBasketResponse {
    pub average_basket: f64,
    /// Pourcentage d’évolution vs période précédente
    pub evolution_percentage: Option<f64>,
}

#[derive(Serialize, ToSchema)]
pub struct AverageBasketByClientTypeResponse {
    pub new_clients: f64,
    pub loyal_clients: f64,
}
