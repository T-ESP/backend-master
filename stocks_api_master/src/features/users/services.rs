use sqlx::PgPool;
use crate::common::{responses::{SuccessResponse, ErrorResponse}, error_codes};
use super::dto::{UserResponse, CreateUserRequest, UpdateUserRequest};

pub async fn get_all_users(pool: &PgPool) -> Result<Vec<UserResponse>, sqlx::Error> {
    let users = sqlx::query_as!(UserResponse,
        "SELECT id_usr as id, email_usr as email, firstname_usr as firstname, lastname_usr as lastname FROM users_usr"
    )
    .fetch_all(pool)
    .await?;

    Ok(users)
}

pub async fn create_user(pool: &PgPool, request: CreateUserRequest) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO users_usr (email_usr, firstname_usr, lastname_usr, password_usr) VALUES ($1, $2, $3, $4)",
        request.email,
        request.firstname,
        request.lastname,
        request.password
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn update_user(pool: &PgPool, id: i32, request: UpdateUserRequest) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE users_usr SET firstname_usr = COALESCE($1, firstname_usr), lastname_usr = COALESCE($2, lastname_usr), phone_usr = COALESCE($3, phone_usr) WHERE id_usr = $4",
        request.firstname,
        request.lastname,
        request.phone,
        id
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn delete_user(pool: &PgPool, id: i32) -> Result<(), sqlx::Error> {
    sqlx::query!("DELETE FROM users_usr WHERE id_usr = $1", id)
        .execute(pool)
        .await?;

    Ok(())
}