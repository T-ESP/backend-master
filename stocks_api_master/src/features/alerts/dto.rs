use serde::{Deserialize, Serialize};
use utoipa::{ToSchema, IntoParams};

#[derive(Debug, Serialize, ToSchema)]
pub struct AlertResponse {
    pub id: i32,
    pub product_id: Option<i32>,
    pub product_name: Option<String>,
    pub model_type: String,
    pub category: String,
    pub notification_type: String,
    pub severity: String,
    pub message: String,
    pub action_recommended: Option<String>,
    pub related_result_id: Option<i32>,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct StatusCounts {
    pub new: i64,
    pub acknowledged: i64,
    pub in_progress: i64,
    pub resolved: i64,
    pub dismissed: i64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SeverityCounts {
    pub critical: i64,
    pub high: i64,
    pub medium: i64,
    pub low: i64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ModelTypeCounts {
    pub model_type: String,
    pub count: i64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AlertSummary {
    pub total: i64,
    pub by_status: StatusCounts,
    pub by_severity: SeverityCounts,
    pub by_model_type: Vec<ModelTypeCounts>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateStatusRequest {
    pub status: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct BulkUpdateStatusRequest {
    pub ids: Vec<i32>,
    pub status: String,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct AlertFilters {
    pub status: Option<String>,
    pub severity: Option<String>,
    pub model_type: Option<String>,
    pub product_id: Option<i32>,
    pub category: Option<String>,
    pub from_date: Option<chrono::NaiveDate>,
    pub to_date: Option<chrono::NaiveDate>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct CleanupParams {
    pub older_than_days: Option<i32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alert_response_serialization() {
        let now = chrono::Utc::now();
        let alert = AlertResponse {
            id: 1,
            product_id: Some(42),
            product_name: Some("Widget".to_string()),
            model_type: "demand_forecast".to_string(),
            category: "stock".to_string(),
            notification_type: "restock_needed".to_string(),
            severity: "high".to_string(),
            message: "Stock running low".to_string(),
            action_recommended: Some("Order 100 units".to_string()),
            related_result_id: Some(10),
            status: "new".to_string(),
            created_at: now,
            updated_at: Some(now),
        };

        let json = serde_json::to_string(&alert).unwrap();
        assert!(json.contains("\"id\":1"));
        assert!(json.contains("\"product_id\":42"));
        assert!(json.contains("Widget"));
        assert!(json.contains("demand_forecast"));
        assert!(json.contains("high"));
        assert!(json.contains("Stock running low"));
        assert!(json.contains("Order 100 units"));
    }

    #[test]
    fn test_alert_response_without_optionals() {
        let now = chrono::Utc::now();
        let alert = AlertResponse {
            id: 2,
            product_id: None,
            product_name: None,
            model_type: "clustering".to_string(),
            category: "performance".to_string(),
            notification_type: "info".to_string(),
            severity: "low".to_string(),
            message: "Cluster updated".to_string(),
            action_recommended: None,
            related_result_id: None,
            status: "new".to_string(),
            created_at: now,
            updated_at: None,
        };

        let json = serde_json::to_string(&alert).unwrap();
        assert!(json.contains("\"product_id\":null"));
        assert!(json.contains("\"product_name\":null"));
        assert!(json.contains("\"action_recommended\":null"));
        assert!(json.contains("\"related_result_id\":null"));
        assert!(json.contains("\"updated_at\":null"));
    }

    #[test]
    fn test_alert_summary_serialization() {
        let summary = AlertSummary {
            total: 50,
            by_status: StatusCounts {
                new: 20,
                acknowledged: 10,
                in_progress: 5,
                resolved: 10,
                dismissed: 5,
            },
            by_severity: SeverityCounts {
                critical: 5,
                high: 15,
                medium: 20,
                low: 10,
            },
            by_model_type: vec![
                ModelTypeCounts {
                    model_type: "demand_forecast".to_string(),
                    count: 30,
                },
                ModelTypeCounts {
                    model_type: "clustering".to_string(),
                    count: 20,
                },
            ],
        };

        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("\"total\":50"));
        assert!(json.contains("\"new\":20"));
        assert!(json.contains("\"acknowledged\":10"));
        assert!(json.contains("\"critical\":5"));
        assert!(json.contains("\"high\":15"));
        assert!(json.contains("demand_forecast"));
        assert!(json.contains("clustering"));
    }

    #[test]
    fn test_update_status_request_deserialization() {
        let json = r#"{"status": "acknowledged"}"#;
        let request: Result<UpdateStatusRequest, _> = serde_json::from_str(json);
        assert!(request.is_ok());
        assert_eq!(request.unwrap().status, "acknowledged");
    }

    #[test]
    fn test_bulk_update_status_request_deserialization() {
        let json = r#"{"ids": [1, 2, 3, 4], "status": "resolved"}"#;
        let request: Result<BulkUpdateStatusRequest, _> = serde_json::from_str(json);
        assert!(request.is_ok());

        let request = request.unwrap();
        assert_eq!(request.ids, vec![1, 2, 3, 4]);
        assert_eq!(request.status, "resolved");
    }

    #[test]
    fn test_alert_filters_deserialization() {
        let json = r#"{
            "status": "new",
            "severity": "high",
            "model_type": "demand_forecast",
            "product_id": 5,
            "category": "stock",
            "limit": 20,
            "offset": 0
        }"#;

        let filters: Result<AlertFilters, _> = serde_json::from_str(json);
        assert!(filters.is_ok());

        let filters = filters.unwrap();
        assert_eq!(filters.status, Some("new".to_string()));
        assert_eq!(filters.severity, Some("high".to_string()));
        assert_eq!(filters.model_type, Some("demand_forecast".to_string()));
        assert_eq!(filters.product_id, Some(5));
        assert_eq!(filters.category, Some("stock".to_string()));
        assert_eq!(filters.limit, Some(20));
        assert_eq!(filters.offset, Some(0));
    }

    #[test]
    fn test_alert_filters_empty() {
        let json = r#"{}"#;

        let filters: Result<AlertFilters, _> = serde_json::from_str(json);
        assert!(filters.is_ok());

        let filters = filters.unwrap();
        assert_eq!(filters.status, None);
        assert_eq!(filters.severity, None);
        assert_eq!(filters.model_type, None);
        assert_eq!(filters.product_id, None);
        assert_eq!(filters.category, None);
        assert_eq!(filters.from_date, None);
        assert_eq!(filters.to_date, None);
        assert_eq!(filters.limit, None);
        assert_eq!(filters.offset, None);
    }

    #[test]
    fn test_cleanup_params_deserialization() {
        let json = r#"{"older_than_days": 30}"#;
        let params: Result<CleanupParams, _> = serde_json::from_str(json);
        assert!(params.is_ok());
        assert_eq!(params.unwrap().older_than_days, Some(30));
    }

    #[test]
    fn test_cleanup_params_empty() {
        let json = r#"{}"#;
        let params: Result<CleanupParams, _> = serde_json::from_str(json);
        assert!(params.is_ok());
        assert_eq!(params.unwrap().older_than_days, None);
    }
}
