use serde::{Deserialize, Serialize};
use sqlx::Row;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use utoipa::{ToSchema, IntoParams};

#[derive(Debug, Serialize, ToSchema)]
pub struct OrderResponse {
    pub id: i32,
    pub user_id: i32,
    pub order_date: DateTime<Utc>,
    pub status: String,
    pub amount: Decimal,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl OrderResponse {
    pub fn from_row(row: &sqlx::postgres::PgRow) -> Self {
        Self {
            id: row.get("id_ord"),
            user_id: row.get("user_id_ord"),
            order_date: row.get("order_date_ord"),
            status: row.get("status_ord"),
            amount: row.get("amount_ord"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateOrderRequest {
    pub user_id: i32,
    pub status: String,
    pub line_items: Vec<CreateLineItemRequest>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateLineItemRequest {
    pub product_id: i32,
    pub quantity: i32,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LineItemResponse {
    pub id: i32,
    pub order_id: i32,
    pub product_id: i32,
    pub quantity: i32,
    pub line_total: Decimal,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl LineItemResponse {
    pub fn from_row(row: &sqlx::postgres::PgRow) -> Self {
        Self {
            id: row.get("id_lor"),
            order_id: row.get("order_id_lor"),
            product_id: row.get("product_id_lor"),
            quantity: row.get("quantity_lor"),
            line_total: row.get("line_total_lor"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct OrderWithItemsResponse {
    pub id: i32,
    pub user_id: i32,
    pub order_date: DateTime<Utc>,
    pub status: String,
    pub amount: Decimal,
    pub line_items: Vec<LineItemResponse>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct OrderQueryParams {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub user_id: Option<i32>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateOrderRequest {
    pub status: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct OrderStatsResponse {
    pub total_orders: i64,
    pub pending_orders: i64,
    pub confirmed_orders: i64,
    pub shipped_orders: i64,
    pub delivered_orders: i64,
    pub cancelled_orders: i64,
    pub total_amount: Decimal,
    pub avg_order_value: Decimal,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    #[test]
    fn test_create_order_request_deserialization() {
        let json = r#"{
            "user_id": 1,
            "status": "pending",
            "line_items": [
                {
                    "product_id": 10,
                    "quantity": 5
                },
                {
                    "product_id": 20,
                    "quantity": 3
                }
            ]
        }"#;

        let request: Result<CreateOrderRequest, _> = serde_json::from_str(json);
        assert!(request.is_ok());

        let request = request.unwrap();
        assert_eq!(request.user_id, 1);
        assert_eq!(request.status, "pending");
        assert_eq!(request.line_items.len(), 2);
        assert_eq!(request.line_items[0].product_id, 10);
        assert_eq!(request.line_items[0].quantity, 5);
        assert_eq!(request.line_items[1].product_id, 20);
        assert_eq!(request.line_items[1].quantity, 3);
    }

    #[test]
    fn test_create_order_request_empty_line_items() {
        let json = r#"{
            "user_id": 1,
            "status": "pending",
            "line_items": []
        }"#;

        let request: Result<CreateOrderRequest, _> = serde_json::from_str(json);
        assert!(request.is_ok());

        let request = request.unwrap();
        assert_eq!(request.line_items.len(), 0);
    }

    #[test]
    fn test_create_line_item_request_deserialization() {
        let json = r#"{
            "product_id": 42,
            "quantity": 10
        }"#;

        let line_item: Result<CreateLineItemRequest, _> = serde_json::from_str(json);
        assert!(line_item.is_ok());

        let line_item = line_item.unwrap();
        assert_eq!(line_item.product_id, 42);
        assert_eq!(line_item.quantity, 10);
    }

    #[test]
    fn test_create_line_item_request_negative_quantity() {
        let json = r#"{
            "product_id": 42,
            "quantity": -5
        }"#;

        let line_item: Result<CreateLineItemRequest, _> = serde_json::from_str(json);
        assert!(line_item.is_ok());
        assert_eq!(line_item.unwrap().quantity, -5);
    }

    #[test]
    fn test_update_order_request_deserialization() {
        let statuses = vec!["pending", "confirmed", "shipped", "delivered", "cancelled"];

        for status in statuses {
            let json = format!(r#"{{"status": "{}"}}"#, status);
            let request: Result<UpdateOrderRequest, _> = serde_json::from_str(&json);
            assert!(request.is_ok());
            assert_eq!(request.unwrap().status, status);
        }
    }

    #[test]
    fn test_order_response_serialization() {
        let order = OrderResponse {
            id: 1,
            user_id: 123,
            order_date: Utc::now(),
            status: "confirmed".to_string(),
            amount: Decimal::from_str("299.99").unwrap(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let json = serde_json::to_string(&order);
        assert!(json.is_ok());

        let json_str = json.unwrap();
        assert!(json_str.contains("confirmed"));
        assert!(json_str.contains("299.99"));
    }

    #[test]
    fn test_line_item_response_serialization() {
        let line_item = LineItemResponse {
            id: 1,
            order_id: 10,
            product_id: 50,
            quantity: 3,
            line_total: Decimal::from_str("149.97").unwrap(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let json = serde_json::to_string(&line_item);
        assert!(json.is_ok());

        let json_str = json.unwrap();
        assert!(json_str.contains("\"order_id\":10"));
        assert!(json_str.contains("\"product_id\":50"));
        assert!(json_str.contains("\"quantity\":3"));
    }

    #[test]
    fn test_order_with_items_response_serialization() {
        let order = OrderWithItemsResponse {
            id: 1,
            user_id: 123,
            order_date: Utc::now(),
            status: "shipped".to_string(),
            amount: Decimal::from_str("599.98").unwrap(),
            line_items: vec![
                LineItemResponse {
                    id: 1,
                    order_id: 1,
                    product_id: 10,
                    quantity: 2,
                    line_total: Decimal::from_str("299.98").unwrap(),
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                },
                LineItemResponse {
                    id: 2,
                    order_id: 1,
                    product_id: 20,
                    quantity: 3,
                    line_total: Decimal::from_str("300.00").unwrap(),
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                },
            ],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let json = serde_json::to_string(&order);
        assert!(json.is_ok());

        let json_str = json.unwrap();
        assert!(json_str.contains("shipped"));
        assert!(json_str.contains("line_items"));
        assert!(json_str.contains("599.98"));
    }

    #[test]
    fn test_order_query_params_deserialization() {
        let json = r#"{
            "limit": 20,
            "offset": 10,
            "user_id": 5,
            "status": "delivered"
        }"#;

        let params: Result<OrderQueryParams, _> = serde_json::from_str(json);
        assert!(params.is_ok());

        let params = params.unwrap();
        assert_eq!(params.limit, Some(20));
        assert_eq!(params.offset, Some(10));
        assert_eq!(params.user_id, Some(5));
        assert_eq!(params.status, Some("delivered".to_string()));
    }

    #[test]
    fn test_order_query_params_optional_fields() {
        let json = r#"{}"#;

        let params: Result<OrderQueryParams, _> = serde_json::from_str(json);
        assert!(params.is_ok());

        let params = params.unwrap();
        assert_eq!(params.limit, None);
        assert_eq!(params.offset, None);
        assert_eq!(params.user_id, None);
        assert_eq!(params.status, None);
    }

    #[test]
    fn test_order_query_params_partial_filters() {
        let json = r#"{
            "limit": 50,
            "status": "pending"
        }"#;

        let params: Result<OrderQueryParams, _> = serde_json::from_str(json);
        assert!(params.is_ok());

        let params = params.unwrap();
        assert_eq!(params.limit, Some(50));
        assert_eq!(params.offset, None);
        assert_eq!(params.user_id, None);
        assert_eq!(params.status, Some("pending".to_string()));
    }

    #[test]
    fn test_order_stats_response_serialization() {
        let stats = OrderStatsResponse {
            total_orders: 150,
            pending_orders: 20,
            confirmed_orders: 50,
            shipped_orders: 40,
            delivered_orders: 35,
            cancelled_orders: 5,
            total_amount: Decimal::from_str("45000.00").unwrap(),
            avg_order_value: Decimal::from_str("300.00").unwrap(),
        };

        let json = serde_json::to_string(&stats);
        assert!(json.is_ok());

        let json_str = json.unwrap();
        assert!(json_str.contains("\"total_orders\":150"));
        assert!(json_str.contains("\"pending_orders\":20"));
        assert!(json_str.contains("\"confirmed_orders\":50"));
        assert!(json_str.contains("\"shipped_orders\":40"));
        assert!(json_str.contains("\"delivered_orders\":35"));
        assert!(json_str.contains("\"cancelled_orders\":5"));
    }

    #[test]
    fn test_order_stats_response_zero_values() {
        let stats = OrderStatsResponse {
            total_orders: 0,
            pending_orders: 0,
            confirmed_orders: 0,
            shipped_orders: 0,
            delivered_orders: 0,
            cancelled_orders: 0,
            total_amount: Decimal::ZERO,
            avg_order_value: Decimal::ZERO,
        };

        let json = serde_json::to_string(&stats);
        assert!(json.is_ok());

        let json_str = json.unwrap();
        assert!(json_str.contains("\"total_orders\":0"));
        assert!(json_str.contains("\"total_amount\":\"0\""));
    }

    #[test]
    fn test_order_status_values() {
        let statuses = vec!["pending", "confirmed", "shipped", "delivered", "cancelled"];

        for status in statuses {
            let order = OrderResponse {
                id: 1,
                user_id: 1,
                order_date: Utc::now(),
                status: status.to_string(),
                amount: Decimal::from_str("100.00").unwrap(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            };

            assert_eq!(order.status, status);
        }
    }

    #[test]
    fn test_decimal_amounts_precision() {
        let amounts = vec!["0.01", "99.99", "1234.56", "10000.00"];

        for amount_str in amounts {
            let order = OrderResponse {
                id: 1,
                user_id: 1,
                order_date: Utc::now(),
                status: "pending".to_string(),
                amount: Decimal::from_str(amount_str).unwrap(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            };

            let json = serde_json::to_string(&order).unwrap();
            assert!(json.contains(amount_str));
        }
    }

    #[test]
    fn test_multiple_line_items_in_order() {
        let json = r#"{
            "user_id": 1,
            "status": "pending",
            "line_items": [
                {"product_id": 1, "quantity": 1},
                {"product_id": 2, "quantity": 2},
                {"product_id": 3, "quantity": 3},
                {"product_id": 4, "quantity": 4},
                {"product_id": 5, "quantity": 5}
            ]
        }"#;

        let request: Result<CreateOrderRequest, _> = serde_json::from_str(json);
        assert!(request.is_ok());

        let request = request.unwrap();
        assert_eq!(request.line_items.len(), 5);

        for (i, item) in request.line_items.iter().enumerate() {
            assert_eq!(item.product_id, (i + 1) as i32);
            assert_eq!(item.quantity, (i + 1) as i32);
        }
    }
}