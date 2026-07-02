use axum::{
    extract::{Path, Query, Extension},
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use sqlx::{PgPool, Row};
use rust_decimal::Decimal;
use chrono::Utc;

use crate::common::responses::{SuccessResponse, ErrorResponse};
use crate::common::error_codes;
use crate::common::security::Claims;

use super::dto::{CreateOrderRequest, UpdateOrderRequest, OrderQueryParams, OrderResponse, LineItemResponse, OrderStatsResponse};
use super::services::OrderService;
use crate::features::loyalty::services::LoyaltyService;
use crate::features::discounts::services::DiscountService;
use crate::features::discounts::dto::CheckLineItem;

/// GET /api/orders - Get all orders with optional filtering
#[utoipa::path(
    get,
    path = "/orders",
    tag = "orders",
    params(OrderQueryParams),
    responses(
        (status = 200, description = "Orders retrieved successfully", body = inline(SuccessResponse<Vec<OrderResponse>>)),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn get_orders(
    Query(params): Query<OrderQueryParams>,
    Extension(pool): Extension<PgPool>,
) -> Response {
    match OrderService::get_orders(&pool, &params).await {
        Ok(orders) => (
            StatusCode::OK,
            Json(SuccessResponse::new(orders, "Orders retrieved successfully".to_string()))
        ).into_response(),
        Err(e) => {
            eprintln!("Database error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    error_codes::DATABASE_ERROR.to_string(),
                    "Erreur lors de la récupération des commandes".to_string()
                ))
            ).into_response()
        }
    }
}

/// POST /api/orders - Create new order with line items
#[utoipa::path(
    post,
    path = "/orders",
    tag = "orders",
    request_body = CreateOrderRequest,
    responses(
        (status = 201, description = "Order created successfully", body = inline(SuccessResponse<OrderResponse>)),
        (status = 404, description = "User or product not found", body = ErrorResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn create_order(
    Extension(pool): Extension<PgPool>,
    Extension(claims): Extension<Claims>,
    Json(request): Json<CreateOrderRequest>,
) -> Response {
    // Verify user exists
    match OrderService::user_exists(&pool, request.user_id).await {
        Ok(false) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    error_codes::NOT_FOUND.to_string(),
                    format!("Utilisateur {} non trouvé", request.user_id)
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

    // Start transaction
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            eprintln!("Transaction error: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    error_codes::DATABASE_ERROR.to_string(),
                    "Erreur de transaction".to_string()
                ))
            ).into_response();
        }
    };

    // Calculate total amount, verify all products exist and have sufficient stock
    let mut total_amount = Decimal::new(0, 0);

    for line_item in &request.line_items {
        // Get product price AND stock quantity
        let price_query = match sqlx::query(
            "SELECT pp.price_prp, p.buying_price_pro, p.stock_quantity_pro
             FROM products_pro p
             LEFT JOIN productprices_prp pp ON p.id_pro = pp.product_ref_prp
             WHERE p.id_pro = $1"
        )
        .bind(line_item.product_id)
        .fetch_optional(&mut *tx)
        .await {
            Ok(Some(row)) => row,
            Ok(None) => {
                return (
                    StatusCode::NOT_FOUND,
                    Json(ErrorResponse::new(
                        error_codes::NOT_FOUND.to_string(),
                        format!("Produit {} non trouvé", line_item.product_id)
                    ))
                ).into_response();
            }
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
        };

        // Check sufficient stock before proceeding
        let stock_quantity: i32 = price_query.get("stock_quantity_pro");
        if stock_quantity < line_item.quantity {
            return (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(ErrorResponse::new(
                    "INSUFFICIENT_STOCK".to_string(),
                    format!(
                        "Stock insuffisant pour le produit {} : disponible {}, demandé {}",
                        line_item.product_id, stock_quantity, line_item.quantity
                    )
                ))
            ).into_response();
        }

        let unit_price: Decimal = match price_query.try_get("price_prp") {
            Ok(price) => price,
            Err(_) => price_query.get("buying_price_pro"),
        };

        let line_total = unit_price * Decimal::new(line_item.quantity as i64, 0);
        total_amount += line_total;
    }

    // Build check_items with real line totals for discount evaluation
    let check_items: Vec<CheckLineItem> = {
        let mut items = Vec::new();
        for li in &request.line_items {
            let row = sqlx::query(
                "SELECT COALESCE(pp.price_prp, p.buying_price_pro) as unit_price
                 FROM products_pro p
                 LEFT JOIN productprices_prp pp ON p.id_pro = pp.product_ref_prp
                 WHERE p.id_pro = $1"
            )
            .bind(li.product_id)
            .fetch_optional(&mut *tx)
            .await;
            let unit_price: Decimal = match row {
                Ok(Some(r)) => r.get("unit_price"),
                _ => Decimal::ZERO,
            };
            items.push(CheckLineItem {
                product_id: li.product_id,
                quantity: li.quantity,
                line_total: unit_price * Decimal::new(li.quantity as i64, 0),
            });
        }
        items
    };

    let (discount_amount, applied_discounts) = match &request.discount_ids {
        Some(ids) if !ids.is_empty() => {
            match DiscountService::compute_order_savings(&pool, ids, &check_items, total_amount).await {
                Ok(result) => result,
                Err(e) => {
                    eprintln!("Discount computation error (non-critical): {e}");
                    (Decimal::ZERO, Vec::new())
                }
            }
        }
        _ => (Decimal::ZERO, Vec::new()),
    };

    let final_amount = total_amount - discount_amount;

    // Create the order
    let order_row = match sqlx::query(
        "INSERT INTO order_ord (user_id_ord, staff_id_ord, order_date_ord, status_ord, amount_ord, discount_amount_ord)
         VALUES ($1, $2, $3, $4, $5, $6)
         RETURNING id_ord, user_id_ord, staff_id_ord, order_date_ord, status_ord, amount_ord, discount_amount_ord, created_at, updated_at"
    )
    .bind(request.user_id)
    .bind(claims.staff_id)
    .bind(Utc::now())
    .bind(&request.status)
    .bind(final_amount)
    .bind(discount_amount)
    .fetch_one(&mut *tx)
    .await {
        Ok(row) => row,
        Err(e) => {
            eprintln!("Database error: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    error_codes::DATABASE_ERROR.to_string(),
                    "Erreur lors de la création de la commande".to_string()
                ))
            ).into_response();
        }
    };

    let order_id: i32 = order_row.get("id_ord");

    // Create line items
    for line_item in &request.line_items {
        // Get product price again
        let product_query = match sqlx::query(
            "SELECT pp.price_prp, p.buying_price_pro
             FROM products_pro p
             LEFT JOIN productprices_prp pp ON p.id_pro = pp.product_ref_prp
             WHERE p.id_pro = $1"
        )
        .bind(line_item.product_id)
        .fetch_one(&mut *tx)
        .await {
            Ok(row) => row,
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
        };

        let unit_price: Decimal = match product_query.try_get("price_prp") {
            Ok(price) => price,
            Err(_) => product_query.get("buying_price_pro"),
        };

        let line_total = unit_price * Decimal::new(line_item.quantity as i64, 0);

        if let Err(e) = sqlx::query(
            "INSERT INTO line_order_lor (order_id_lor, product_id_lor, quantity_lor, line_total_lor)
             VALUES ($1, $2, $3, $4)"
        )
        .bind(order_id)
        .bind(line_item.product_id)
        .bind(line_item.quantity)
        .bind(line_total)
        .execute(&mut *tx)
        .await {
            eprintln!("Database error: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    error_codes::DATABASE_ERROR.to_string(),
                    "Erreur lors de la création des lignes de commande".to_string()
                ))
            ).into_response();
        }

        // Decrement stock quantity atomically in the same transaction
        // Also update status to 'out_of_stock' if stock reaches 0
        if let Err(e) = sqlx::query(
            "UPDATE products_pro
             SET
                stock_quantity_pro = stock_quantity_pro - $1,
                status_pro = CASE
                    WHEN (stock_quantity_pro - $1) <= 0 THEN 'out_of_stock'::product_status_enum
                    ELSE status_pro
                END,
                updated_at = NOW()
             WHERE id_pro = $2"
        )
        .bind(line_item.quantity)
        .bind(line_item.product_id)
        .execute(&mut *tx)
        .await {
            eprintln!("Stock update error: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    error_codes::DATABASE_ERROR.to_string(),
                    "Erreur lors de la mise à jour du stock".to_string()
                ))
            ).into_response();
        }
    }

    // Record applied discounts in junction table
    for applied in &applied_discounts {
        if let Err(e) = sqlx::query(
            "INSERT INTO order_discounts_odc (order_id_odc, discount_id_odc, saving_amount_odc)
             VALUES ($1, $2, $3)
             ON CONFLICT DO NOTHING"
        )
        .bind(order_id)
        .bind(applied.id)
        .bind(applied.saving_amount)
        .execute(&mut *tx)
        .await {
            eprintln!("Discount junction insert error (non-critical): {e}");
        }
    }

    if let Err(e) = tx.commit().await {
        eprintln!("Transaction commit error: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                error_codes::DATABASE_ERROR.to_string(),
                "Erreur lors de la validation de la commande".to_string()
            ))
        ).into_response();
    }

    if let Err(e) = LoyaltyService::award_points(&pool, request.user_id, order_id, final_amount).await {
        eprintln!("Loyalty points error (non-critical): {}", e);
    }

    let created_order = OrderResponse::from_row(&order_row);
    (
        StatusCode::CREATED,
        Json(SuccessResponse::new(created_order, "Order created successfully".to_string()))
    ).into_response()
}

/// GET /api/orders/:id - Get order by ID
#[utoipa::path(
    get,
    path = "/orders/{id}",
    tag = "orders",
    params(
        ("id" = i32, Path, description = "Order ID")
    ),
    responses(
        (status = 200, description = "Order retrieved successfully", body = inline(SuccessResponse<OrderResponse>)),
        (status = 404, description = "Order not found", body = ErrorResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn get_order_by_id(
    Path((_commerce_id, id)): Path<(String, i32)>,
    Extension(pool): Extension<PgPool>,
) -> Response {
    match OrderService::get_order_by_id(&pool, id).await {
        Ok(Some(order)) => (
            StatusCode::OK,
            Json(SuccessResponse::new(Some(order), "Order retrieved successfully".to_string()))
        ).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                error_codes::NOT_FOUND.to_string(),
                format!("Commande {} non trouvée", id)
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

/// PUT /api/orders/:id - Update order status
#[utoipa::path(
    put,
    path = "/orders/{id}",
    tag = "orders",
    params(
        ("id" = i32, Path, description = "Order ID")
    ),
    request_body = UpdateOrderRequest,
    responses(
        (status = 200, description = "Order updated successfully", body = inline(SuccessResponse<OrderResponse>)),
        (status = 404, description = "Order not found", body = ErrorResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn update_order(
    Path((_commerce_id, id)): Path<(String, i32)>,
    Extension(pool): Extension<PgPool>,
    Json(request): Json<UpdateOrderRequest>,
) -> Response {
    match OrderService::update_order_status(&pool, id, &request.status).await {
        Ok(Some(order)) => (
            StatusCode::OK,
            Json(SuccessResponse::new(order, "Order updated successfully".to_string()))
        ).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                error_codes::NOT_FOUND.to_string(),
                format!("Commande {} non trouvée", id)
            ))
        ).into_response(),
        Err(e) => {
            eprintln!("Database error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    error_codes::DATABASE_ERROR.to_string(),
                    "Erreur lors de la mise à jour de la commande".to_string()
                ))
            ).into_response()
        }
    }
}

/// DELETE /api/orders/:id - Delete order and its line items
#[utoipa::path(
    delete,
    path = "/orders/{id}",
    tag = "orders",
    params(
        ("id" = i32, Path, description = "Order ID")
    ),
    responses(
        (status = 200, description = "Order deleted successfully", body = inline(SuccessResponse<()>)),
        (status = 404, description = "Order not found", body = ErrorResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn delete_order(
    Path((_commerce_id, id)): Path<(String, i32)>,
    Extension(pool): Extension<PgPool>,
) -> Response {
    // Check if order exists
    match OrderService::order_exists(&pool, id).await {
        Ok(false) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    error_codes::NOT_FOUND.to_string(),
                    format!("Commande {} non trouvée", id)
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

    match OrderService::delete_order(&pool, id).await {
        Ok(rows_affected) => {
            if rows_affected == 0 {
                (
                    StatusCode::NOT_FOUND,
                    Json(ErrorResponse::new(
                        error_codes::NOT_FOUND.to_string(),
                        format!("Commande {} non trouvée", id)
                    ))
                ).into_response()
            } else {
                (
                    StatusCode::OK,
                    Json(SuccessResponse::new((), format!("Commande {} supprimée avec succès", id)))
                ).into_response()
            }
        }
        Err(e) => {
            eprintln!("Database error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    error_codes::DATABASE_ERROR.to_string(),
                    "Erreur lors de la suppression de la commande".to_string()
                ))
            ).into_response()
        }
    }
}

/// GET /api/orders/:id/items - Get line items for an order
#[utoipa::path(
    get,
    path = "/orders/{id}/items",
    tag = "orders",
    params(
        ("id" = i32, Path, description = "Order ID")
    ),
    responses(
        (status = 200, description = "Line items retrieved successfully", body = inline(SuccessResponse<Vec<LineItemResponse>>)),
        (status = 404, description = "Order not found", body = ErrorResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn get_order_items(
    Path((_commerce_id, id)): Path<(String, i32)>,
    Extension(pool): Extension<PgPool>,
) -> Response {
    // Check if order exists
    match OrderService::order_exists(&pool, id).await {
        Ok(false) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    error_codes::NOT_FOUND.to_string(),
                    format!("Commande {} non trouvée", id)
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

    match OrderService::get_order_items(&pool, id).await {
        Ok(line_items) => (
            StatusCode::OK,
            Json(SuccessResponse::new(line_items, "Line items retrieved successfully".to_string()))
        ).into_response(),
        Err(e) => {
            eprintln!("Database error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    error_codes::DATABASE_ERROR.to_string(),
                    "Erreur lors de la récupération des lignes".to_string()
                ))
            ).into_response()
        }
    }
}

/// GET /api/orders/user/:user_id - Get all orders for a specific user
#[utoipa::path(
    get,
    path = "/orders/user/{user_id}",
    tag = "orders",
    params(
        ("user_id" = i32, Path, description = "User ID")
    ),
    responses(
        (status = 200, description = "Orders retrieved successfully", body = inline(SuccessResponse<Vec<OrderResponse>>)),
        (status = 404, description = "User not found", body = ErrorResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn get_orders_by_user(
    Path((_commerce_id, user_id)): Path<(String, i32)>,
    Extension(pool): Extension<PgPool>,
) -> Response {
    // Check if user exists
    match OrderService::user_exists(&pool, user_id).await {
        Ok(false) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    error_codes::NOT_FOUND.to_string(),
                    format!("Utilisateur {} non trouvé", user_id)
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

    match OrderService::get_orders_by_user(&pool, user_id).await {
        Ok(orders) => (
            StatusCode::OK,
            Json(SuccessResponse::new(orders, "Orders retrieved successfully".to_string()))
        ).into_response(),
        Err(e) => {
            eprintln!("Database error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    error_codes::DATABASE_ERROR.to_string(),
                    "Erreur lors de la récupération des commandes".to_string()
                ))
            ).into_response()
        }
    }
}

/// GET /api/orders/stats - Get order statistics
#[utoipa::path(
    get,
    path = "/orders/stats",
    tag = "orders",
    responses(
        (status = 200, description = "Order statistics retrieved successfully", body = inline(SuccessResponse<OrderStatsResponse>)),
        (status = 500, description = "Database error", body = ErrorResponse)
    )
)]
pub async fn get_order_stats(
    Extension(pool): Extension<PgPool>,
) -> Response {
    match OrderService::get_order_stats(&pool).await {
        Ok(stats) => (
            StatusCode::OK,
            Json(SuccessResponse::new(stats, "Order statistics retrieved successfully".to_string()))
        ).into_response(),
        Err(e) => {
            eprintln!("Database error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    error_codes::DATABASE_ERROR.to_string(),
                    "Erreur lors de la récupération des statistiques".to_string()
                ))
            ).into_response()
        }
    }
}
