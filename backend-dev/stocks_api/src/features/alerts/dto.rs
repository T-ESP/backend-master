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
