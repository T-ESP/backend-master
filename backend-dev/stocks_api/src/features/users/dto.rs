use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct UserResponse {
    pub id: i32,
    pub email: String,
    pub firstname: String,
    pub lastname: String,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_response_serialization() {
        let user = UserResponse {
            id: 1,
            email: "test@example.com".to_string(),
            firstname: "John".to_string(),
            lastname: "Doe".to_string(),
        };

        let json = serde_json::to_string(&user);
        assert!(json.is_ok());

        let json_str = json.unwrap();
        assert!(json_str.contains("test@example.com"));
        assert!(json_str.contains("John"));
        assert!(json_str.contains("Doe"));
    }

    #[test]
    fn test_user_response_deserialization() {
        let json = r#"{
            "id": 1,
            "email": "test@example.com",
            "firstname": "John",
            "lastname": "Doe"
        }"#;

        let user: Result<UserResponse, _> = serde_json::from_str(json);
        assert!(user.is_ok());

        let user = user.unwrap();
        assert_eq!(user.id, 1);
        assert_eq!(user.email, "test@example.com");
        assert_eq!(user.firstname, "John");
        assert_eq!(user.lastname, "Doe");
    }

    #[test]
    fn test_create_user_request_deserialization() {
        let json = r#"{
            "email": "newuser@example.com",
            "firstname": "Jane",
            "lastname": "Smith",
            "password": "securepassword123"
        }"#;

        let request: Result<CreateUserRequest, _> = serde_json::from_str(json);
        assert!(request.is_ok());

        let request = request.unwrap();
        assert_eq!(request.email, "newuser@example.com");
        assert_eq!(request.firstname, "Jane");
        assert_eq!(request.lastname, "Smith");
        assert_eq!(request.password, "securepassword123");
    }

    #[test]
    fn test_update_user_request_partial_update() {
        let json = r#"{
            "firstname": "UpdatedName"
        }"#;

        let request: Result<UpdateUserRequest, _> = serde_json::from_str(json);
        assert!(request.is_ok());

        let request = request.unwrap();
        assert_eq!(request.firstname, Some("UpdatedName".to_string()));
        assert_eq!(request.lastname, None);
        assert_eq!(request.phone, None);
    }

    #[test]
    fn test_update_user_request_full_update() {
        let json = r#"{
            "firstname": "NewFirstName",
            "lastname": "NewLastName",
            "phone": "+1234567890"
        }"#;

        let request: Result<UpdateUserRequest, _> = serde_json::from_str(json);
        assert!(request.is_ok());

        let request = request.unwrap();
        assert_eq!(request.firstname, Some("NewFirstName".to_string()));
        assert_eq!(request.lastname, Some("NewLastName".to_string()));
        assert_eq!(request.phone, Some("+1234567890".to_string()));
    }

    #[test]
    fn test_update_user_request_empty() {
        let json = r#"{}"#;

        let request: Result<UpdateUserRequest, _> = serde_json::from_str(json);
        assert!(request.is_ok());

        let request = request.unwrap();
        assert_eq!(request.firstname, None);
        assert_eq!(request.lastname, None);
        assert_eq!(request.phone, None);
    }

    #[test]
    fn test_create_user_request_with_special_characters() {
        let json = r#"{
            "email": "user+test@example.com",
            "firstname": "José",
            "lastname": "O'Brien",
            "password": "p@ssw0rd!#$"
        }"#;

        let request: Result<CreateUserRequest, _> = serde_json::from_str(json);
        assert!(request.is_ok());

        let request = request.unwrap();
        assert_eq!(request.email, "user+test@example.com");
        assert_eq!(request.firstname, "José");
        assert_eq!(request.lastname, "O'Brien");
        assert_eq!(request.password, "p@ssw0rd!#$");
    }
}