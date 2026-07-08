use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::common::security::{self, SecurityError};
use crate::common::tenant_pool::TenantPoolManager;

#[derive(Debug)]
pub struct AuthenticatedUser {
    pub commerce_id: Option<Uuid>,
    pub slug: Option<String>,
    pub email: String,
    pub role: String,
    pub staff_id: Option<i32>,
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
    tenant_pool_manager: &TenantPoolManager,
    email: &str,
    password: &str,
    commerce_id_hint: Option<Uuid>,
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
                slug: None,
                email: email_db,
                role: "platform_admin".to_string(),
                staff_id: None,
            });
        } else {
            return Err(AuthServiceError::InvalidCredentials);
        }
    }

    // 2️⃣ Commerce (tenant owner)
    if let Some(row) = sqlx::query(
        r#"
        SELECT id, email, slug, password_hash
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
        let slug: String = row.get("slug");
        let password_hash: String = row.get("password_hash");

        if security::verify_password(password, &password_hash)? {
            return Ok(AuthenticatedUser {
                commerce_id: Some(id),
                slug: Some(slug),
                email: email_db,
                role: "commerce".to_string(),
                staff_id: None,
            });
        } else {
            return Err(AuthServiceError::InvalidCredentials);
        }
    }

    // 3️⃣ Employé (staff) — l'email n'est unique que dans son commerce, donc
    // le front doit préciser sur quel commerce se connecter.
    if let Some(commerce_id) = commerce_id_hint {
        let tenant_pool = tenant_pool_manager
            .get_pool(commerce_id)
            .await
            .map_err(|_| AuthServiceError::InvalidCredentials)?;

        if let Some(row) = sqlx::query(
            r#"
            SELECT id_stf, email_stf, password_stf, role_stf
            FROM staff_stf
            WHERE email_stf = $1 AND status_stf = 'active'
            "#,
        )
            .bind(email)
            .fetch_optional(&tenant_pool)
            .await?
        {
            let staff_id: i32 = row.get("id_stf");
            let email_db: String = row.get("email_stf");
            let password_hash: String = row.get("password_stf");
            let role: String = row.get("role_stf");

            if security::verify_password(password, &password_hash)? {
                // Le slug est nécessaire pour rediriger l'employé vers le
                // sous-domaine de son commerce, comme pour le compte commerce.
                let slug: Option<String> = sqlx::query_scalar(
                    "SELECT slug FROM commerces WHERE id = $1"
                )
                    .bind(commerce_id)
                    .fetch_optional(pool)
                    .await?;

                return Ok(AuthenticatedUser {
                    commerce_id: Some(commerce_id),
                    slug,
                    email: email_db,
                    role,
                    staff_id: Some(staff_id),
                });
            } else {
                return Err(AuthServiceError::InvalidCredentials);
            }
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
