use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct UserResponse {
    pub id: i32,
    pub email: String,
    pub firstname: String,
    pub lastname: String,
    pub fidelity_code: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct CreateUserResponse {
    pub id: i32,
    pub email: String,
    pub firstname: String,
    pub lastname: String,
    pub fidelity_code: String,
}

#[derive(Deserialize, ToSchema)]
pub struct CreateUserRequest {
    pub email: String,
    pub firstname: String,
    pub lastname: String,
    pub password: String,
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateUserRequest {
    pub firstname: Option<String>,
    pub lastname: Option<String>,
    pub phone: Option<String>,
}
