use serde::{Deserialize, Serialize};
use utoipa::{ToSchema, IntoParams};
use rust_decimal::Decimal;

#[derive(Debug, Serialize, ToSchema)]
pub struct StockResponse {
    pub id: i32,
    pub name: String,
    pub category: String,
    pub reference: String,
    pub supplier_id: i32,
    pub stock_quantity: i32,
    pub buying_price: Decimal,
    pub date_last_reassor: chrono::DateTime<chrono::Utc>,
    pub stock_status: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct StockAlert {
    pub product: StockResponse,
    pub alert_type: String,
    pub severity: String,
    pub message: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct StockSummary {
    pub total_products: i64,
    pub out_of_stock_count: i64,
    pub low_stock_count: i64,
    pub overstock_count: i64,
    pub total_stock_value: Decimal,
    pub categories_affected: Vec<String>,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct StockParams {
    pub limit: Option<i32>,
    pub threshold: Option<i32>,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct OverstockParams {
    pub limit: Option<i32>,
    pub multiplier: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    #[test]
    fn test_stock_response_serialization() {
        let stock = StockResponse {
            id: 1,
            name: "Test Product".to_string(),
            category: "Electronics".to_string(),
            reference: "REF001".to_string(),
            supplier_id: 5,
            stock_quantity: 100,
            buying_price: Decimal::from_str("49.99").unwrap(),
            date_last_reassor: chrono::Utc::now(),
            stock_status: "in_stock".to_string(),
        };

        let json = serde_json::to_string(&stock);
        assert!(json.is_ok());

        let json_str = json.unwrap();
        assert!(json_str.contains("Test Product"));
        assert!(json_str.contains("Electronics"));
        assert!(json_str.contains("REF001"));
        assert!(json_str.contains("in_stock"));
    }

    #[test]
    fn test_stock_alert_serialization() {
        let stock = StockResponse {
            id: 1,
            name: "Low Stock Item".to_string(),
            category: "Hardware".to_string(),
            reference: "REF002".to_string(),
            supplier_id: 3,
            stock_quantity: 5,
            buying_price: Decimal::from_str("25.00").unwrap(),
            date_last_reassor: chrono::Utc::now(),
            stock_status: "low_stock".to_string(),
        };

        let alert = StockAlert {
            product: stock,
            alert_type: "low_stock".to_string(),
            severity: "warning".to_string(),
            message: "Stock level is below threshold".to_string(),
        };

        let json = serde_json::to_string(&alert);
        assert!(json.is_ok());

        let json_str = json.unwrap();
        assert!(json_str.contains("low_stock"));
        assert!(json_str.contains("warning"));
        assert!(json_str.contains("Stock level is below threshold"));
        assert!(json_str.contains("Low Stock Item"));
    }

    #[test]
    fn test_stock_summary_serialization() {
        let summary = StockSummary {
            total_products: 150,
            out_of_stock_count: 10,
            low_stock_count: 25,
            overstock_count: 5,
            total_stock_value: Decimal::from_str("75000.00").unwrap(),
            categories_affected: vec!["Electronics".to_string(), "Hardware".to_string()],
        };

        let json = serde_json::to_string(&summary);
        assert!(json.is_ok());

        let json_str = json.unwrap();
        assert!(json_str.contains("\"total_products\":150"));
        assert!(json_str.contains("\"out_of_stock_count\":10"));
        assert!(json_str.contains("\"low_stock_count\":25"));
        assert!(json_str.contains("\"overstock_count\":5"));
        assert!(json_str.contains("Electronics"));
        assert!(json_str.contains("Hardware"));
    }

    #[test]
    fn test_stock_summary_empty_categories() {
        let summary = StockSummary {
            total_products: 0,
            out_of_stock_count: 0,
            low_stock_count: 0,
            overstock_count: 0,
            total_stock_value: Decimal::ZERO,
            categories_affected: vec![],
        };

        let json = serde_json::to_string(&summary);
        assert!(json.is_ok());

        let json_str = json.unwrap();
        assert!(json_str.contains("\"total_products\":0"));
        assert!(json_str.contains("\"categories_affected\":[]"));
    }

    #[test]
    fn test_stock_params_deserialization() {
        let json = r#"{
            "limit": 50,
            "threshold": 10
        }"#;

        let params: Result<StockParams, _> = serde_json::from_str(json);
        assert!(params.is_ok());

        let params = params.unwrap();
        assert_eq!(params.limit, Some(50));
        assert_eq!(params.threshold, Some(10));
    }

    #[test]
    fn test_stock_params_optional_fields() {
        let json = r#"{}"#;

        let params: Result<StockParams, _> = serde_json::from_str(json);
        assert!(params.is_ok());

        let params = params.unwrap();
        assert_eq!(params.limit, None);
        assert_eq!(params.threshold, None);
    }

    #[test]
    fn test_stock_params_partial() {
        let json = r#"{"limit": 100}"#;

        let params: Result<StockParams, _> = serde_json::from_str(json);
        assert!(params.is_ok());

        let params = params.unwrap();
        assert_eq!(params.limit, Some(100));
        assert_eq!(params.threshold, None);
    }

    #[test]
    fn test_overstock_params_deserialization() {
        let json = r#"{
            "limit": 25,
            "multiplier": 2.5
        }"#;

        let params: Result<OverstockParams, _> = serde_json::from_str(json);
        assert!(params.is_ok());

        let params = params.unwrap();
        assert_eq!(params.limit, Some(25));
        assert_eq!(params.multiplier, Some(2.5));
    }

    #[test]
    fn test_overstock_params_optional_fields() {
        let json = r#"{}"#;

        let params: Result<OverstockParams, _> = serde_json::from_str(json);
        assert!(params.is_ok());

        let params = params.unwrap();
        assert_eq!(params.limit, None);
        assert_eq!(params.multiplier, None);
    }

    #[test]
    fn test_overstock_params_partial() {
        let json = r#"{"multiplier": 1.5}"#;

        let params: Result<OverstockParams, _> = serde_json::from_str(json);
        assert!(params.is_ok());

        let params = params.unwrap();
        assert_eq!(params.limit, None);
        assert_eq!(params.multiplier, Some(1.5));
    }

    #[test]
    fn test_stock_status_values() {
        let statuses = vec!["in_stock", "low_stock", "out_of_stock", "overstock"];

        for status in statuses {
            let stock = StockResponse {
                id: 1,
                name: "Product".to_string(),
                category: "Category".to_string(),
                reference: "REF".to_string(),
                supplier_id: 1,
                stock_quantity: 10,
                buying_price: Decimal::from_str("10.00").unwrap(),
                date_last_reassor: chrono::Utc::now(),
                stock_status: status.to_string(),
            };

            assert_eq!(stock.stock_status, status);
        }
    }

    #[test]
    fn test_alert_severity_levels() {
        let severities = vec!["info", "warning", "critical"];

        for severity in severities {
            let stock = StockResponse {
                id: 1,
                name: "Product".to_string(),
                category: "Category".to_string(),
                reference: "REF".to_string(),
                supplier_id: 1,
                stock_quantity: 0,
                buying_price: Decimal::from_str("10.00").unwrap(),
                date_last_reassor: chrono::Utc::now(),
                stock_status: "out_of_stock".to_string(),
            };

            let alert = StockAlert {
                product: stock,
                alert_type: "out_of_stock".to_string(),
                severity: severity.to_string(),
                message: "Alert message".to_string(),
            };

            assert_eq!(alert.severity, severity);
        }
    }

    #[test]
    fn test_decimal_prices() {
        let prices = vec!["0.01", "99.99", "1234.56", "10000.00"];

        for price_str in prices {
            let stock = StockResponse {
                id: 1,
                name: "Product".to_string(),
                category: "Category".to_string(),
                reference: "REF".to_string(),
                supplier_id: 1,
                stock_quantity: 10,
                buying_price: Decimal::from_str(price_str).unwrap(),
                date_last_reassor: chrono::Utc::now(),
                stock_status: "in_stock".to_string(),
            };

            let json = serde_json::to_string(&stock).unwrap();
            assert!(json.contains(price_str));
        }
    }
}