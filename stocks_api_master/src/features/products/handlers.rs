use axum::{
    extract::{Path, Query, Extension},
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use sqlx::PgPool;

use crate::common::responses::{SuccessResponse, ErrorResponse};
use crate::common::error_codes;
use serde_json::{json, Value};

use super::dto::{CreateProductRequest, UpdateProductRequest, StockUpdateRequest, AddPriceRequest, SearchParams, ProductResponse, ProductLightResponse};
use super::services::ProductService;

/// GET /api/products/light - Light search for products
#[utoipa::path(
    get,
    path = "/products/light",
    tag = "products",
    params(
        ("q" = Option<String>, Query, description = "Search term")
    ),
    responses(
        (status = 200, description = "Products retrieved successfully", body = Vec<ProductLightResponse>),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn search_products_light(
    Query(params): Query<std::collections::HashMap<String, String>>,
    Extension(pool): Extension<PgPool>,
) -> Response {
    let q = params.get("q").cloned();
    match ProductService::search_products_light(&pool, q).await {
        Ok(products) => (
            StatusCode::OK,
            Json(SuccessResponse::new(products, "Products retrieved successfully".to_string()))
        ).into_response(),
        Err(e) => {
            eprintln!("Database error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    error_codes::DATABASE_ERROR.to_string(),
                    "Erreur de base de données".to_string()
                ))
            ).into_response()
        }
    }
}

/// GET /api/products - Get all products with pagination
#[utoipa::path(
    get,
    path = "/products",
    tag = "products",
    params(SearchParams),
    responses(
        (status = 200, description = "Products retrieved successfully", body = Vec<ProductResponse>),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn get_products(
    Query(params): Query<SearchParams>,
    Extension(pool): Extension<PgPool>,
) -> Response {
    match ProductService::get_products(&pool, &params).await {
        Ok(products) => (
            StatusCode::OK,
            Json(SuccessResponse::new(products, "Products retrieved successfully".to_string()))
        ).into_response(),
        Err(e) => {
            eprintln!("Database error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    error_codes::DATABASE_ERROR.to_string(),
                    "Erreur de base de données".to_string()
                ))
            ).into_response()
        }
    }
}

/// GET /api/products/:id - Get product by ID
#[utoipa::path(
    get,
    path = "/products/{id}",
    tag = "products",
    params(
        ("id" = i32, Path, description = "Product ID")
    ),
    responses(
        (status = 200, description = "Product retrieved successfully", body = ProductResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn get_product_by_id(
    Path((_, id)): Path<(String, i32)>,
    Extension(pool): Extension<PgPool>,
) -> Response {
    match ProductService::get_product_by_id(&pool, id).await {
        Ok(product) => (
            StatusCode::OK,
            Json(SuccessResponse::new(product, "Product retrieved successfully".to_string()))
        ).into_response(),
        Err(e) => {
            eprintln!("Database error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    error_codes::DATABASE_ERROR.to_string(),
                    "Erreur de base de données".to_string()
                ))
            ).into_response()
        }
    }
}

/// GET /api/products/reference/:reference - Get product by reference
pub async fn get_product_by_reference(
    Path((_, reference)): Path<(String, String)>,
    Extension(pool): Extension<PgPool>,
) -> Response {
    match ProductService::get_product_by_reference(&pool, &reference).await {
        Ok(product) => (
            StatusCode::OK,
            Json(SuccessResponse::new(product, "Product retrieved successfully".to_string()))
        ).into_response(),
        Err(e) => {
            eprintln!("Database error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    error_codes::DATABASE_ERROR.to_string(),
                    "Erreur de base de données".to_string()
                ))
            ).into_response()
        }
    }
}

/// GET /api/products/:id/stock - Get stock quantity for a product (lightweight)
#[utoipa::path(
    get,
    path = "/products/{id}/stock",
    tag = "products",
    params(
        ("id" = i32, Path, description = "Product ID")
    ),
    responses(
        (status = 200, description = "Stock retrieved successfully"),
        (status = 404, description = "Product not found", body = ErrorResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn get_product_stock(
    Path((_, id)): Path<(String, i32)>,
    Extension(pool): Extension<PgPool>,
) -> Response {
    let row = sqlx::query(
        "SELECT id_pro, stock_quantity_pro, status_pro::text as status_pro
         FROM products_pro
         WHERE id_pro = $1"
    )
    .bind(id)
    .fetch_optional(&pool)
    .await;

    match row {
        Ok(Some(row)) => {
            use sqlx::Row;
            let data = json!({
                "product_id": row.get::<i32, _>("id_pro"),
                "stock_quantity": row.get::<i32, _>("stock_quantity_pro"),
                "status": row.get::<String, _>("status_pro"),
            });
            (
                StatusCode::OK,
                Json(SuccessResponse::new(data, "Stock retrieved successfully".to_string()))
            ).into_response()
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                error_codes::NOT_FOUND.to_string(),
                format!("Produit {} non trouvé", id)
            ))
        ).into_response(),
        Err(e) => {
            eprintln!("Database error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    error_codes::DATABASE_ERROR.to_string(),
                    "Erreur de base de données".to_string()
                ))
            ).into_response()
        }
    }
}


/// PATCH /api/products/:id/stock - Update stock quantity for a product
#[utoipa::path(
    patch,
    path = "/products/{id}/stock",
    tag = "products",
    params(
        ("id" = i32, Path, description = "Product ID")
    ),
    request_body = StockUpdateRequest,
    responses(
        (status = 200, description = "Stock updated successfully", body = ProductResponse),
        (status = 404, description = "Product not found", body = ErrorResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn update_stock(
    Path((_, id)): Path<(String, i32)>,
    Extension(pool): Extension<PgPool>,
    Json(payload): Json<StockUpdateRequest>,
) -> Response {
    match ProductService::product_exists(&pool, id).await {
        Ok(false) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    error_codes::NOT_FOUND.to_string(),
                    format!("Produit avec ID {} non trouvé", id)
                ))
            ).into_response();
        }
        Ok(true) => {} // Continue
        Err(e) => {
            eprintln!("Database error: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    error_codes::DATABASE_ERROR.to_string(),
                    "Erreur de base de données".to_string()
                ))
            ).into_response();
        }
    }

    match ProductService::update_stock(&pool, id, payload.quantity).await {
        Ok(updated_product) => (
            StatusCode::OK,
            Json(SuccessResponse::new(updated_product, "Stock updated successfully".to_string()))
        ).into_response(),
        Err(e) => {
            eprintln!("Database error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    error_codes::DATABASE_ERROR.to_string(),
                    "Erreur lors de la mise à jour du stock".to_string()
                ))
            ).into_response()
        }
    }
}

/// POST /api/products - Create a new product
#[utoipa::path(
    post,
    path = "/products",
    tag = "products",
    request_body = CreateProductRequest,
    responses(
        (status = 201, description = "Product created successfully", body = ProductResponse),
        (status = 409, description = "Product reference already exists", body = ErrorResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn create_product(
    Extension(pool): Extension<PgPool>,
    Json(req): Json<CreateProductRequest>,
) -> Response {
    // Check if reference already exists
    match ProductService::check_reference_exists(&pool, &req.reference).await {
        Ok(true) => {
            return (
                StatusCode::CONFLICT,
                Json(ErrorResponse::new(
                    error_codes::ALREADY_EXISTS.to_string(),
                    format!("Un produit avec la référence '{}' existe déjà", req.reference)
                ))
            ).into_response();
        }
        Ok(false) => {} // Continue
        Err(e) => {
            eprintln!("Database error: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    error_codes::DATABASE_ERROR.to_string(),
                    "Erreur de base de données".to_string()
                ))
            ).into_response();
        }
    }

    // Insert new product
    match ProductService::create_product(
        &pool,
        &req.name,
        &req.category,
        &req.reference,
        req.supplier_id,
        req.stock_quantity,
        req.buying_price,
        req.status,  // ✅ ADD THIS LINE
    ).await {
        Ok(new_product) => (
            StatusCode::CREATED,
            Json(SuccessResponse::new(new_product, "Product created successfully".to_string()))
        ).into_response(),
        Err(e) => {
            eprintln!("Database error: {}", e);
            let (code, message) = if e.to_string().contains("foreign key constraint") {
                (error_codes::NOT_FOUND, "supplier_id invalide".to_string())
            } else {
                (error_codes::DATABASE_ERROR, "Erreur lors de la création du produit".to_string())
            };

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(code.to_string(), message))
            ).into_response()
        }
    }
}


/// PUT /api/products/:id - Update a product
#[utoipa::path(
    put,
    path = "/products/{id}",
    tag = "products",
    params(
        ("id" = i32, Path, description = "Product ID")
    ),
    request_body = UpdateProductRequest,
    responses(
        (status = 200, description = "Product updated successfully", body = ProductResponse),
        (status = 404, description = "Product not found", body = ErrorResponse),
        (status = 409, description = "Reference already exists", body = ErrorResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn update_product(
    Path((_, id)): Path<(String, i32)>,
    Extension(pool): Extension<PgPool>,
    Json(product): Json<UpdateProductRequest>,
) -> Response {
    // Check if product exists
    match ProductService::product_exists(&pool, id).await {
        Ok(false) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    error_codes::NOT_FOUND.to_string(),
                    format!("Produit avec ID {} non trouvé", id)
                ))
            ).into_response();
        }
        Ok(true) => {} // Continue
        Err(e) => {
            eprintln!("Database error: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    error_codes::DATABASE_ERROR.to_string(),
                    "Erreur de base de données".to_string()
                ))
            ).into_response();
        }
    }

    // Check if reference is being changed and if new reference already exists
    if let Some(ref new_reference) = product.reference {
        match ProductService::check_reference_exists_excluding_id(&pool, new_reference, id).await {
            Ok(true) => {
                return (
                    StatusCode::CONFLICT,
                    Json(ErrorResponse::new(
                        error_codes::ALREADY_EXISTS.to_string(),
                        format!("Un autre produit utilise déjà la référence '{}'", new_reference)
                    ))
                ).into_response();
            }
            Ok(false) => {} // Continue
            Err(e) => {
                eprintln!("Database error: {}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new(
                        error_codes::DATABASE_ERROR.to_string(),
                        "Erreur de base de données".to_string()
                    ))
                ).into_response();
            }
        }
    }

    // Update the product with date_last_reassor_pro = NOW() when stock changes
    match ProductService::update_product(
        &pool,
        id,
        product.name.as_deref(),
        product.category.as_deref(),
        product.reference.as_deref(),
        product.supplier_id,
        product.stock_quantity,
        product.buying_price,
        product.status,  // ✅ ADD THIS LINE
        product.stock_quantity.is_some(),
    ).await {
        Ok(updated_product) => (
            StatusCode::OK,
            Json(SuccessResponse::new(updated_product, "Product updated successfully".to_string()))
        ).into_response(),
        Err(e) => {
            eprintln!("Database error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    error_codes::DATABASE_ERROR.to_string(),
                    "Erreur lors de la mise à jour du produit".to_string()
                ))
            ).into_response()
        }
    }
}

/// DELETE /api/products/:id - Delete a product
#[utoipa::path(
    delete,
    path = "/products/{id}",
    tag = "products",
    params(
        ("id" = i32, Path, description = "Product ID")
    ),
    responses(
        (status = 200, description = "Product deleted successfully"),
        (status = 404, description = "Product not found", body = ErrorResponse),
        (status = 500, description = "Database error or foreign key constraint", body = ErrorResponse)
    )
)]
pub async fn delete_product(
    Path((_, id)): Path<(String, i32)>,
    Extension(pool): Extension<PgPool>,
) -> Response {
    // Check if product exists
    match ProductService::product_exists(&pool, id).await {
        Ok(false) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    error_codes::NOT_FOUND.to_string(),
                    format!("Produit avec ID {} non trouvé", id)
                ))
            ).into_response();
        }
        Ok(true) => {} // Continue
        Err(e) => {
            eprintln!("Database error: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    error_codes::DATABASE_ERROR.to_string(),
                    "Erreur de base de données".to_string()
                ))
            ).into_response();
        }
    }

    // Delete the product
    match ProductService::delete_product(&pool, id).await {
        Ok(rows_affected) => {
            if rows_affected == 0 {
                (
                    StatusCode::NOT_FOUND,
                    Json(ErrorResponse::new(
                        error_codes::NOT_FOUND.to_string(),
                        format!("Produit avec ID {} non trouvé", id)
                    ))
                ).into_response()
            } else {
                (
                    StatusCode::OK,
                    Json(SuccessResponse::new((), "Product deleted successfully".to_string()))
                ).into_response()
            }
        }
        Err(e) => {
            eprintln!("Database error: {}", e);
            let (code, message) = if e.to_string().contains("foreign key constraint") {
                (error_codes::DATABASE_ERROR, "Impossible de supprimer ce produit car il est référencé dans d'autres tables".to_string())
            } else {
                (error_codes::DATABASE_ERROR, "Erreur lors de la suppression du produit".to_string())
            };

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(code.to_string(), message))
            ).into_response()
        }
    }
}

/// POST /products/:id/prices - Add a selling price entry
#[utoipa::path(
    post,
    path = "/products/{id}/prices",
    tag = "products",
    params(("id" = i32, Path, description = "Product ID")),
    request_body = AddPriceRequest,
    responses(
        (status = 201, description = "Price added successfully"),
        (status = 404, description = "Product not found", body = ErrorResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn add_product_price(
    Path((_, id)): Path<(String, i32)>,
    Extension(pool): Extension<PgPool>,
    Json(req): Json<AddPriceRequest>,
) -> Response {
    match ProductService::product_exists(&pool, id).await {
        Ok(false) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    error_codes::NOT_FOUND.to_string(),
                    format!("Produit avec ID {} non trouvé", id)
                ))
            ).into_response();
        }
        Ok(true) => {}
        Err(e) => {
            eprintln!("Database error: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    error_codes::DATABASE_ERROR.to_string(),
                    "Erreur de base de données".to_string()
                ))
            ).into_response();
        }
    }

    match ProductService::add_product_price(&pool, id, req.selling_price).await {
        Ok(price) => (
            StatusCode::CREATED,
            Json(SuccessResponse::new(price, "Price added successfully".to_string()))
        ).into_response(),
        Err(e) => {
            eprintln!("Database error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    error_codes::DATABASE_ERROR.to_string(),
                    "Erreur lors de l'ajout du prix".to_string()
                ))
            ).into_response()
        }
    }
}
