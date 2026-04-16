use serde::{Deserialize, Serialize};
use validator::Validate;
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct SupplierResponse {
    pub id: i32,
    pub name_sup: String,
    pub email_sup: String,
    pub phone_sup: String,
    pub address_sup: String,
    pub created_at: Option<chrono::NaiveDateTime>,
    pub updated_at: Option<chrono::NaiveDateTime>,
}

#[derive(Deserialize, Validate, ToSchema)]
pub struct CreateSupplierRequest {
    #[validate(length(min = 1, max = 255))]
    pub name_sup: String,
    #[validate(email)]
    pub email_sup: String,
    #[validate(length(min = 1, max = 20))]
    pub phone_sup: String,
    #[validate(length(min = 1, max = 500))]
    pub address_sup: String,
}

#[derive(Deserialize, Validate, ToSchema)]
pub struct UpdateSupplierRequest {
    #[validate(length(min = 1, max = 255))]
    pub name_sup: Option<String>,
    #[validate(email)]
    pub email_sup: Option<String>,
    #[validate(length(min = 1, max = 20))]
    pub phone_sup: Option<String>,
    #[validate(length(min = 1, max = 500))]
    pub address_sup: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supplier_response_serialization() {
        let supplier = SupplierResponse {
            id: 1,
            name_sup: "ABC Supplies".to_string(),
            email_sup: "contact@abc.com".to_string(),
            phone_sup: "+1234567890".to_string(),
            address_sup: "123 Main St".to_string(),
            created_at: None,
            updated_at: None,
        };

        let json = serde_json::to_string(&supplier);
        assert!(json.is_ok());

        let json_str = json.unwrap();
        assert!(json_str.contains("ABC Supplies"));
        assert!(json_str.contains("contact@abc.com"));
        assert!(json_str.contains("+1234567890"));
        assert!(json_str.contains("123 Main St"));
    }

    #[test]
    fn test_supplier_response_deserialization() {
        let json = r#"{
            "id": 1,
            "name_sup": "XYZ Corp",
            "email_sup": "info@xyz.com",
            "phone_sup": "+9876543210",
            "address_sup": "456 Oak Ave",
            "created_at": null,
            "updated_at": null
        }"#;

        let supplier: Result<SupplierResponse, _> = serde_json::from_str(json);
        assert!(supplier.is_ok());

        let supplier = supplier.unwrap();
        assert_eq!(supplier.id, 1);
        assert_eq!(supplier.name_sup, "XYZ Corp");
        assert_eq!(supplier.email_sup, "info@xyz.com");
        assert_eq!(supplier.phone_sup, "+9876543210");
        assert_eq!(supplier.address_sup, "456 Oak Ave");
    }

    #[test]
    fn test_create_supplier_request_deserialization() {
        let json = r#"{
            "name_sup": "New Supplier",
            "email_sup": "new@supplier.com",
            "phone_sup": "+1111111111",
            "address_sup": "789 Elm St"
        }"#;

        let request: Result<CreateSupplierRequest, _> = serde_json::from_str(json);
        assert!(request.is_ok());

        let request = request.unwrap();
        assert_eq!(request.name_sup, "New Supplier");
        assert_eq!(request.email_sup, "new@supplier.com");
        assert_eq!(request.phone_sup, "+1111111111");
        assert_eq!(request.address_sup, "789 Elm St");
    }

    #[test]
    fn test_update_supplier_request_partial_update() {
        let json = r#"{
            "name_sup": "Updated Name"
        }"#;

        let request: Result<UpdateSupplierRequest, _> = serde_json::from_str(json);
        assert!(request.is_ok());

        let request = request.unwrap();
        assert_eq!(request.name_sup, Some("Updated Name".to_string()));
        assert_eq!(request.email_sup, None);
        assert_eq!(request.phone_sup, None);
        assert_eq!(request.address_sup, None);
    }

    #[test]
    fn test_update_supplier_request_full_update() {
        let json = r#"{
            "name_sup": "Fully Updated Supplier",
            "email_sup": "updated@supplier.com",
            "phone_sup": "+2222222222",
            "address_sup": "999 Updated Rd"
        }"#;

        let request: Result<UpdateSupplierRequest, _> = serde_json::from_str(json);
        assert!(request.is_ok());

        let request = request.unwrap();
        assert_eq!(request.name_sup, Some("Fully Updated Supplier".to_string()));
        assert_eq!(request.email_sup, Some("updated@supplier.com".to_string()));
        assert_eq!(request.phone_sup, Some("+2222222222".to_string()));
        assert_eq!(request.address_sup, Some("999 Updated Rd".to_string()));
    }

    #[test]
    fn test_update_supplier_request_empty() {
        let json = r#"{}"#;

        let request: Result<UpdateSupplierRequest, _> = serde_json::from_str(json);
        assert!(request.is_ok());

        let request = request.unwrap();
        assert_eq!(request.name_sup, None);
        assert_eq!(request.email_sup, None);
        assert_eq!(request.phone_sup, None);
        assert_eq!(request.address_sup, None);
    }

    #[test]
    fn test_create_supplier_request_with_long_address() {
        let long_address = "1234 Very Long Street Name, Apartment 567, Building C, District 8, City Name, State, Country, Postal Code 12345";

        let json = format!(r#"{{
            "name_sup": "Supplier Inc",
            "email_sup": "contact@supplier.com",
            "phone_sup": "+1234567890",
            "address_sup": "{}"
        }}"#, long_address);

        let request: Result<CreateSupplierRequest, _> = serde_json::from_str(&json);
        assert!(request.is_ok());

        let request = request.unwrap();
        assert_eq!(request.address_sup, long_address);
    }

    #[test]
    fn test_supplier_response_with_timestamps() {
        let now = chrono::NaiveDateTime::from_timestamp_opt(1609459200, 0).unwrap();

        let supplier = SupplierResponse {
            id: 1,
            name_sup: "Test Supplier".to_string(),
            email_sup: "test@test.com".to_string(),
            phone_sup: "+1234567890".to_string(),
            address_sup: "Test Address".to_string(),
            created_at: Some(now),
            updated_at: Some(now),
        };

        let json = serde_json::to_string(&supplier);
        assert!(json.is_ok());
    }

    #[test]
    fn test_create_supplier_request_email_variations() {
        let emails = vec![
            "simple@example.com",
            "user+tag@example.com",
            "user.name@example.co.uk",
            "admin@subdomain.example.com",
        ];

        for email in emails {
            let json = format!(r#"{{
                "name_sup": "Supplier",
                "email_sup": "{}",
                "phone_sup": "+1234567890",
                "address_sup": "Address"
            }}"#, email);

            let request: Result<CreateSupplierRequest, _> = serde_json::from_str(&json);
            assert!(request.is_ok());
            assert_eq!(request.unwrap().email_sup, email);
        }
    }

    #[test]
    fn test_update_supplier_request_multiple_fields() {
        let json = r#"{
            "email_sup": "newemail@supplier.com",
            "phone_sup": "+9999999999"
        }"#;

        let request: Result<UpdateSupplierRequest, _> = serde_json::from_str(json);
        assert!(request.is_ok());

        let request = request.unwrap();
        assert_eq!(request.name_sup, None);
        assert_eq!(request.email_sup, Some("newemail@supplier.com".to_string()));
        assert_eq!(request.phone_sup, Some("+9999999999".to_string()));
        assert_eq!(request.address_sup, None);
    }
}
