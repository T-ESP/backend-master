use sqlx::{PgPool, postgres::PgPoolOptions};
use uuid::Uuid;
use rand::{distributions::Alphanumeric, Rng};

use crate::common::security;
use super::model::Tenant;
use super::repository;

const TENANT_SCHEMA: &str = include_str!("../../../sql/tenant_schema.sql");

#[derive(Debug)]
pub enum TenantServiceError {
    SlugAlreadyExists,
    EmailAlreadyExists,
    NotFound,
    Database(sqlx::Error),
    Security(security::SecurityError),
    DatabaseProvisioning(String),
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

fn parse_base_url(database_url: &str) -> Result<String, TenantServiceError> {
    database_url
        .rfind('/')
        .map(|i| database_url[..i].to_string())
        .ok_or_else(|| TenantServiceError::DatabaseProvisioning(
            "Invalid DATABASE_URL format".to_string()
        ))
}

pub async fn create_tenant(
    pool: &PgPool,
    database_url: &str,
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

    let base_url = parse_base_url(database_url)?;

    // Connect to the 'postgres' maintenance DB to run CREATE DATABASE
    let pg_url = format!("{}/postgres", base_url);
    let pg_pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&pg_url)
        .await
        .map_err(|e| TenantServiceError::DatabaseProvisioning(
            format!("Failed to connect to postgres maintenance DB: {}", e)
        ))?;

    sqlx::raw_sql(&format!("CREATE DATABASE \"{}\"", db_name))
        .execute(&pg_pool)
        .await
        .map_err(|e| TenantServiceError::DatabaseProvisioning(
            format!("Failed to create database '{}': {}", db_name, e)
        ))?;

    pg_pool.close().await;

    // Connect to the new tenant database and apply schema
    let tenant_url = format!("{}/{}", base_url, db_name);

    let tenant_pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&tenant_url)
        .await
        .map_err(|e| TenantServiceError::DatabaseProvisioning(
            format!("Failed to connect to new database '{}': {}", db_name, e)
        ))?;

    sqlx::raw_sql(TENANT_SCHEMA)
        .execute(&tenant_pool)
        .await
        .map_err(|e| TenantServiceError::DatabaseProvisioning(
            format!("Failed to apply schema to '{}': {}", db_name, e)
        ))?;

    tenant_pool.close().await;

    // Insert into master DB only after the tenant DB is fully ready
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

pub async fn seed_tenant(
    pool: &PgPool,
    database_url: &str,
    tenant_id: Uuid,
) -> Result<(), TenantServiceError> {
    let tenant = repository::find_by_id(pool, tenant_id)
        .await?
        .ok_or(TenantServiceError::NotFound)?;

    let base_url = parse_base_url(database_url)?;
    let tenant_url = format!("{}/{}", base_url, tenant.db_name);

    let seed_binary = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("seed")))
        .unwrap_or_else(|| std::path::PathBuf::from("/app/seed"));

    let seed_result = tokio::process::Command::new(&seed_binary)
        .env("DATABASE_URL", &tenant_url)
        .env("SEED_RESET", "0")
        .output()
        .await
        .map_err(|e| TenantServiceError::DatabaseProvisioning(
            format!("Failed to launch seed: {}", e)
        ))?;

    if !seed_result.status.success() {
        let stderr = String::from_utf8_lossy(&seed_result.stderr);
        return Err(TenantServiceError::DatabaseProvisioning(
            format!("Seed failed: {}", stderr)
        ));
    }

    Ok(())
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

pub async fn update_tenant(
    pool: &PgPool,
    id: Uuid,
    name: Option<String>,
    email: Option<String>,
    phone: Option<String>,
    address: Option<String>,
    siret: Option<String>,
    status: Option<String>,
) -> Result<Tenant, TenantServiceError> {
    let tenant = repository::update(pool, id, name, email, phone, address, siret, status).await?;
    tenant.ok_or(TenantServiceError::NotFound)
}

pub async fn delete_tenant(pool: &PgPool, id: Uuid) -> Result<(), TenantServiceError> {
    let deleted = repository::delete(pool, id).await?;
    if !deleted {
        return Err(TenantServiceError::NotFound);
    }
    Ok(())
}
