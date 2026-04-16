use serde::Serialize;
use utoipa::{ToSchema, IntoParams};
use serde::Deserialize;
use rust_decimal::Decimal;

// ==================== Demand Forecasts ====================

#[derive(Debug, Serialize, ToSchema)]
pub struct ForecastResponse {
    pub id: i32,
    pub product_id: i32,
    pub product_name: Option<String>,
    pub forecast_date: chrono::DateTime<chrono::Utc>,
    pub forecast_days: i32,
    pub total_predicted_demand: Decimal,
    pub avg_daily_demand: Decimal,
    pub current_stock: Option<i32>,
    pub recommended_stock: Option<i32>,
    pub reorder_quantity: Option<i32>,
    pub days_until_stockout: Option<i32>,
    pub urgency: Option<String>,
    pub mape: Option<Decimal>,
    pub rmse: Option<Decimal>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

// ==================== Product Classifications ====================

#[derive(Debug, Serialize, ToSchema)]
pub struct ClassificationResponse {
    pub id: i32,
    pub product_id: i32,
    pub product_name: Option<String>,
    pub abc_class: String,
    pub xyz_class: String,
    pub combined_class: String,
    pub total_revenue: Option<Decimal>,
    pub revenue_contribution_pct: Option<Decimal>,
    pub total_units_sold: Option<i32>,
    pub coefficient_of_variation: Option<Decimal>,
    pub strategy: Option<String>,
    pub priority: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

// ==================== Product Clusters ====================

#[derive(Debug, Serialize, ToSchema)]
pub struct ClusterResponse {
    pub id: i32,
    pub product_id: i32,
    pub product_name: Option<String>,
    pub cluster_id: i32,
    pub cluster_name: Option<String>,
    pub revenue_score: Option<Decimal>,
    pub variability_score: Option<Decimal>,
    pub trend_score: Option<Decimal>,
    pub seasonality_score: Option<Decimal>,
    pub frequency_score: Option<Decimal>,
    pub margin_score: Option<Decimal>,
    pub distance_to_centroid: Option<Decimal>,
    pub n_clusters: Option<i32>,
    pub silhouette_score: Option<Decimal>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

// ==================== Supplier Scores ====================

#[derive(Debug, Serialize, ToSchema)]
pub struct SupplierScoreResponse {
    pub id: i32,
    pub supplier_id: i32,
    pub supplier_name: Option<String>,
    pub overall_score: Decimal,
    pub delivery_score: Option<Decimal>,
    pub quality_score: Option<Decimal>,
    pub lead_time_score: Option<Decimal>,
    pub fulfillment_score: Option<Decimal>,
    pub rating: Option<String>,
    pub total_restocks: Option<i32>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

// ==================== Price Suggestions ====================

#[derive(Debug, Serialize, ToSchema)]
pub struct PriceSuggestionResponse {
    pub id: i32,
    pub product_id: i32,
    pub product_name: Option<String>,
    pub suggested_price: Decimal,
    pub current_price: Decimal,
    pub reason: Option<String>,
    pub confidence: Option<f64>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

// ==================== Price Anomalies ====================

#[derive(Debug, Serialize, ToSchema)]
pub struct PriceAnomalyResponse {
    pub id: i32,
    pub product_id: i32,
    pub product_name: Option<String>,
    pub current_price: Decimal,
    pub expected_price: Decimal,
    pub anomaly_score: Option<f64>,
    pub is_anomaly: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

// ==================== Sales Anomalies ====================

#[derive(Debug, Serialize, ToSchema)]
pub struct SalesAnomalyResponse {
    pub id: i32,
    pub product_id: i32,
    pub product_name: Option<String>,
    pub sales_volume: i32,
    pub expected_sales: Option<f64>,
    pub anomaly_score: Option<f64>,
    pub is_anomaly: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

// ==================== Urgent Restocks ====================

#[derive(Debug, Serialize, ToSchema)]
pub struct UrgentRestockResponse {
    pub product_id: i32,
    pub product_name: Option<String>,
    pub current_stock: Option<i32>,
    pub recommended_stock: Option<i32>,
    pub reorder_quantity: Option<i32>,
    pub days_until_stockout: Option<i32>,
    pub urgency: Option<String>,
    pub avg_daily_demand: Option<Decimal>,
    pub forecast_date: chrono::DateTime<chrono::Utc>,
}

// ==================== Query Params ====================

#[derive(Debug, Deserialize, IntoParams)]
pub struct PaginationParams {
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct AnomalyParams {
    pub limit: Option<i32>,
    pub only_anomalies: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    #[test]
    fn test_forecast_response_serialization() {
        let now = chrono::Utc::now();
        let forecast = ForecastResponse {
            id: 1,
            product_id: 10,
            product_name: Some("Widget A".to_string()),
            forecast_date: now,
            forecast_days: 30,
            total_predicted_demand: Decimal::from_str("150.00").unwrap(),
            avg_daily_demand: Decimal::from_str("5.00").unwrap(),
            current_stock: Some(200),
            recommended_stock: Some(250),
            reorder_quantity: Some(50),
            days_until_stockout: Some(40),
            urgency: Some("low".to_string()),
            mape: Some(Decimal::from_str("8.50").unwrap()),
            rmse: Some(Decimal::from_str("3.20").unwrap()),
            created_at: now,
        };

        let json = serde_json::to_string(&forecast).unwrap();
        assert!(json.contains("\"id\":1"));
        assert!(json.contains("\"product_id\":10"));
        assert!(json.contains("Widget A"));
        assert!(json.contains("\"forecast_days\":30"));
        assert!(json.contains("\"current_stock\":200"));
        assert!(json.contains("\"urgency\":\"low\""));
    }

    #[test]
    fn test_forecast_response_without_optionals() {
        let now = chrono::Utc::now();
        let forecast = ForecastResponse {
            id: 2,
            product_id: 20,
            product_name: None,
            forecast_date: now,
            forecast_days: 7,
            total_predicted_demand: Decimal::from_str("35.00").unwrap(),
            avg_daily_demand: Decimal::from_str("5.00").unwrap(),
            current_stock: None,
            recommended_stock: None,
            reorder_quantity: None,
            days_until_stockout: None,
            urgency: None,
            mape: None,
            rmse: None,
            created_at: now,
        };

        let json = serde_json::to_string(&forecast).unwrap();
        assert!(json.contains("\"product_name\":null"));
        assert!(json.contains("\"current_stock\":null"));
        assert!(json.contains("\"urgency\":null"));
        assert!(json.contains("\"mape\":null"));
    }

    #[test]
    fn test_classification_response_serialization() {
        let now = chrono::Utc::now();
        let classification = ClassificationResponse {
            id: 1,
            product_id: 5,
            product_name: Some("Gadget B".to_string()),
            abc_class: "A".to_string(),
            xyz_class: "X".to_string(),
            combined_class: "AX".to_string(),
            total_revenue: Some(Decimal::from_str("50000.00").unwrap()),
            revenue_contribution_pct: Some(Decimal::from_str("15.50").unwrap()),
            total_units_sold: Some(1000),
            coefficient_of_variation: Some(Decimal::from_str("0.12").unwrap()),
            strategy: Some("tight_control".to_string()),
            priority: Some("high".to_string()),
            created_at: now,
        };

        let json = serde_json::to_string(&classification).unwrap();
        assert!(json.contains("Gadget B"));
        assert!(json.contains("\"abc_class\":\"A\""));
        assert!(json.contains("\"xyz_class\":\"X\""));
        assert!(json.contains("\"combined_class\":\"AX\""));
        assert!(json.contains("tight_control"));
    }

    #[test]
    fn test_cluster_response_serialization() {
        let now = chrono::Utc::now();
        let cluster = ClusterResponse {
            id: 1,
            product_id: 7,
            product_name: Some("Item C".to_string()),
            cluster_id: 2,
            cluster_name: Some("High Performers".to_string()),
            revenue_score: Some(Decimal::from_str("0.85").unwrap()),
            variability_score: Some(Decimal::from_str("0.30").unwrap()),
            trend_score: Some(Decimal::from_str("0.70").unwrap()),
            seasonality_score: Some(Decimal::from_str("0.45").unwrap()),
            frequency_score: Some(Decimal::from_str("0.90").unwrap()),
            margin_score: Some(Decimal::from_str("0.60").unwrap()),
            distance_to_centroid: Some(Decimal::from_str("1.23").unwrap()),
            n_clusters: Some(5),
            silhouette_score: Some(Decimal::from_str("0.72").unwrap()),
            created_at: now,
        };

        let json = serde_json::to_string(&cluster).unwrap();
        assert!(json.contains("Item C"));
        assert!(json.contains("\"cluster_id\":2"));
        assert!(json.contains("High Performers"));
        assert!(json.contains("\"n_clusters\":5"));
    }

    #[test]
    fn test_supplier_score_response_serialization() {
        let now = chrono::Utc::now();
        let score = SupplierScoreResponse {
            id: 1,
            supplier_id: 3,
            supplier_name: Some("Acme Corp".to_string()),
            overall_score: Decimal::from_str("85.50").unwrap(),
            delivery_score: Some(Decimal::from_str("90.00").unwrap()),
            quality_score: Some(Decimal::from_str("88.00").unwrap()),
            lead_time_score: Some(Decimal::from_str("75.00").unwrap()),
            fulfillment_score: Some(Decimal::from_str("92.00").unwrap()),
            rating: Some("A".to_string()),
            total_restocks: Some(50),
            created_at: now,
        };

        let json = serde_json::to_string(&score).unwrap();
        assert!(json.contains("Acme Corp"));
        assert!(json.contains("\"supplier_id\":3"));
        assert!(json.contains("\"rating\":\"A\""));
        assert!(json.contains("\"total_restocks\":50"));
    }

    #[test]
    fn test_price_suggestion_response_serialization() {
        let now = chrono::Utc::now();
        let suggestion = PriceSuggestionResponse {
            id: 1,
            product_id: 15,
            product_name: Some("Premium Widget".to_string()),
            suggested_price: Decimal::from_str("29.99").unwrap(),
            current_price: Decimal::from_str("24.99").unwrap(),
            reason: Some("High demand detected".to_string()),
            confidence: Some(0.92),
            created_at: now,
        };

        let json = serde_json::to_string(&suggestion).unwrap();
        assert!(json.contains("Premium Widget"));
        assert!(json.contains("\"product_id\":15"));
        assert!(json.contains("High demand detected"));
    }

    #[test]
    fn test_price_anomaly_response_serialization() {
        let now = chrono::Utc::now();
        let anomaly = PriceAnomalyResponse {
            id: 1,
            product_id: 8,
            product_name: Some("Outlier Product".to_string()),
            current_price: Decimal::from_str("99.99").unwrap(),
            expected_price: Decimal::from_str("49.99").unwrap(),
            anomaly_score: Some(2.5),
            is_anomaly: true,
            created_at: now,
        };

        let json = serde_json::to_string(&anomaly).unwrap();
        assert!(json.contains("Outlier Product"));
        assert!(json.contains("\"is_anomaly\":true"));
    }

    #[test]
    fn test_sales_anomaly_response_serialization() {
        let now = chrono::Utc::now();
        let anomaly = SalesAnomalyResponse {
            id: 1,
            product_id: 12,
            product_name: Some("Unusual Seller".to_string()),
            sales_volume: 500,
            expected_sales: Some(100.0),
            anomaly_score: Some(3.8),
            is_anomaly: true,
            created_at: now,
        };

        let json = serde_json::to_string(&anomaly).unwrap();
        assert!(json.contains("Unusual Seller"));
        assert!(json.contains("\"sales_volume\":500"));
        assert!(json.contains("\"is_anomaly\":true"));
    }

    #[test]
    fn test_urgent_restock_response_serialization() {
        let now = chrono::Utc::now();
        let urgent = UrgentRestockResponse {
            product_id: 4,
            product_name: Some("Critical Item".to_string()),
            current_stock: Some(2),
            recommended_stock: Some(100),
            reorder_quantity: Some(98),
            days_until_stockout: Some(1),
            urgency: Some("critical".to_string()),
            avg_daily_demand: Some(Decimal::from_str("15.00").unwrap()),
            forecast_date: now,
        };

        let json = serde_json::to_string(&urgent).unwrap();
        assert!(json.contains("Critical Item"));
        assert!(json.contains("\"current_stock\":2"));
        assert!(json.contains("\"days_until_stockout\":1"));
        assert!(json.contains("\"urgency\":\"critical\""));
    }

    #[test]
    fn test_pagination_params_deserialization() {
        let json = r#"{"limit": 25, "offset": 50}"#;
        let params: Result<PaginationParams, _> = serde_json::from_str(json);
        assert!(params.is_ok());

        let params = params.unwrap();
        assert_eq!(params.limit, Some(25));
        assert_eq!(params.offset, Some(50));
    }

    #[test]
    fn test_pagination_params_empty() {
        let json = r#"{}"#;
        let params: Result<PaginationParams, _> = serde_json::from_str(json);
        assert!(params.is_ok());

        let params = params.unwrap();
        assert_eq!(params.limit, None);
        assert_eq!(params.offset, None);
    }

    #[test]
    fn test_anomaly_params_deserialization() {
        let json = r#"{"limit": 10, "only_anomalies": true}"#;
        let params: Result<AnomalyParams, _> = serde_json::from_str(json);
        assert!(params.is_ok());

        let params = params.unwrap();
        assert_eq!(params.limit, Some(10));
        assert_eq!(params.only_anomalies, Some(true));
    }

    #[test]
    fn test_anomaly_params_empty() {
        let json = r#"{}"#;
        let params: Result<AnomalyParams, _> = serde_json::from_str(json);
        assert!(params.is_ok());

        let params = params.unwrap();
        assert_eq!(params.limit, None);
        assert_eq!(params.only_anomalies, None);
    }
}
