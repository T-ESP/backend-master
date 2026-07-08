use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TenantResponse {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub email: String,
    pub phone: Option<String>,
    pub address: Option<String>,
    pub siret: Option<String>,
    pub status: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateTenantRequest {
    pub name: String,
    pub slug: String,
    pub email: String,
    pub phone: Option<String>,
    pub address: Option<String>,
    pub siret: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateTenantRequest {
    pub name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub address: Option<String>,
    pub siret: Option<String>,
    pub status: Option<String>,
}

/// Vue admin plateforme d'un employé, avec le contexte du commerce auquel il appartient.
#[derive(Debug, Serialize, ToSchema)]
pub struct AdminStaffResponse {
    pub commerce_id: Uuid,
    pub commerce_name: String,
    #[serde(flatten)]
    pub staff: crate::features::staff::dto::StaffResponse,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tenant_response_serialization() {
        let tenant = TenantResponse {
            id: Uuid::new_v4(),
            name: "Test Shop".to_string(),
            slug: "test-shop".to_string(),
            email: "shop@test.com".to_string(),
            phone: Some("+33123456789".to_string()),
            address: Some("1 rue de Paris".to_string()),
            siret: Some("12345678901234".to_string()),
            status: "active".to_string(),
        };

        let json = serde_json::to_string(&tenant);
        assert!(json.is_ok());
    }

    #[test]
    fn test_create_tenant_request_deserialization() {
        let json = r#"{
            "name": "My Commerce",
            "slug": "my-commerce",
            "email": "contact@mycommerce.com",
            "phone": "+33999999999",
            "address": "10 avenue des Champs",
            "siret": "12345678901234"
        }"#;

        let request: Result<CreateTenantRequest, _> = serde_json::from_str(json);
        assert!(request.is_ok());

        let request = request.unwrap();
        assert_eq!(request.name, "My Commerce");
        assert_eq!(request.slug, "my-commerce");
        assert_eq!(request.email, "contact@mycommerce.com");
        assert_eq!(request.phone, Some("+33999999999".to_string()));
        assert_eq!(request.address, Some("10 avenue des Champs".to_string()));
        assert_eq!(request.siret, Some("12345678901234".to_string()));
    }

    #[test]
    fn test_create_tenant_request_minimal() {
        let json = r#"{
            "name": "Minimal",
            "slug": "minimal",
            "email": "minimal@test.com"
        }"#;

        let request: Result<CreateTenantRequest, _> = serde_json::from_str(json);
        assert!(request.is_ok());

        let request = request.unwrap();
        assert_eq!(request.name, "Minimal");
        assert_eq!(request.slug, "minimal");
        assert_eq!(request.phone, None);
        assert_eq!(request.address, None);
        assert_eq!(request.siret, None);
    }
}
