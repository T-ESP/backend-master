use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use chrono::NaiveDateTime;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Tenant {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub email: String,
    pub password_hash: String,
    pub phone: Option<String>,
    pub address: Option<String>,
    pub siret: Option<String>,
    pub db_name: String,
    pub status: String,
    pub created_at: NaiveDateTime,
}
