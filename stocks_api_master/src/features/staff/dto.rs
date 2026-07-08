use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct StaffResponse {
    pub id: i32,
    pub email: String,
    pub firstname: String,
    pub lastname: String,
    pub role: String,
    pub status: String,
}

#[derive(Deserialize, ToSchema)]
pub struct CreateStaffRequest {
    pub email: String,
    pub firstname: String,
    pub lastname: String,
    pub password: String,
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateStaffRequest {
    pub firstname: Option<String>,
    pub lastname: Option<String>,
    pub status: Option<String>,
}
