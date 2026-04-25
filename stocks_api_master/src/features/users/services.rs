use sqlx::{PgPool, Row};
use rand::Rng;

use super::dto::{UserResponse, CreateUserResponse, CreateUserRequest, UpdateUserRequest};

const FIDELITY_CODE_PREFIX: &str = "FID-";
const FIDELITY_CODE_DIGITS: usize = 8;
const FIDELITY_CODE_MAX_RETRIES: usize = 5;

fn generate_fidelity_code() -> String {
    let mut rng = rand::thread_rng();
    let mut code = String::with_capacity(FIDELITY_CODE_PREFIX.len() + FIDELITY_CODE_DIGITS);
    code.push_str(FIDELITY_CODE_PREFIX);
    for _ in 0..FIDELITY_CODE_DIGITS {
        code.push(char::from(b'0' + rng.gen_range(0..10)));
    }
    code
}

fn row_to_user(row: &sqlx::postgres::PgRow) -> UserResponse {
    UserResponse {
        id: row.get("id_usr"),
        email: row.get("email_usr"),
        firstname: row.get("firstname_usr"),
        lastname: row.get("lastname_usr"),
        fidelity_code: row.try_get("fidelity_code_usr").ok(),
    }
}

pub async fn get_all_users(pool: &PgPool) -> Result<Vec<UserResponse>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id_usr, email_usr, firstname_usr, lastname_usr, fidelity_code_usr
         FROM users_usr
         ORDER BY id_usr"
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.iter().map(row_to_user).collect())
}

pub async fn get_user_by_id(pool: &PgPool, id: i32) -> Result<Option<UserResponse>, sqlx::Error> {
    let row = sqlx::query(
        "SELECT id_usr, email_usr, firstname_usr, lastname_usr, fidelity_code_usr
         FROM users_usr
         WHERE id_usr = $1"
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(row.as_ref().map(row_to_user))
}

pub async fn get_user_by_fidelity_code(pool: &PgPool, code: &str) -> Result<Option<UserResponse>, sqlx::Error> {
    let row = sqlx::query(
        "SELECT id_usr, email_usr, firstname_usr, lastname_usr, fidelity_code_usr
         FROM users_usr
         WHERE fidelity_code_usr = $1"
    )
    .bind(code)
    .fetch_optional(pool)
    .await?;

    Ok(row.as_ref().map(row_to_user))
}

pub async fn create_user(pool: &PgPool, request: CreateUserRequest) -> Result<CreateUserResponse, sqlx::Error> {
    for _ in 0..FIDELITY_CODE_MAX_RETRIES {
        let code = generate_fidelity_code();

        let result = sqlx::query(
            "INSERT INTO users_usr (email_usr, firstname_usr, lastname_usr, password_usr, fidelity_code_usr)
             VALUES ($1, $2, $3, $4, $5)
             RETURNING id_usr, email_usr, firstname_usr, lastname_usr, fidelity_code_usr"
        )
        .bind(&request.email)
        .bind(&request.firstname)
        .bind(&request.lastname)
        .bind(&request.password)
        .bind(&code)
        .fetch_one(pool)
        .await;

        match result {
            Ok(row) => {
                return Ok(CreateUserResponse {
                    id: row.get("id_usr"),
                    email: row.get("email_usr"),
                    firstname: row.get("firstname_usr"),
                    lastname: row.get("lastname_usr"),
                    fidelity_code: row.get("fidelity_code_usr"),
                });
            }
            Err(sqlx::Error::Database(db_err)) if db_err.constraint() == Some("users_usr_fidelity_code_usr_key") => {
                continue;
            }
            Err(e) => return Err(e),
        }
    }

    Err(sqlx::Error::Protocol("Impossible de générer un code de fidélité unique".to_string()))
}

pub async fn update_user(pool: &PgPool, id: i32, request: UpdateUserRequest) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE users_usr
         SET firstname_usr = COALESCE($1, firstname_usr),
             lastname_usr  = COALESCE($2, lastname_usr),
             phone_usr     = COALESCE($3, phone_usr),
             updated_at    = NOW()
         WHERE id_usr = $4"
    )
    .bind(&request.firstname)
    .bind(&request.lastname)
    .bind(&request.phone)
    .bind(id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn delete_user(pool: &PgPool, id: i32) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM users_usr WHERE id_usr = $1")
        .bind(id)
        .execute(pool)
        .await?;

    Ok(())
}
