use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct LoyaltyConfigResponse {
    pub id: i32,
    pub euros_per_point: Decimal,
    pub points_required: i32,
    pub discount_percent: Decimal,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateLoyaltyConfigRequest {
    pub euros_per_point: Option<Decimal>,
    pub points_required: Option<i32>,
    pub discount_percent: Option<Decimal>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UserLoyaltyResponse {
    pub user_id: i32,
    pub total_points: i64,
    pub transactions: Vec<LoyaltyTransactionResponse>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LoyaltyTransactionResponse {
    pub id: i32,
    pub order_id: Option<i32>,
    pub points: i32,
    pub reason: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct AdjustPointsRequest {
    pub points: i32,
    pub reason: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AdjustPointsResponse {
    pub user_id: i32,
    pub adjustment: i32,
    pub reason: Option<String>,
    pub new_total_points: i64,
    pub transaction: LoyaltyTransactionResponse,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UserDiscountResponse {
    pub user_id: i32,
    pub total_points: i64,
    pub points_required: i32,
    pub discount_percent: Decimal,
    pub available_discount_percent: Decimal,
    pub points_used: i64,
}
