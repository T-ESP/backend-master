use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use utoipa::ToSchema;

// ── Enums ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, sqlx::Type)]
#[sqlx(type_name = "discount_trigger_enum")]
#[serde(rename_all = "snake_case")]
pub enum DiscountTrigger {
    #[sqlx(rename = "product")]
    Product,
    #[sqlx(rename = "total_amount")]
    TotalAmount,
    #[sqlx(rename = "quantity")]
    Quantity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, sqlx::Type)]
#[sqlx(type_name = "discount_action_enum")]
#[serde(rename_all = "snake_case")]
pub enum DiscountAction {
    #[sqlx(rename = "fixed_eur")]
    FixedEur,
    #[sqlx(rename = "percentage")]
    Percentage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, sqlx::Type)]
#[sqlx(type_name = "discount_scope_enum")]
#[serde(rename_all = "snake_case")]
pub enum DiscountScope {
    #[sqlx(rename = "per_product")]
    PerProduct,
    #[sqlx(rename = "global")]
    Global,
}

// ── Response ──────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, ToSchema)]
pub struct DiscountResponse {
    pub id: i32,
    pub name: String,
    pub trigger_type: DiscountTrigger,
    pub trigger_product_id: Option<i32>,
    pub trigger_min_amount: Option<Decimal>,
    pub trigger_min_qty: Option<i32>,
    pub action_type: DiscountAction,
    pub action_value: Decimal,
    pub scope: DiscountScope,
    pub scope_product_id: Option<i32>,
    pub cumulative: bool,
    pub valid_from: Option<DateTime<Utc>>,
    pub valid_until: Option<DateTime<Utc>>,
    pub is_active: bool,
    pub usage_count: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl DiscountResponse {
    pub fn from_row(row: &sqlx::postgres::PgRow) -> Self {
        Self {
            id: row.get("id_dis"),
            name: row.get("name_dis"),
            trigger_type: row.get("trigger_type_dis"),
            trigger_product_id: row.get("trigger_product_id_dis"),
            trigger_min_amount: row.get("trigger_min_amount_dis"),
            trigger_min_qty: row.get("trigger_min_qty_dis"),
            action_type: row.get("action_type_dis"),
            action_value: row.get("action_value_dis"),
            scope: row.get("scope_dis"),
            scope_product_id: row.get("scope_product_id_dis"),
            cumulative: row.get("cumulative_dis"),
            valid_from: row.get("valid_from_dis"),
            valid_until: row.get("valid_until_dis"),
            is_active: row.get("is_active_dis"),
            usage_count: row.try_get("usage_count").unwrap_or(0),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DiscountOrderSummary {
    pub order_id: i32,
    pub order_date: DateTime<Utc>,
    pub saving_amount: Decimal,
    pub order_total: Decimal,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct OrderDiscountSummary {
    pub discount_id: i32,
    pub discount_name: String,
    pub action_type: DiscountAction,
    pub action_value: Decimal,
    pub saving_amount: Decimal,
}

// ── Create / Update requests ──────────────────────────────────────────────────

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateDiscountRequest {
    pub name: String,
    pub trigger_type: DiscountTrigger,
    pub trigger_product_id: Option<i32>,
    pub trigger_min_amount: Option<Decimal>,
    pub trigger_min_qty: Option<i32>,
    pub action_type: DiscountAction,
    pub action_value: Decimal,
    pub scope: DiscountScope,
    pub scope_product_id: Option<i32>,
    pub cumulative: Option<bool>,
    pub valid_from: Option<DateTime<Utc>>,
    pub valid_until: Option<DateTime<Utc>>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateDiscountRequest {
    pub name: Option<String>,
    pub action_value: Option<Decimal>,
    pub cumulative: Option<bool>,
    pub valid_from: Option<DateTime<Utc>>,
    pub valid_until: Option<DateTime<Utc>>,
    pub is_active: Option<bool>,
}

// ── Check endpoint ────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, ToSchema, Clone)]
pub struct CheckLineItem {
    pub product_id: i32,
    pub quantity: i32,
    pub line_total: Decimal,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CheckDiscountRequest {
    pub line_items: Vec<CheckLineItem>,
    pub total_amount: Decimal,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ApplicableDiscount {
    pub id: i32,
    pub name: String,
    pub trigger_type: DiscountTrigger,
    pub action_type: DiscountAction,
    pub action_value: Decimal,
    pub scope: DiscountScope,
    pub scope_product_id: Option<i32>,
    pub saving_amount: Decimal,
    pub cumulative: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CheckDiscountResponse {
    pub applicable_discounts: Vec<ApplicableDiscount>,
    pub total_before: Decimal,
    pub total_after: Decimal,
    pub total_saving: Decimal,
}

// ── Applied discount summary (stored in order_discounts_odc) ─────────────────

#[derive(Debug, Serialize, ToSchema)]
pub struct AppliedDiscountSummary {
    pub id: i32,
    pub name: String,
    pub action_type: DiscountAction,
    pub action_value: Decimal,
    pub saving_amount: Decimal,
}
