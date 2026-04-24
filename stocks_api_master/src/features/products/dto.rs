use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::Type;
use utoipa::{ToSchema, IntoParams};

// ---- Response DTOs ----
#[derive(Debug, Serialize, ToSchema)]
pub struct ProductResponse {
    pub id: i32,
    pub name: String,
    pub category: String,
    pub reference: String,
    pub supplier_id: i32,
    pub stock_quantity: i32,
    pub buying_price: f64,
    pub status: ProductStatus,
    pub date_last_reassor: chrono::DateTime<chrono::Utc>,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ProductWithSupplierResponse {
    pub id: i32,
    pub name: String,
    pub category: String,
    pub reference: String,
    pub supplier_id: i32,
    pub supplier_name: String,
    pub supplier_email: String,
    pub stock_quantity: i32,
    pub buying_price: f64,
    pub status: ProductStatus,
    pub date_last_reassor: chrono::DateTime<chrono::Utc>,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ProductStockResponse {
    pub id: i32,
    pub name: String,
    pub reference: String,
    pub stock_quantity: i32,
    pub previous_quantity: i32,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ProductLightResponse {
    pub id: i32,
    pub name: String,
    pub category: String,
}

// ---- Request DTOs ----
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateProductRequest {
    pub name: String,
    pub category: String,
    pub reference: String,
    pub supplier_id: i32,
    pub stock_quantity: i32,
    pub buying_price: f64,
    pub status: Option<ProductStatus>,
    /// Prix de vente initial : inséré dans `productprices_prp` pour des KPI cohérents dès la création.
    #[serde(default)]
    pub selling_price: Option<f64>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct AddPriceRequest {
    pub selling_price: f64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ProductPriceResponse {
    pub id: i32,
    pub product_ref: i32,
    pub selling_price: f64,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateProductRequest {
    pub name: Option<String>,
    pub category: Option<String>,
    pub reference: Option<String>,
    pub supplier_id: Option<i32>,
    pub stock_quantity: Option<i32>,
    pub buying_price: Option<f64>,
    pub status: Option<ProductStatus>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct StockUpdateRequest {
    pub quantity: i32,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct SearchParams {
    pub q: Option<String>,
    pub category: Option<String>,
    pub supplier_id: Option<i32>,
    pub min_price: Option<f64>,
    pub max_price: Option<f64>,
    pub min_stock: Option<i32>,
    pub max_stock: Option<i32>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

// ---- Helper implementations ----
impl ProductResponse {
    pub fn from_row(row: &sqlx::postgres::PgRow) -> Self {
        use sqlx::Row;
        Self {
            id: row.get("id_pro"),
            name: row.get("name_pro"),
            category: row.get("category_pro"),
            reference: row.get("reference_pro"),
            supplier_id: row.get("supplier_id_pro"),
            stock_quantity: row.get("stock_quantity_pro"),
            buying_price: row.get::<Decimal, _>("buying_price_pro")
                .to_f64()
                .unwrap_or(0.0),
            status: row.get::<ProductStatus, _>("status_pro"),  // ✅ ADD TYPE ANNOTATION
            date_last_reassor: row.get("date_last_reassor_pro"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    }
}

impl ProductWithSupplierResponse {
    pub fn from_row(row: &sqlx::postgres::PgRow) -> Self {
        use sqlx::Row;
        Self {
            id: row.get("id_pro"),
            name: row.get("name_pro"),
            category: row.get("category_pro"),
            reference: row.get("reference_pro"),
            supplier_id: row.get("supplier_id_pro"),
            supplier_name: row.get("supplier_name"),
            supplier_email: row.get("supplier_email"),
            stock_quantity: row.get("stock_quantity_pro"),
            buying_price: row.get::<Decimal, _>("buying_price_pro")
                .to_f64()
                .unwrap_or(0.0),
            status: row.get::<ProductStatus, _>("status_pro"),
            date_last_reassor: row.get("date_last_reassor_pro"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    }
}

impl Default for SearchParams {
    fn default() -> Self {
        Self {
            q: None,
            category: None,
            supplier_id: None,
            min_price: None,
            max_price: None,
            min_stock: None,
            max_stock: None,
            limit: Some(50),
            offset: Some(0),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "product_status_enum")]
pub enum ProductStatus {
    #[sqlx(rename = "in_stock")]
    InStock,
    #[sqlx(rename = "out_of_stock")]
    OutOfStock,
    #[sqlx(rename = "discontinued")]
    Discontinued,
    #[sqlx(rename = "ordered")]
    Ordered,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_params_default() {
        let default_params = SearchParams::default();

        assert_eq!(default_params.q, None);
        assert_eq!(default_params.category, None);
        assert_eq!(default_params.supplier_id, None);
        assert_eq!(default_params.min_price, None);
        assert_eq!(default_params.max_price, None);
        assert_eq!(default_params.min_stock, None);
        assert_eq!(default_params.max_stock, None);
        assert_eq!(default_params.limit, Some(50));
        assert_eq!(default_params.offset, Some(0));
    }

    #[test]
    fn test_product_status_equality() {
        assert_eq!(ProductStatus::InStock, ProductStatus::InStock);
        assert_eq!(ProductStatus::OutOfStock, ProductStatus::OutOfStock);
        assert_eq!(ProductStatus::Discontinued, ProductStatus::Discontinued);
        assert_eq!(ProductStatus::Ordered, ProductStatus::Ordered);

        assert_ne!(ProductStatus::InStock, ProductStatus::OutOfStock);
        assert_ne!(ProductStatus::Discontinued, ProductStatus::Ordered);
    }

    #[test]
    fn test_product_status_clone() {
        let status = ProductStatus::InStock;
        let cloned = status.clone();
        assert_eq!(status, cloned);
    }

    #[test]
    fn test_product_status_serialization() {
        let status = ProductStatus::InStock;
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("InStock") || json.contains("in_stock"));
    }

    #[test]
    fn test_product_status_deserialization() {
        let json_in_stock = r#""InStock""#;
        let status: Result<ProductStatus, _> = serde_json::from_str(json_in_stock);
        assert!(status.is_ok());
        assert_eq!(status.unwrap(), ProductStatus::InStock);

        let json_out_of_stock = r#""OutOfStock""#;
        let status: Result<ProductStatus, _> = serde_json::from_str(json_out_of_stock);
        assert!(status.is_ok());
        assert_eq!(status.unwrap(), ProductStatus::OutOfStock);

        let json_discontinued = r#""Discontinued""#;
        let status: Result<ProductStatus, _> = serde_json::from_str(json_discontinued);
        assert!(status.is_ok());
        assert_eq!(status.unwrap(), ProductStatus::Discontinued);

        let json_ordered = r#""Ordered""#;
        let status: Result<ProductStatus, _> = serde_json::from_str(json_ordered);
        assert!(status.is_ok());
        assert_eq!(status.unwrap(), ProductStatus::Ordered);
    }

    #[test]
    fn test_create_product_request_deserialization() {
        let json = r#"{
            "name": "Test Product",
            "category": "Electronics",
            "reference": "REF001",
            "supplier_id": 1,
            "stock_quantity": 100,
            "buying_price": 49.99,
            "status": "InStock"
        }"#;

        let request: Result<CreateProductRequest, _> = serde_json::from_str(json);
        assert!(request.is_ok());

        let request = request.unwrap();
        assert_eq!(request.name, "Test Product");
        assert_eq!(request.category, "Electronics");
        assert_eq!(request.reference, "REF001");
        assert_eq!(request.supplier_id, 1);
        assert_eq!(request.stock_quantity, 100);
        assert_eq!(request.buying_price, 49.99);
        assert_eq!(request.status, Some(ProductStatus::InStock));
    }

    #[test]
    fn test_create_product_request_without_status() {
        let json = r#"{
            "name": "Test Product",
            "category": "Electronics",
            "reference": "REF001",
            "supplier_id": 1,
            "stock_quantity": 100,
            "buying_price": 49.99
        }"#;

        let request: Result<CreateProductRequest, _> = serde_json::from_str(json);
        assert!(request.is_ok());

        let request = request.unwrap();
        assert_eq!(request.status, None);
    }

    #[test]
    fn test_update_product_request_partial_update() {
        let json = r#"{
            "name": "Updated Name"
        }"#;

        let request: Result<UpdateProductRequest, _> = serde_json::from_str(json);
        assert!(request.is_ok());

        let request = request.unwrap();
        assert_eq!(request.name, Some("Updated Name".to_string()));
        assert_eq!(request.category, None);
        assert_eq!(request.reference, None);
        assert_eq!(request.supplier_id, None);
        assert_eq!(request.stock_quantity, None);
        assert_eq!(request.buying_price, None);
        assert_eq!(request.status, None);
    }

    #[test]
    fn test_update_product_request_full_update() {
        let json = r#"{
            "name": "Updated Product",
            "category": "New Category",
            "reference": "REF002",
            "supplier_id": 2,
            "stock_quantity": 200,
            "buying_price": 99.99,
            "status": "OutOfStock"
        }"#;

        let request: Result<UpdateProductRequest, _> = serde_json::from_str(json);
        assert!(request.is_ok());

        let request = request.unwrap();
        assert_eq!(request.name, Some("Updated Product".to_string()));
        assert_eq!(request.category, Some("New Category".to_string()));
        assert_eq!(request.reference, Some("REF002".to_string()));
        assert_eq!(request.supplier_id, Some(2));
        assert_eq!(request.stock_quantity, Some(200));
        assert_eq!(request.buying_price, Some(99.99));
        assert_eq!(request.status, Some(ProductStatus::OutOfStock));
    }

    #[test]
    fn test_stock_update_request_deserialization() {
        let json = r#"{"quantity": 50}"#;
        let request: Result<StockUpdateRequest, _> = serde_json::from_str(json);
        assert!(request.is_ok());
        assert_eq!(request.unwrap().quantity, 50);
    }

    #[test]
    fn test_stock_update_request_negative_quantity() {
        let json = r#"{"quantity": -10}"#;
        let request: Result<StockUpdateRequest, _> = serde_json::from_str(json);
        assert!(request.is_ok());
        assert_eq!(request.unwrap().quantity, -10);
    }

    #[test]
    fn test_search_params_custom_values() {
        let json = r#"{
            "q": "laptop",
            "category": "Electronics",
            "supplier_id": 5,
            "min_price": 100.0,
            "max_price": 500.0,
            "min_stock": 10,
            "max_stock": 100,
            "limit": 20,
            "offset": 10
        }"#;

        let params: Result<SearchParams, _> = serde_json::from_str(json);
        assert!(params.is_ok());

        let params = params.unwrap();
        assert_eq!(params.q, Some("laptop".to_string()));
        assert_eq!(params.category, Some("Electronics".to_string()));
        assert_eq!(params.supplier_id, Some(5));
        assert_eq!(params.min_price, Some(100.0));
        assert_eq!(params.max_price, Some(500.0));
        assert_eq!(params.min_stock, Some(10));
        assert_eq!(params.max_stock, Some(100));
        assert_eq!(params.limit, Some(20));
        assert_eq!(params.offset, Some(10));
    }

    #[test]
    fn test_product_response_serialization() {
        let product = ProductResponse {
            id: 1,
            name: "Test Product".to_string(),
            category: "Electronics".to_string(),
            reference: "REF001".to_string(),
            supplier_id: 1,
            stock_quantity: 100,
            buying_price: 49.99,
            status: ProductStatus::InStock,
            date_last_reassor: chrono::Utc::now(),
            created_at: Some(chrono::Utc::now()),
            updated_at: Some(chrono::Utc::now()),
        };

        let json = serde_json::to_string(&product);
        assert!(json.is_ok());

        let json_str = json.unwrap();
        assert!(json_str.contains("Test Product"));
        assert!(json_str.contains("Electronics"));
        assert!(json_str.contains("REF001"));
    }

    #[test]
    fn test_product_with_supplier_response_serialization() {
        let product = ProductWithSupplierResponse {
            id: 1,
            name: "Test Product".to_string(),
            category: "Electronics".to_string(),
            reference: "REF001".to_string(),
            supplier_id: 1,
            supplier_name: "Test Supplier".to_string(),
            supplier_email: "supplier@test.com".to_string(),
            stock_quantity: 100,
            buying_price: 49.99,
            status: ProductStatus::InStock,
            date_last_reassor: chrono::Utc::now(),
            created_at: Some(chrono::Utc::now()),
            updated_at: Some(chrono::Utc::now()),
        };

        let json = serde_json::to_string(&product);
        assert!(json.is_ok());

        let json_str = json.unwrap();
        assert!(json_str.contains("Test Supplier"));
        assert!(json_str.contains("supplier@test.com"));
    }
}