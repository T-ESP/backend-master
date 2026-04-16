use sqlx::{PgPool, Row};
use rust_decimal::prelude::ToPrimitive;
use super::dto::{SupplierResponse, SupplierProfileResponse, SupplierProductItem, SupplierScoreInfo, CreateSupplierRequest, UpdateSupplierRequest};

fn map_supplier(row: &sqlx::postgres::PgRow) -> SupplierResponse {
    SupplierResponse {
        id: row.get("id_sup"),
        name_sup: row.get("name_sup"),
        email_sup: row.get("email_sup"),
        phone_sup: row.get("phone_sup"),
        address_sup: row.get("address_sup"),
        created_at: row.get::<Option<chrono::DateTime<chrono::Utc>>, _>("created_at")
            .map(|dt| dt.naive_utc()),
        updated_at: row.get::<Option<chrono::DateTime<chrono::Utc>>, _>("updated_at")
            .map(|dt| dt.naive_utc()),
    }
}

pub async fn get_all_suppliers(pool: &PgPool) -> Result<Vec<SupplierResponse>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id_sup, name_sup, email_sup, phone_sup, address_sup, created_at, updated_at FROM supplier_sup"
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.iter().map(map_supplier).collect())
}

pub async fn create_supplier(pool: &PgPool, request: CreateSupplierRequest) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO supplier_sup (name_sup, email_sup, phone_sup, address_sup, created_at, updated_at) VALUES ($1, $2, $3, $4, NOW(), NOW())"
    )
    .bind(&request.name_sup)
    .bind(&request.email_sup)
    .bind(&request.phone_sup)
    .bind(&request.address_sup)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn update_supplier(pool: &PgPool, id: i32, request: UpdateSupplierRequest) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE supplier_sup SET name_sup = COALESCE($1, name_sup), email_sup = COALESCE($2, email_sup), phone_sup = COALESCE($3, phone_sup), address_sup = COALESCE($4, address_sup), updated_at = NOW() WHERE id_sup = $5"
    )
    .bind(&request.name_sup)
    .bind(&request.email_sup)
    .bind(&request.phone_sup)
    .bind(&request.address_sup)
    .bind(id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_supplier_by_id(pool: &PgPool, id: i32) -> Result<Option<SupplierResponse>, sqlx::Error> {
    let row = sqlx::query(
        "SELECT id_sup, name_sup, email_sup, phone_sup, address_sup, created_at, updated_at FROM supplier_sup WHERE id_sup = $1"
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(row.as_ref().map(map_supplier))
}

pub async fn get_supplier_by_email(pool: &PgPool, email: &str) -> Result<Option<SupplierResponse>, sqlx::Error> {
    let row = sqlx::query(
        "SELECT id_sup, name_sup, email_sup, phone_sup, address_sup, created_at, updated_at FROM supplier_sup WHERE email_sup = $1"
    )
    .bind(email)
    .fetch_optional(pool)
    .await?;

    Ok(row.as_ref().map(map_supplier))
}

pub async fn get_supplier_profile(pool: &PgPool, id: i32) -> Result<Option<SupplierProfileResponse>, sqlx::Error> {
    let sup_row = sqlx::query(
        "SELECT id_sup, name_sup, email_sup, phone_sup, address_sup, created_at, updated_at
         FROM supplier_sup WHERE id_sup = $1"
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    let sup_row = match sup_row {
        Some(r) => r,
        None => return Ok(None),
    };

    let product_rows = sqlx::query(
        "SELECT id_pro, name_pro, category_pro, reference_pro, stock_quantity_pro,
                buying_price_pro, status_pro::TEXT as status_text
         FROM products_pro WHERE supplier_id_pro = $1
         ORDER BY name_pro ASC"
    )
    .bind(id)
    .fetch_all(pool)
    .await?;

    let products: Vec<SupplierProductItem> = product_rows.iter().map(|r| SupplierProductItem {
        id: r.get("id_pro"),
        name: r.get("name_pro"),
        category: r.get("category_pro"),
        reference: r.get("reference_pro"),
        stock_quantity: r.get("stock_quantity_pro"),
        buying_price: r.get::<rust_decimal::Decimal, _>("buying_price_pro").to_f64().unwrap_or(0.0),
        status: r.get("status_text"),
    }).collect();

    let product_count = products.len() as i64;

    let score_row = sqlx::query(
        "SELECT overall_score, delivery_score, quality_score, lead_time_score,
                fulfillment_score, rating, total_restocks
         FROM supplier_scores WHERE supplier_id = $1
         ORDER BY created_at DESC LIMIT 1"
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    let score = score_row.map(|r| SupplierScoreInfo {
        overall_score: r.get::<rust_decimal::Decimal, _>("overall_score").to_f64().unwrap_or(0.0),
        delivery_score: r.get::<Option<rust_decimal::Decimal>, _>("delivery_score").and_then(|v| v.to_f64()),
        quality_score: r.get::<Option<rust_decimal::Decimal>, _>("quality_score").and_then(|v| v.to_f64()),
        lead_time_score: r.get::<Option<rust_decimal::Decimal>, _>("lead_time_score").and_then(|v| v.to_f64()),
        fulfillment_score: r.get::<Option<rust_decimal::Decimal>, _>("fulfillment_score").and_then(|v| v.to_f64()),
        rating: r.get("rating"),
        total_restocks: r.get("total_restocks"),
    });

    Ok(Some(SupplierProfileResponse {
        id: sup_row.get("id_sup"),
        name: sup_row.get("name_sup"),
        email: sup_row.get("email_sup"),
        phone: sup_row.get("phone_sup"),
        address: sup_row.get("address_sup"),
        product_count,
        products,
        score,
        created_at: sup_row.get::<Option<chrono::DateTime<chrono::Utc>>, _>("created_at").map(|dt| dt.naive_utc()),
        updated_at: sup_row.get::<Option<chrono::DateTime<chrono::Utc>>, _>("updated_at").map(|dt| dt.naive_utc()),
    }))
}

pub async fn delete_supplier(pool: &PgPool, id: i32) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM supplier_sup WHERE id_sup = $1")
        .bind(id)
        .execute(pool)
        .await?;

    Ok(())
}
