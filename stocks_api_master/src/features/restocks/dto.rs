use serde::{Deserialize, Serialize};
use utoipa::{ToSchema, IntoParams};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

// ---- Enums ----
#[derive(Debug, Serialize, Deserialize, ToSchema, Clone)]
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

