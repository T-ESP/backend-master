use sqlx::PgPool;
use uuid::Uuid;
use rand::{distributions::Alphanumeric, Rng};

use crate::common::security;
use super::model::Tenant;
use super::repository;

#[derive(Debug)]
pub enum TenantServiceError {
    SlugAlreadyExists,
    EmailAlreadyExists,
    NotFound,
    Database(sqlx::Error),
    Security(security::SecurityError),
}

impl From<sqlx::Error> for TenantServiceError {
    fn from(value: sqlx::Error) -> Self {
        TenantServiceError::Database(value)
    }
}

impl From<security::SecurityError> for TenantServiceError {
    fn from(value: security::SecurityError) -> Self {
        TenantServiceError::Security(value)
    }
}

pub struct CreatedTenant {
    pub id: Uuid,
    pub email: String,
    pub generated_password: String,
}

pub async fn create_tenant(
    pool: &PgPool,
    name: String,
    slug: String,
    email: String,
    phone: Option<String>,
    address: Option<String>,
    siret: Option<String>,
) -> Result<CreatedTenant, TenantServiceError> {

    let existing_slug = sqlx::query(
        "SELECT id FROM commerces WHERE slug = $1"
    )
        .bind(&slug)
        .fetch_optional(pool)
        .await?;

    if existing_slug.is_some() {
        return Err(TenantServiceError::SlugAlreadyExists);
    }

    let existing_email = sqlx::query(
        "SELECT id FROM commerces WHERE email = $1"
    )
        .bind(&email)
        .fetch_optional(pool)
        .await?;

    if existing_email.is_some() {
        return Err(TenantServiceError::EmailAlreadyExists);
    }

    let generated_password: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(12)
        .map(char::from)
        .collect();

    let hashed_password = security::hash_password(&generated_password)?;

    let id = Uuid::new_v4();
    let db_name = format!("tenant_{}", slug);

    sqlx::query(
        r#"
        INSERT INTO commerces (
            id, name, slug, email, password_hash,
            phone, address, siret, db_name
        )
        VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)
        "#
    )
        .bind(id)
        .bind(&name)
        .bind(&slug)
        .bind(&email)
        .bind(&hashed_password)
        .bind(&phone)
        .bind(&address)
        .bind(&siret)
        .bind(&db_name)
        .execute(pool)
        .await?;

    Ok(CreatedTenant {
        id,
        email,
        generated_password,
    })
}

pub async fn get_all_tenants(pool: &PgPool) -> Result<Vec<Tenant>, TenantServiceError> {
    let tenants = repository::find_all(pool).await?;
    Ok(tenants)
}

pub async fn get_tenant_by_id(pool: &PgPool, id: Uuid) -> Result<Tenant, TenantServiceError> {
    let tenant = repository::find_by_id(pool, id).await?;
    tenant.ok_or(TenantServiceError::NotFound)
}

pub async fn get_tenant_by_slug(pool: &PgPool, slug: &str) -> Result<Tenant, TenantServiceError> {
    let tenant = repository::find_by_slug(pool, slug).await?;
    tenant.ok_or(TenantServiceError::NotFound)
}

pub async fn delete_tenant(pool: &PgPool, id: Uuid) -> Result<(), TenantServiceError> {
    let deleted = repository::delete(pool, id).await?;
    if !deleted {
        return Err(TenantServiceError::NotFound);
    }
    Ok(())
}
