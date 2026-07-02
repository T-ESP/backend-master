use sqlx::{PgPool, Row};

use crate::common::security;

use super::dto::{CreateStaffRequest, StaffResponse, UpdateStaffRequest};

#[derive(Debug)]
pub enum StaffServiceError {
    EmailAlreadyExists,
    NotFound,
    Database(sqlx::Error),
    Security(security::SecurityError),
}

impl From<sqlx::Error> for StaffServiceError {
    fn from(value: sqlx::Error) -> Self {
        StaffServiceError::Database(value)
    }
}

impl From<security::SecurityError> for StaffServiceError {
    fn from(value: security::SecurityError) -> Self {
        StaffServiceError::Security(value)
    }
}

fn row_to_staff(row: &sqlx::postgres::PgRow) -> StaffResponse {
    StaffResponse {
        id: row.get("id_stf"),
        email: row.get("email_stf"),
        firstname: row.get("firstname_stf"),
        lastname: row.get("lastname_stf"),
        role: row.get("role_stf"),
        status: row.get("status_stf"),
    }
}

pub async fn get_all_staff(pool: &PgPool) -> Result<Vec<StaffResponse>, StaffServiceError> {
    let rows = sqlx::query(
        "SELECT id_stf, email_stf, firstname_stf, lastname_stf, role_stf, status_stf
         FROM staff_stf
         ORDER BY id_stf",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.iter().map(row_to_staff).collect())
}

pub async fn get_staff_by_id(pool: &PgPool, id: i32) -> Result<Option<StaffResponse>, StaffServiceError> {
    let row = sqlx::query(
        "SELECT id_stf, email_stf, firstname_stf, lastname_stf, role_stf, status_stf
         FROM staff_stf
         WHERE id_stf = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(row.as_ref().map(row_to_staff))
}

pub async fn create_staff(pool: &PgPool, request: CreateStaffRequest) -> Result<StaffResponse, StaffServiceError> {
    let existing = sqlx::query("SELECT id_stf FROM staff_stf WHERE email_stf = $1")
        .bind(&request.email)
        .fetch_optional(pool)
        .await?;

    if existing.is_some() {
        return Err(StaffServiceError::EmailAlreadyExists);
    }

    let hashed_password = security::hash_password(&request.password)?;

    let row = sqlx::query(
        "INSERT INTO staff_stf (email_stf, firstname_stf, lastname_stf, password_stf)
         VALUES ($1, $2, $3, $4)
         RETURNING id_stf, email_stf, firstname_stf, lastname_stf, role_stf, status_stf",
    )
    .bind(&request.email)
    .bind(&request.firstname)
    .bind(&request.lastname)
    .bind(&hashed_password)
    .fetch_one(pool)
    .await?;

    Ok(row_to_staff(&row))
}

pub async fn update_staff(
    pool: &PgPool,
    id: i32,
    request: UpdateStaffRequest,
) -> Result<StaffResponse, StaffServiceError> {
    let row = sqlx::query(
        "UPDATE staff_stf
         SET firstname_stf = COALESCE($1, firstname_stf),
             lastname_stf  = COALESCE($2, lastname_stf),
             status_stf    = COALESCE($3, status_stf),
             updated_at    = NOW()
         WHERE id_stf = $4
         RETURNING id_stf, email_stf, firstname_stf, lastname_stf, role_stf, status_stf",
    )
    .bind(&request.firstname)
    .bind(&request.lastname)
    .bind(&request.status)
    .bind(id)
    .fetch_optional(pool)
    .await?;

    row.map(|r| row_to_staff(&r)).ok_or(StaffServiceError::NotFound)
}

pub async fn delete_staff(pool: &PgPool, id: i32) -> Result<(), StaffServiceError> {
    let result = sqlx::query("DELETE FROM staff_stf WHERE id_stf = $1")
        .bind(id)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(StaffServiceError::NotFound);
    }

    Ok(())
}
