use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Deserialize, ToSchema)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct AdminRegisterRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LoginResponse {
    pub token: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_login_request_deserialization() {
        let json = r#"{
            "email": "admin@example.com",
            "password": "securepassword"
        }"#;

        let request: Result<LoginRequest, _> = serde_json::from_str(json);
        assert!(request.is_ok());

        let request = request.unwrap();
        assert_eq!(request.email, "admin@example.com");
        assert_eq!(request.password, "securepassword");
    }

    #[test]
    fn test_admin_register_request_deserialization() {
        let json = r#"{
            "email": "admin@example.com",
            "password": "adminpass123"
        }"#;

        let request: Result<AdminRegisterRequest, _> = serde_json::from_str(json);
        assert!(request.is_ok());

        let request = request.unwrap();
        assert_eq!(request.email, "admin@example.com");
        assert_eq!(request.password, "adminpass123");
    }

    #[test]
    fn test_login_response_serialization() {
        let response = LoginResponse {
            token: "jwt_token_example".to_string(),
        };

        let json = serde_json::to_string(&response);
        assert!(json.is_ok());

        let json_str = json.unwrap();
        assert!(json_str.contains("jwt_token_example"));
    }
}
