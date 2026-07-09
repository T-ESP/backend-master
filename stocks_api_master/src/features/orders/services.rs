use sqlx::{PgPool, Row, Postgres};
use sqlx::postgres::PgRow;
use rust_decimal::Decimal;
use chrono::Utc;

use super::dto::{OrderResponse, LineItemResponse, OrderStatsResponse, OrderQueryParams, PaginatedOrdersResponse};

/// Plafond par requête. La page commandes n'en demande que 100 au plus, mais les écrans
/// de statistiques parcourent l'historique complet par lots de 1000.
const MAX_LIMIT: i64 = 1000;
const DEFAULT_LIMIT: i64 = 50;

/// Traduit `sort_by` en colonne SQL. Toute valeur inconnue retombe sur `created_at` :
/// la valeur ne doit jamais être interpolée telle quelle sous peine d'injection.
fn sort_column(sort_by: Option<&str>) -> &'static str {
    match sort_by {
        Some("date") => "order_date_ord",
        Some("amount") => "amount_ord",
        Some("status") => "status_ord",
        Some("user") => "user_id_ord",
        _ => "created_at",
    }
}

fn sort_direction(sort_order: Option<&str>) -> &'static str {
    match sort_order {
        Some("asc") => "ASC",
        _ => "DESC",
    }
}

/// Construit la clause WHERE partagée par le comptage et la requête de lignes.
/// L'ordre des placeholders doit correspondre exactement à celui de `bind_filters`.
fn build_where_clause(params: &OrderQueryParams, search_pattern: Option<&str>) -> String {
    let mut conditions: Vec<String> = Vec::new();
    let mut bind_count = 0;

    if params.user_id.is_some() {
        bind_count += 1;
        conditions.push(format!("user_id_ord = ${}", bind_count));
    }

    if params.status.is_some() {
        bind_count += 1;
        conditions.push(format!("status_ord = ${}", bind_count));
    }

    if search_pattern.is_some() {
        bind_count += 1;
        conditions.push(format!(
            "(CAST(id_ord AS TEXT) ILIKE ${n} OR CAST(user_id_ord AS TEXT) ILIKE ${n} OR status_ord ILIKE ${n})",
            n = bind_count
        ));
    }

    if params.min_amount.is_some() {
        bind_count += 1;
        conditions.push(format!("amount_ord >= ${}", bind_count));
    }

    if params.max_amount.is_some() {
        bind_count += 1;
        conditions.push(format!("amount_ord <= ${}", bind_count));
    }

    if params.date_from.is_some() {
        bind_count += 1;
        conditions.push(format!("order_date_ord >= ${}", bind_count));
    }

    if params.date_until.is_some() {
        bind_count += 1;
        conditions.push(format!("order_date_ord <= ${}", bind_count));
    }

    if conditions.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", conditions.join(" AND "))
    }
}

/// Nombre de valeurs bindées par `bind_filters`. Le placeholder de recherche est répété
/// trois fois dans la clause WHERE mais ne compte que pour un bind.
fn filter_bind_count(params: &OrderQueryParams, search_pattern: Option<&str>) -> usize {
    [
        params.user_id.is_some(),
        params.status.is_some(),
        search_pattern.is_some(),
        params.min_amount.is_some(),
        params.max_amount.is_some(),
        params.date_from.is_some(),
        params.date_until.is_some(),
    ]
    .iter()
    .filter(|present| **present)
    .count()
}

/// Applique les binds dans le même ordre que `build_where_clause` déclare les placeholders.
fn bind_filters<'q>(
    mut query: sqlx::query::Query<'q, Postgres, sqlx::postgres::PgArguments>,
    params: &'q OrderQueryParams,
    search_pattern: Option<&'q str>,
) -> sqlx::query::Query<'q, Postgres, sqlx::postgres::PgArguments> {
    if let Some(user_id) = params.user_id {
        query = query.bind(user_id);
    }
    if let Some(ref status) = params.status {
        query = query.bind(status.as_str());
    }
    if let Some(pattern) = search_pattern {
        query = query.bind(pattern);
    }
    if let Some(min_amount) = params.min_amount {
        query = query.bind(min_amount);
    }
    if let Some(max_amount) = params.max_amount {
        query = query.bind(max_amount);
    }
    if let Some(date_from) = params.date_from {
        query = query.bind(date_from);
    }
    if let Some(date_until) = params.date_until {
        query = query.bind(date_until);
    }
    query
}

pub struct OrderService;

impl OrderService {
    /// Renvoie une page de commandes et le nombre total de lignes correspondant aux filtres.
    pub async fn get_orders(pool: &PgPool, params: &OrderQueryParams) -> Result<PaginatedOrdersResponse, sqlx::Error> {
        let limit = params.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
        let offset = params.offset.unwrap_or(0).max(0);

        let search_pattern = params
            .search
            .as_ref()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| format!("%{}%", s));
        let search_pattern = search_pattern.as_deref();

        let where_clause = build_where_clause(params, search_pattern);

        let count_sql = format!("SELECT COUNT(*) FROM order_ord{}", where_clause);
        let total: i64 = bind_filters(sqlx::query(&count_sql), params, search_pattern)
            .fetch_one(pool)
            .await?
            .get(0);

        // `id_ord` en clé de tri secondaire : sans elle, deux lignes de même date peuvent
        // changer d'ordre entre deux pages et une commande serait sautée ou dupliquée.
        let mut rows_sql = format!(
            "SELECT id_ord, user_id_ord, staff_id_ord, order_date_ord, status_ord, amount_ord, discount_amount_ord, created_at, updated_at FROM order_ord{}",
            where_clause
        );
        rows_sql.push_str(&format!(
            " ORDER BY {} {}, id_ord DESC",
            sort_column(params.sort_by.as_deref()),
            sort_direction(params.sort_order.as_deref())
        ));

        let bind_count = filter_bind_count(params, search_pattern);
        rows_sql.push_str(&format!(" LIMIT ${} OFFSET ${}", bind_count + 1, bind_count + 2));

        let rows = bind_filters(sqlx::query(&rows_sql), params, search_pattern)
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await?;

        let orders: Vec<OrderResponse> = rows.iter()
            .map(|row| OrderResponse::from_row(row))
            .collect();

        Ok(PaginatedOrdersResponse {
            items: orders,
            total,
            limit,
            offset,
        })
    }

    pub async fn get_order_by_id(pool: &PgPool, id: i32) -> Result<Option<OrderResponse>, sqlx::Error> {
        let row = sqlx::query(
            "SELECT id_ord, user_id_ord, staff_id_ord, order_date_ord, status_ord, amount_ord, discount_amount_ord, created_at, updated_at
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
             RETURNING id_ord, user_id_ord, staff_id_ord, order_date_ord, status_ord, amount_ord, discount_amount_ord, created_at, updated_at"
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
            "SELECT id_ord, user_id_ord, staff_id_ord, order_date_ord, status_ord, amount_ord, discount_amount_ord, created_at, updated_at
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

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use std::collections::HashSet;

    fn params() -> OrderQueryParams {
        OrderQueryParams {
            limit: None,
            offset: None,
            user_id: None,
            status: None,
            search: None,
            min_amount: None,
            max_amount: None,
            date_from: None,
            date_until: None,
            sort_by: None,
            sort_order: None,
        }
    }

    /// Compte les placeholders distincts ($1, $2, ...) présents dans la clause.
    fn distinct_placeholders(clause: &str) -> usize {
        clause
            .match_indices('$')
            .map(|(i, _)| {
                clause[i + 1..]
                    .chars()
                    .take_while(|c| c.is_ascii_digit())
                    .collect::<String>()
            })
            .collect::<HashSet<_>>()
            .len()
    }

    /// L'invariant qui casse silencieusement tout : autant de placeholders distincts
    /// dans le WHERE que de valeurs bindées, sinon les filtres glissent d'une position.
    #[test]
    fn where_clause_placeholders_match_bind_count() {
        let mut p = params();
        p.user_id = Some(1);
        p.status = Some("pending".to_string());
        p.min_amount = Some(Decimal::from(10));
        p.max_amount = Some(Decimal::from(500));
        p.date_from = Some(Utc::now());
        p.date_until = Some(Utc::now());

        let pattern = Some("%42%");
        let clause = build_where_clause(&p, pattern);

        assert_eq!(distinct_placeholders(&clause), filter_bind_count(&p, pattern));
        assert_eq!(filter_bind_count(&p, pattern), 7);
    }

    #[test]
    fn where_clause_is_empty_without_filters() {
        let p = params();
        assert_eq!(build_where_clause(&p, None), "");
        assert_eq!(filter_bind_count(&p, None), 0);
    }

    /// La recherche occupe 3 fois le même placeholder mais ne compte que pour un bind.
    #[test]
    fn search_reuses_a_single_placeholder() {
        let p = params();
        let pattern = Some("%abc%");
        let clause = build_where_clause(&p, pattern);

        assert_eq!(clause.matches("$1").count(), 3);
        assert_eq!(distinct_placeholders(&clause), 1);
        assert_eq!(filter_bind_count(&p, pattern), 1);
    }

    /// Les placeholders doivent se suivre sans trou quand seuls certains filtres sont posés.
    #[test]
    fn placeholders_are_numbered_consecutively() {
        let mut p = params();
        p.status = Some("shipped".to_string());
        p.date_until = Some(Utc::now());

        let clause = build_where_clause(&p, None);
        assert!(clause.contains("status_ord = $1"));
        assert!(clause.contains("order_date_ord <= $2"));
        assert_eq!(filter_bind_count(&p, None), 2);
    }

    #[test]
    fn sort_column_rejects_unknown_input() {
        assert_eq!(sort_column(Some("amount")), "amount_ord");
        assert_eq!(sort_column(Some("user")), "user_id_ord");
        assert_eq!(sort_column(None), "created_at");
        // Une tentative d'injection retombe sur la colonne par défaut.
        assert_eq!(sort_column(Some("amount_ord; DROP TABLE order_ord --")), "created_at");
    }

    #[test]
    fn sort_direction_defaults_to_desc() {
        assert_eq!(sort_direction(Some("asc")), "ASC");
        assert_eq!(sort_direction(Some("desc")), "DESC");
        assert_eq!(sort_direction(Some("; DROP TABLE")), "DESC");
        assert_eq!(sort_direction(None), "DESC");
    }
}