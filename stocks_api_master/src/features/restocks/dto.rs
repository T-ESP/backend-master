use serde::{Deserialize, Serialize};
use utoipa::{ToSchema, IntoParams};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

// ---- Enums ----
#[derive(Debug, Serialize, Deserialize, ToSchema, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RestockStatus {
    Pending,
    InTransit,
    Received,
    Cancelled,
}

impl RestockStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            RestockStatus::Pending => "pending",
            RestockStatus::InTransit => "in_transit",
            RestockStatus::Received => "received",
            RestockStatus::Cancelled => "cancelled",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(RestockStatus::Pending),
            "in_transit" => Some(RestockStatus::InTransit),
            "received" => Some(RestockStatus::Received),
            "cancelled" => Some(RestockStatus::Cancelled),
            _ => None,
        }
    }
}

// ---- Response DTOs ----
#[derive(Debug, Serialize, ToSchema)]
pub struct LineRestockResponse {
    pub product_id: i32,
    pub product_name: String,
    pub product_reference: String,
    pub quantity: i32,
    pub unit_price: Decimal,
    pub total_price: Decimal,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RestockResponse {
    pub id: i32,
    pub quantity: i32,
    pub supplier_id: Option<i32>,
    pub status: RestockStatus,
    pub restock_date: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub lines: Vec<LineRestockResponse>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LineRestockWithSupplierResponse {
    pub product_id: i32,
    pub product_name: String,
    pub product_reference: String,
    pub supplier_id: i32,
    pub supplier_name: String,
    pub supplier_email: String,
    pub quantity: i32,
    pub unit_price: Decimal,
    pub total_price: Decimal,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RestockWithSupplierResponse {
    pub id: i32,
    pub quantity: i32,
    pub supplier_id: Option<i32>,
    pub supplier_name: Option<String>,
    pub status: RestockStatus,
    pub restock_date: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub lines: Vec<LineRestockWithSupplierResponse>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RestockStatsResponse {
    pub total_restocks: i64,
    pub total_quantity_restocked: i64,
    pub average_quantity: f64,
    pub product_id: Option<i32>,
    pub last_restock_date: Option<DateTime<Utc>>,
}

// ---- Request DTOs ----
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateLineRestockRequest {
    pub product_id: i32,
    pub quantity: i32,
    pub unit_price: Decimal,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateRestockRequest {
    pub supplier_id: Option<i32>,
    pub lines: Vec<CreateLineRestockRequest>,
    #[serde(default)]
    pub status: Option<RestockStatus>,
    #[serde(default)]
    pub restock_date: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateRestockRequest {
    pub supplier_id: Option<i32>,
    pub status: Option<RestockStatus>,
    pub restock_date: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct RestockSearchParams {
    pub product_id: Option<i32>,
    pub supplier_id: Option<i32>,
    pub from_date: Option<DateTime<Utc>>,
    pub to_date: Option<DateTime<Utc>>,
    pub min_quantity: Option<i32>,
    pub max_quantity: Option<i32>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_restock_status_serialization() {
        let status = RestockStatus::Pending;
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("pending"));

        let status = RestockStatus::InTransit;
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("in_transit"));

        let status = RestockStatus::Received;
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("received"));

        let status = RestockStatus::Cancelled;
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("cancelled"));
    }

    #[test]
    fn test_restock_status_deserialization() {
        let json = r#""pending""#;
        let status: Result<RestockStatus, _> = serde_json::from_str(json);
        assert!(status.is_ok());

        let json = r#""in_transit""#;
        let status: Result<RestockStatus, _> = serde_json::from_str(json);
        assert!(status.is_ok());

        let json = r#""received""#;
        let status: Result<RestockStatus, _> = serde_json::from_str(json);
        assert!(status.is_ok());

        let json = r#""cancelled""#;
        let status: Result<RestockStatus, _> = serde_json::from_str(json);
        assert!(status.is_ok());
    }

    #[test]
    fn test_restock_status_as_str() {
        assert_eq!(RestockStatus::Pending.as_str(), "pending");
        assert_eq!(RestockStatus::InTransit.as_str(), "in_transit");
        assert_eq!(RestockStatus::Received.as_str(), "received");
        assert_eq!(RestockStatus::Cancelled.as_str(), "cancelled");
    }

    #[test]
    fn test_restock_status_from_str() {
        assert!(RestockStatus::from_str("pending").is_some());
        assert!(RestockStatus::from_str("in_transit").is_some());
        assert!(RestockStatus::from_str("received").is_some());
        assert!(RestockStatus::from_str("cancelled").is_some());
        assert!(RestockStatus::from_str("unknown").is_none());
    }

    #[test]
    fn test_restock_response_serialization() {
        let now = chrono::Utc::now();
        let response = RestockResponse {
            id: 1,
            quantity: 50,
            supplier_id: Some(3),
            status: RestockStatus::Pending,
            restock_date: now,
            created_at: now,
            updated_at: now,
            lines: vec![LineRestockResponse {
                product_id: 10,
                product_name: "Widget A".to_string(),
                product_reference: "WA-001".to_string(),
                quantity: 50,
                unit_price: Decimal::from_str("12.50").unwrap(),
                total_price: Decimal::from_str("625.00").unwrap(),
            }],
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"id\":1"));
        assert!(json.contains("\"quantity\":50"));
        assert!(json.contains("\"supplier_id\":3"));
        assert!(json.contains("pending"));
        assert!(json.contains("Widget A"));
        assert!(json.contains("WA-001"));
    }

    #[test]
    fn test_restock_stats_response_serialization() {
        let stats = RestockStatsResponse {
            total_restocks: 100,
            total_quantity_restocked: 5000,
            average_quantity: 50.0,
            product_id: Some(7),
            last_restock_date: Some(chrono::Utc::now()),
        };

        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("\"total_restocks\":100"));
        assert!(json.contains("\"total_quantity_restocked\":5000"));
        assert!(json.contains("\"product_id\":7"));
    }

    #[test]
    fn test_restock_stats_response_without_optionals() {
        let stats = RestockStatsResponse {
            total_restocks: 0,
            total_quantity_restocked: 0,
            average_quantity: 0.0,
            product_id: None,
            last_restock_date: None,
        };

        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("\"product_id\":null"));
        assert!(json.contains("\"last_restock_date\":null"));
    }

    #[test]
    fn test_create_restock_request_deserialization() {
        let json = r#"{
            "supplier_id": 2,
            "lines": [
                {"product_id": 1, "quantity": 10, "unit_price": "5.00"},
                {"product_id": 2, "quantity": 20, "unit_price": "7.50"}
            ],
            "status": "pending"
        }"#;

        let request: Result<CreateRestockRequest, _> = serde_json::from_str(json);
        assert!(request.is_ok());

        let request = request.unwrap();
        assert_eq!(request.supplier_id, Some(2));
        assert_eq!(request.lines.len(), 2);
        assert_eq!(request.lines[0].product_id, 1);
        assert_eq!(request.lines[0].quantity, 10);
        assert_eq!(request.lines[1].product_id, 2);
    }

    #[test]
    fn test_create_restock_request_minimal() {
        let json = r#"{
            "lines": [
                {"product_id": 1, "quantity": 5, "unit_price": "10.00"}
            ]
        }"#;

        let request: Result<CreateRestockRequest, _> = serde_json::from_str(json);
        assert!(request.is_ok());

        let request = request.unwrap();
        assert_eq!(request.supplier_id, None);
        assert_eq!(request.status, None);
        assert_eq!(request.restock_date, None);
        assert_eq!(request.lines.len(), 1);
    }

    #[test]
    fn test_update_restock_request_deserialization() {
        let json = r#"{
            "supplier_id": 5,
            "status": "received"
        }"#;

        let request: Result<UpdateRestockRequest, _> = serde_json::from_str(json);
        assert!(request.is_ok());

        let request = request.unwrap();
        assert_eq!(request.supplier_id, Some(5));
    }

    #[test]
    fn test_update_restock_request_empty() {
        let json = r#"{}"#;

        let request: Result<UpdateRestockRequest, _> = serde_json::from_str(json);
        assert!(request.is_ok());

        let request = request.unwrap();
        assert!(request.supplier_id.is_none());
        assert!(request.status.is_none());
        assert!(request.restock_date.is_none());
    }

    #[test]
    fn test_restock_search_params_deserialization() {
        let json = r#"{
            "product_id": 1,
            "supplier_id": 2,
            "min_quantity": 10,
            "max_quantity": 100,
            "limit": 50,
            "offset": 0
        }"#;

        let params: Result<RestockSearchParams, _> = serde_json::from_str(json);
        assert!(params.is_ok());

        let params = params.unwrap();
        assert_eq!(params.product_id, Some(1));
        assert_eq!(params.supplier_id, Some(2));
        assert_eq!(params.min_quantity, Some(10));
        assert_eq!(params.max_quantity, Some(100));
        assert_eq!(params.limit, Some(50));
        assert_eq!(params.offset, Some(0));
    }

    #[test]
    fn test_restock_search_params_empty() {
        let json = r#"{}"#;

        let params: Result<RestockSearchParams, _> = serde_json::from_str(json);
        assert!(params.is_ok());

        let params = params.unwrap();
        assert_eq!(params.product_id, None);
        assert_eq!(params.supplier_id, None);
        assert_eq!(params.from_date, None);
        assert_eq!(params.to_date, None);
        assert_eq!(params.min_quantity, None);
        assert_eq!(params.max_quantity, None);
        assert_eq!(params.limit, None);
        assert_eq!(params.offset, None);
    }
}
