use sqlx::{PgPool, Row};

use crate::common::security::{self, SecurityError};

#[derive(Debug)]
pub struct AuthenticatedUser {
    pub id: i32,
    pub email: String,
    pub firstname: String,
    pub lastname: String,
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
    let maybe_user = sqlx::query(
        r#"
        SELECT id_usr, email_usr, password_usr, firstname_usr, lastname_usr
        FROM users_usr
        WHERE email_usr = $1
        "#,
    )
    .bind(email)
    .fetch_optional(pool)
    .await?;

    let row = match maybe_user {
        Some(row) => row,
        None => return Err(AuthServiceError::InvalidCredentials),
    };

    let id: i32 = row.get("id_usr");
    let email_usr: String = row.get("email_usr");
    let password_usr: String = row.get("password_usr");
    let firstname_usr: String = row.get("firstname_usr");
    let lastname_usr: String = row.get("lastname_usr");

    let is_valid = security::verify_password(password, &password_usr)?;
    if !is_valid {
        return Err(AuthServiceError::InvalidCredentials);
    }

    Ok(AuthenticatedUser {
        id,
        email: email_usr,
        firstname: firstname_usr,
        lastname: lastname_usr,
    })
}

pub async fn register_user(
    pool: &PgPool,
    email: &str,
    password: &str,
    firstname: &str,
    lastname: &str,
) -> Result<(), AuthServiceError> {
    // 1) Vérifie si l'email existe déjà
    let existing_user = sqlx::query(
        r#"
        SELECT id_usr FROM users_usr WHERE email_usr = $1
        "#,
    )
    .bind(email)
    .fetch_optional(pool)
    .await?;

    if existing_user.is_some() {
        return Err(AuthServiceError::EmailAlreadyExists);
    }

    // 2) Hash le mot de passe
    let hashed_password = security::hash_password(password)?;

    // 3) Crée l'utilisateur
    sqlx::query(
        r#"
        INSERT INTO users_usr (email_usr, firstname_usr, lastname_usr, password_usr)
        VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind(email)
    .bind(firstname)
    .bind(lastname)
    .bind(hashed_password)
    .execute(pool)
    .await?;

    Ok(())
}

