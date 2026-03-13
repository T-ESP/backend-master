use sqlx::{PgPool, Row, Postgres};
use sqlx::postgres::PgRow;
use rust_decimal::Decimal;
use chrono::Utc;

use super::dto::{OrderResponse, LineItemResponse, OrderStatsResponse, OrderQueryParams};

pub struct OrderService;

impl OrderService {
    pub async fn get_orders(pool: &PgPool, params: &OrderQueryParams) -> Result<Vec<OrderResponse>, sqlx::Error> {
        let limit = params.limit.unwrap_or(50).min(100);
        let offset = params.offset.unwrap_or(0);

        let mut query = "SELECT id_ord, user_id_ord, order_date_ord, status_ord, amount_ord, created_at, updated_at FROM order_ord".to_string();
        let mut conditions = Vec::new();
        let mut bind_count = 0;

        // Add filters
        if params.user_id.is_some() {
            bind_count += 1;
            conditions.push(format!("user_id_ord = ${}", bind_count));
        }

        if params.status.is_some() {
            bind_count += 1;
            conditions.push(format!("status_ord = ${}", bind_count));
        }

        if !conditions.is_empty() {
            query.push_str(&format!(" WHERE {}", conditions.join(" AND ")));
        }

        bind_count += 1;
        query.push_str(&format!(" ORDER BY created_at DESC LIMIT ${}", bind_count));
        bind_count += 1;
        query.push_str(&format!(" OFFSET ${}", bind_count));

        let mut sql_query = sqlx::query(&query);

        if let Some(user_id) = params.user_id {
            sql_query = sql_query.bind(user_id);
        }

        if let Some(ref status) = params.status {
            sql_query = sql_query.bind(status);
        }

        sql_query = sql_query.bind(limit).bind(offset);

        let rows = sql_query.fetch_all(pool).await?;

        let orders: Vec<OrderResponse> = rows.iter()
            .map(|row| OrderResponse::from_row(row))
            .collect();

        Ok(orders)
    }

    pub async fn get_order_by_id(pool: &PgPool, id: i32) -> Result<Option<OrderResponse>, sqlx::Error> {
        let row = sqlx::query(
            "SELECT id_ord, user_id_ord, order_date_ord, status_ord, amount_ord, created_at, updated_at 
             FROM order_ord 
             WHERE id_ord = $1"
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;

        Ok(row.map(|row| OrderResponse::from_row(&row)))
    }

    pub async fn update_order_status(pool: &PgPool, id: i32, status: &str) -> Result<Option<OrderResponse>, sqlx::Error> {
        let row = sqlx::query(
            "UPDATE order_ord 
             SET status_ord = $1, updated_at = NOW() 
             WHERE id_ord = $2 
             RETURNING id_ord, user_id_ord, order_date_ord, status_ord, amount_ord, created_at, updated_at"
        )
        .bind(status)
        .bind(id)
        .fetch_optional(pool)
        .await?;

        Ok(row.map(|row| OrderResponse::from_row(&row)))
    }

    pub async fn delete_order(pool: &PgPool, id: i32) -> Result<u64, sqlx::Error> {
        let mut tx = pool.begin().await?;

        // Delete line items first
        sqlx::query("DELETE FROM line_order_lor WHERE order_id_lor = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;

        // Delete the order
        let result = sqlx::query("DELETE FROM order_ord WHERE id_ord = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;

        Ok(result.rows_affected())
    }

    pub async fn get_order_items(pool: &PgPool, id: i32) -> Result<Vec<LineItemResponse>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT l.id_lor, l.order_id_lor, l.product_id_lor, l.quantity_lor, l.line_total_lor, 
                    l.created_at, l.updated_at, p.name_pro as product_name
             FROM line_order_lor l
             JOIN products_pro p ON l.product_id_lor = p.id_pro
             WHERE l.order_id_lor = $1
             ORDER BY l.created_at"
        )
        .bind(id)
        .fetch_all(pool)
        .await?;

        let line_items: Vec<LineItemResponse> = rows.iter()
            .map(|row| LineItemResponse::from_row(row))
            .collect();

        Ok(line_items)
    }

    pub async fn get_orders_by_user(pool: &PgPool, user_id: i32) -> Result<Vec<OrderResponse>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT id_ord, user_id_ord, order_date_ord, status_ord, amount_ord, created_at, updated_at 
             FROM order_ord 
             WHERE user_id_ord = $1 
             ORDER BY created_at DESC"
        )
        .bind(user_id)
        .fetch_all(pool)
        .await?;

        let orders: Vec<OrderResponse> = rows.iter()
            .map(|row| OrderResponse::from_row(row))
            .collect();

        Ok(orders)
    }

    pub async fn get_order_stats(pool: &PgPool) -> Result<OrderStatsResponse, sqlx::Error> {
        let row = sqlx::query(
            "SELECT 
                COUNT(*) as total_orders,
                COUNT(CASE WHEN status_ord = 'pending' THEN 1 END) as pending_orders,
                COUNT(CASE WHEN status_ord = 'confirmed' THEN 1 END) as confirmed_orders,
                COUNT(CASE WHEN status_ord = 'shipped' THEN 1 END) as shipped_orders,
                COUNT(CASE WHEN status_ord = 'delivered' THEN 1 END) as delivered_orders,
                COUNT(CASE WHEN status_ord = 'cancelled' THEN 1 END) as cancelled_orders,
                COALESCE(SUM(amount_ord), 0) as total_amount,
                COALESCE(AVG(amount_ord), 0) as avg_order_value
             FROM order_ord"
        )
        .fetch_one(pool)
        .await?;

        Ok(OrderStatsResponse {
            total_orders: row.get("total_orders"),
            pending_orders: row.get("pending_orders"),
            confirmed_orders: row.get("confirmed_orders"),
            shipped_orders: row.get("shipped_orders"),
            delivered_orders: row.get("delivered_orders"),
            cancelled_orders: row.get("cancelled_orders"),
            total_amount: row.get("total_amount"),
            avg_order_value: row.get("avg_order_value"),
        })
    }

    pub async fn user_exists(pool: &PgPool, user_id: i32) -> Result<bool, sqlx::Error> {
        let exists = sqlx::query("SELECT id_usr FROM users_usr WHERE id_usr = $1")
            .bind(user_id)
            .fetch_optional(pool)
            .await?;

        Ok(exists.is_some())
    }

    pub async fn order_exists(pool: &PgPool, order_id: i32) -> Result<bool, sqlx::Error> {
        let exists = sqlx::query("SELECT id_ord FROM order_ord WHERE id_ord = $1")
            .bind(order_id)
            .fetch_optional(pool)
            .await?;

        Ok(exists.is_some())
    }
}