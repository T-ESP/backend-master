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
    /// Baseline simple: prévision = total de la période précédente
    pub forecast: f64,
    pub actual: f64,
}

#[derive(Serialize, ToSchema)]
pub struct AverageBasketResponse {
    pub average_basket: f64,
    /// Pourcentage d'évolution vs période précédente
    pub evolution_percentage: Option<f64>,
}

#[derive(Serialize, ToSchema)]
pub struct AverageBasketByClientTypeResponse {
    pub new_clients: f64,
    pub loyal_clients: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_grain_serialization() {
        let day = TimeGrain::Day;
        let json = serde_json::to_string(&day).unwrap();
        assert!(json.contains("day"));

        let week = TimeGrain::Week;
        let json = serde_json::to_string(&week).unwrap();
        assert!(json.contains("week"));

        let month = TimeGrain::Month;
        let json = serde_json::to_string(&month).unwrap();
        assert!(json.contains("month"));
    }

    #[test]
    fn test_time_grain_deserialization() {
        let json = r#""day""#;
        let grain: Result<TimeGrain, _> = serde_json::from_str(json);
        assert!(grain.is_ok());

        let json = r#""week""#;
        let grain: Result<TimeGrain, _> = serde_json::from_str(json);
        assert!(grain.is_ok());

        let json = r#""month""#;
        let grain: Result<TimeGrain, _> = serde_json::from_str(json);
        assert!(grain.is_ok());
    }

    #[test]
    fn test_period_query_deserialization() {
        let json = r#"{"start_date": "2024-01-01", "end_date": "2024-01-31"}"#;
        let query: Result<PeriodQuery, _> = serde_json::from_str(json);
        assert!(query.is_ok());

        let query = query.unwrap();
        assert_eq!(query.start_date, "2024-01-01");
        assert_eq!(query.end_date, "2024-01-31");
    }

    #[test]
    fn test_evolution_query_deserialization() {
        let json = r#"{"start_date": "2024-01-01", "end_date": "2024-03-31", "grain": "month"}"#;
        let query: Result<EvolutionQuery, _> = serde_json::from_str(json);
        assert!(query.is_ok());

        let query = query.unwrap();
        assert_eq!(query.start_date, "2024-01-01");
        assert_eq!(query.end_date, "2024-03-31");
        assert_eq!(query.grain, Some("month".to_string()));
    }

    #[test]
    fn test_evolution_query_default_grain() {
        let json = r#"{"start_date": "2024-01-01", "end_date": "2024-01-31"}"#;
        let query: Result<EvolutionQuery, _> = serde_json::from_str(json);
        assert!(query.is_ok());

        let query = query.unwrap();
        assert_eq!(query.grain, Some("day".to_string()));
    }

    #[test]
    fn test_total_revenue_response_serialization() {
        let response = TotalRevenueResponse {
            total_revenue: 150000.50,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"total_revenue\":150000.5"));
    }

    #[test]
    fn test_evolution_response_serialization() {
        let response = EvolutionResponse {
            grain: "day".to_string(),
            data: vec![
                EvolutionDataPoint {
                    date: "2024-01-01".to_string(),
                    revenue: 5000.0,
                },
                EvolutionDataPoint {
                    date: "2024-01-02".to_string(),
                    revenue: 7500.0,
                },
            ],
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"grain\":\"day\""));
        assert!(json.contains("2024-01-01"));
        assert!(json.contains("2024-01-02"));
        assert!(json.contains("5000.0"));
        assert!(json.contains("7500.0"));
    }

    #[test]
    fn test_comparison_response_serialization() {
        let response = ComparisonResponse {
            forecast: 100000.0,
            actual: 95000.0,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"forecast\":100000.0"));
        assert!(json.contains("\"actual\":95000.0"));
    }

    #[test]
    fn test_average_basket_response_serialization() {
        let response = AverageBasketResponse {
            average_basket: 75.50,
            evolution_percentage: Some(12.3),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"average_basket\":75.5"));
        assert!(json.contains("\"evolution_percentage\":12.3"));
    }

    #[test]
    fn test_average_basket_response_without_evolution() {
        let response = AverageBasketResponse {
            average_basket: 60.0,
            evolution_percentage: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"average_basket\":60.0"));
        assert!(json.contains("\"evolution_percentage\":null"));
    }

    #[test]
    fn test_average_basket_by_client_type_serialization() {
        let response = AverageBasketByClientTypeResponse {
            new_clients: 45.00,
            loyal_clients: 85.50,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"new_clients\":45.0"));
        assert!(json.contains("\"loyal_clients\":85.5"));
    }
}
