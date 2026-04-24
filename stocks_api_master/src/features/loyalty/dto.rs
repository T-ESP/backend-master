use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct LoyaltyConfigResponse {
    pub id: i32,
    pub euros_per_point: Decimal,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateLoyaltyConfigRequest {
    pub euros_per_point: Decimal,
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
    pub order_id: i32,
    pub points: i32,
    pub created_at: DateTime<Utc>,
}
