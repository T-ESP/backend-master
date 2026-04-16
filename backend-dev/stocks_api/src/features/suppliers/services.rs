use sqlx::PgPool;
use crate::common::{responses::{SuccessResponse, ErrorResponse}, error_codes};
use super::dto::{SupplierResponse, CreateSupplierRequest, UpdateSupplierRequest};

pub async fn get_all_suppliers(pool: &PgPool) -> Result<Vec<SupplierResponse>, sqlx::Error> {
    let suppliers = sqlx::query_as!(SupplierResponse,
        "SELECT id_sup as id, name_sup, email_sup, phone_sup, address_sup, created_at, updated_at FROM supplier_sup"
    )
        .fetch_all(pool)
        .await?;

    Ok(suppliers)
}

pub async fn create_supplier(pool: &PgPool, request: CreateSupplierRequest) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO supplier_sup (name_sup, email_sup, phone_sup, address_sup, created_at, updated_at) VALUES ($1, $2, $3, $4, NOW(), NOW())",
        request.name_sup,
        request.email_sup,
        request.phone_sup,
        request.address_sup
    )
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn update_supplier(pool: &PgPool, id: i32, request: UpdateSupplierRequest) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE supplier_sup SET name_sup = COALESCE($1, name_sup), email_sup = COALESCE($2, email_sup), phone_sup = COALESCE($3, phone_sup), address_sup = COALESCE($4, address_sup), updated_at = NOW() WHERE id_sup = $5",
        request.name_sup,
        request.email_sup,
        request.phone_sup,
        request.address_sup,
        id
    )
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn get_supplier_by_id(pool: &PgPool, id: i32) -> Result<Option<SupplierResponse>, sqlx::Error> {
    let supplier = sqlx::query_as!(SupplierResponse,
        "SELECT id_sup as id, name_sup, email_sup, phone_sup, address_sup, created_at, updated_at FROM supplier_sup WHERE id_sup = $1",
        id
    )
        .fetch_optional(pool)
        .await?;

    Ok(supplier)
}

pub async fn get_supplier_by_email(pool: &PgPool, email: &str) -> Result<Option<SupplierResponse>, sqlx::Error> {
    let supplier = sqlx::query_as!(SupplierResponse,
        "SELECT id_sup as id, name_sup, email_sup, phone_sup, address_sup, created_at, updated_at FROM supplier_sup WHERE email_sup = $1",
        email
    )
        .fetch_optional(pool)
        .await?;

    Ok(supplier)
}

pub async fn delete_supplier(pool: &PgPool, id: i32) -> Result<(), sqlx::Error> {
    sqlx::query!("DELETE FROM supplier_sup WHERE id_sup = $1", id)
        .execute(pool)
        .await?;

    Ok(())
}
