use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::common::security::{self, SecurityError};

#[derive(Debug)]
pub struct AuthenticatedUser {
    pub id: String,
    pub email: String,
    pub role: String,
}

#[derive(Debug)]
pub enum AuthServiceError {
    InvalidCredentials,
    EmailAlreadyExists,
    Database(sqlx::Error),
    Security(SecurityError),
}

impl From<sqlx::Error> for AuthServiceError {
    fn from(value: sqlx::Error) -> Self {
        AuthServiceError::Database(value)
    }
}

impl From<SecurityError> for AuthServiceError {
    fn from(value: SecurityError) -> Self {
        AuthServiceError::Security(value)
    }
}

pub async fn authenticate_user(
    pool: &PgPool,
    email: &str,
    password: &str,
) -> Result<AuthenticatedUser, AuthServiceError> {

    // 1️⃣ Vérifie platform_admin
    if let Some(row) = sqlx::query(
        r#"
        SELECT id, email, password_hash
        FROM platform_admins
        WHERE email = $1
        "#,
    )
        .bind(email)
        .fetch_optional(pool)
        .await?
    {
        let id: i32 = row.get("id");
        let email_db: String = row.get("email");
        let password_hash: String = row.get("password_hash");

        if security::verify_password(password, &password_hash)? {
            return Ok(AuthenticatedUser {
                id: id.to_string(),
                email: email_db,
                role: "platform_admin".to_string(),
            });
        } else {
            return Err(AuthServiceError::InvalidCredentials);
        }
    }

    // 2️⃣ Vérifie commerce
    if let Some(row) = sqlx::query(
        r#"
        SELECT id, email, password_hash
        FROM commerces
        WHERE email = $1
        "#,
    )
        .bind(email)
        .fetch_optional(pool)
        .await?
    {
        let id: Uuid = row.get("id");
        let email_db: String = row.get("email");
        let password_hash: String = row.get("password_hash");

        if security::verify_password(password, &password_hash)? {
            return Ok(AuthenticatedUser {
                id: id.to_string(),
                email: email_db,
                role: "tenant".to_string(),
            });
        } else {
            return Err(AuthServiceError::InvalidCredentials);
        }
    }

    Err(AuthServiceError::InvalidCredentials)
}

pub async fn register_platform_admin(
    pool: &PgPool,
    email: &str,
    password: &str,
) -> Result<(), AuthServiceError> {

    let existing = sqlx::query(
        "SELECT id FROM platform_admins WHERE email = $1"
    )
        .bind(email)
        .fetch_optional(pool)
        .await?;

    if existing.is_some() {
        return Err(AuthServiceError::EmailAlreadyExists);
    }

    let hashed_password = security::hash_password(password)?;

    sqlx::query(
        r#"
        INSERT INTO platform_admins (email, password_hash)
        VALUES ($1, $2)
        "#
    )
        .bind(email)
        .bind(hashed_password)
        .execute(pool)
        .await?;

    Ok(())
}
