use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::common::security::{self, SecurityError};

#[derive(Debug)]
pub struct AuthenticatedUser {
    pub commerce_id: Option<Uuid>,
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

    // 1️⃣ Platform admin
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
        let email_db: String = row.get("email");
        let password_hash: String = row.get("password_hash");

        if security::verify_password(password, &password_hash)? {
            return Ok(AuthenticatedUser {
                commerce_id: None,
                email: email_db,
                role: "platform_admin".to_string(),
            });
        } else {
            return Err(AuthServiceError::InvalidCredentials);
        }
    }

    // 2️⃣ Commerce (tenant)
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
                commerce_id: Some(id),
                email: email_db,
                role: "commerce".to_string(),
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

    // Vérifier si email déjà existant
    let existing = sqlx::query(
        "SELECT id FROM platform_admins WHERE email = $1"
    )
        .bind(email)
        .fetch_optional(pool)
        .await?;

    if existing.is_some() {
        return Err(AuthServiceError::EmailAlreadyExists);
    }

    // Hash du mot de passe via ton module security
    let hashed = security::hash_password(password)?;

    sqlx::query(
        r#"
        INSERT INTO platform_admins (email, password_hash)
        VALUES ($1, $2)
        "#
    )
        .bind(email)
        .bind(hashed)
        .execute(pool)
        .await?;

    Ok(())
}
