use sqlx::PgPool;
use sqlx::Row;
use std::collections::HashMap;
use super::dto::{
    RestockResponse, RestockWithSupplierResponse, RestockStatsResponse,
    CreateRestockRequest, UpdateRestockRequest, RestockSearchParams, LineRestockResponse,
    LineRestockWithSupplierResponse, RestockStatus
};

pub struct RestockService;

impl RestockService {
    /// Get all restocks with optional filters and pagination
    pub async fn get_restocks(
        pool: &PgPool,
        params: &RestockSearchParams,
    ) -> Result<Vec<RestockResponse>, sqlx::Error> {
        let limit = params.limit.unwrap_or(50).min(500);
        let offset = params.offset.unwrap_or(0).max(0);

        // Build the WHERE conditions for filtering
        let mut conditions: Vec<String> = vec![];

        if let Some(product_id) = params.product_id {
            conditions.push(format!("lr.product_id_lrs = {}", product_id));
        }

        if let Some(min_qty) = params.min_quantity {
            conditions.push(format!("r.quantity_res >= {}", min_qty));
        }

        if let Some(max_qty) = params.max_quantity {
            conditions.push(format!("r.quantity_res <= {}", max_qty));
        }

        if let Some(from_date) = params.from_date {
            conditions.push(format!("r.restock_date_res >= '{}'", from_date.to_rfc3339()));
        }

        if let Some(to_date) = params.to_date {
            conditions.push(format!("r.restock_date_res <= '{}'", to_date.to_rfc3339()));
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        // Get restocks with their lines
        let query = format!(
            "SELECT
                r.id_res,
                r.quantity_res,
                r.supplier_id_res,
                r.status_res,
                r.restock_date_res,
                r.created_at,
                r.updated_at,
                lr.product_id_lrs,
                p.name_pro as product_name,
                p.reference_pro as product_reference,
                lr.quantity_lrs,
                lr.unit_price_lrs,
                lr.total_price_lrs
            FROM restock_res r
            LEFT JOIN line_restock_lrs lr ON lr.restock_id_lrs = r.id_res
            LEFT JOIN products_pro p ON p.id_pro = lr.product_id_lrs
            {}
            ORDER BY r.restock_date_res DESC, r.id_res, lr.product_id_lrs
            LIMIT {} OFFSET {}",
            where_clause, limit, offset
        );

        let rows = sqlx::query(&query).fetch_all(pool).await?;

        // Group lines by restock
        let mut restocks_map: HashMap<i32, RestockResponse> = HashMap::new();

        for row in rows {
            let restock_id: i32 = row.get("id_res");

            let restock = restocks_map.entry(restock_id).or_insert_with(|| {
                let status_str: String = row.get("status_res");
                RestockResponse {
                    id: restock_id,
                    quantity: row.get("quantity_res"),
                    supplier_id: row.get("supplier_id_res"),
                    status: RestockStatus::from_str(&status_str).unwrap_or(RestockStatus::Pending),
                    restock_date: row.get("restock_date_res"),
                    created_at: row.get("created_at"),
                    updated_at: row.get("updated_at"),
                    lines: Vec::new(),
                }
            });

            // Add line if it exists (LEFT JOIN might return NULL)
            if let Ok(product_id) = row.try_get::<i32, _>("product_id_lrs") {
                restock.lines.push(LineRestockResponse {
                    product_id,
                    product_name: row.get("product_name"),
                    product_reference: row.get("product_reference"),
                    quantity: row.get("quantity_lrs"),
                    unit_price: row.get("unit_price_lrs"),
                    total_price: row.get("total_price_lrs"),
                });
            }
        }

        Ok(restocks_map.into_values().collect())
    }

    /// Get restock by ID
    pub async fn get_restock_by_id(
        pool: &PgPool,
        id: i32,
    ) -> Result<RestockResponse, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT
                r.id_res,
                r.quantity_res,
                r.supplier_id_res,
                r.status_res,
                r.restock_date_res,
                r.created_at,
                r.updated_at,
                lr.product_id_lrs,
                p.name_pro as product_name,
                p.reference_pro as product_reference,
                lr.quantity_lrs,
                lr.unit_price_lrs,
                lr.total_price_lrs
            FROM restock_res r
            LEFT JOIN line_restock_lrs lr ON lr.restock_id_lrs = r.id_res
            LEFT JOIN products_pro p ON p.id_pro = lr.product_id_lrs
            WHERE r.id_res = $1
            ORDER BY lr.product_id_lrs"
        )
        .bind(id)
        .fetch_all(pool)
        .await?;

        if rows.is_empty() {
            return Err(sqlx::Error::RowNotFound);
        }

        let first_row = &rows[0];
        let status_str: String = first_row.get("status_res");
        let mut restock = RestockResponse {
            id: first_row.get("id_res"),
            quantity: first_row.get("quantity_res"),
            supplier_id: first_row.get("supplier_id_res"),
            status: RestockStatus::from_str(&status_str).unwrap_or(RestockStatus::Pending),
            restock_date: first_row.get("restock_date_res"),
            created_at: first_row.get("created_at"),
            updated_at: first_row.get("updated_at"),
            lines: Vec::new(),
        };

        for row in rows {
            if let Ok(product_id) = row.try_get::<i32, _>("product_id_lrs") {
                restock.lines.push(LineRestockResponse {
                    product_id,
                    product_name: row.get("product_name"),
                    product_reference: row.get("product_reference"),
                    quantity: row.get("quantity_lrs"),
                    unit_price: row.get("unit_price_lrs"),
                    total_price: row.get("total_price_lrs"),
                });
            }
        }

        Ok(restock)
    }

    /// Get restocks with supplier information
    pub async fn get_restocks_with_supplier(
        pool: &PgPool,
        params: &RestockSearchParams,
    ) -> Result<Vec<RestockWithSupplierResponse>, sqlx::Error> {
        let limit = params.limit.unwrap_or(50).min(500);
        let offset = params.offset.unwrap_or(0).max(0);

        let mut conditions: Vec<String> = vec![];

        if let Some(product_id) = params.product_id {
            conditions.push(format!("lr.product_id_lrs = {}", product_id));
        }

        if let Some(supplier_id) = params.supplier_id {
            conditions.push(format!("s.id_sup = {}", supplier_id));
        }

        if let Some(min_qty) = params.min_quantity {
            conditions.push(format!("r.quantity_res >= {}", min_qty));
        }

        if let Some(max_qty) = params.max_quantity {
            conditions.push(format!("r.quantity_res <= {}", max_qty));
        }

        if let Some(from_date) = params.from_date {
            conditions.push(format!("r.restock_date_res >= '{}'", from_date.to_rfc3339()));
        }

        if let Some(to_date) = params.to_date {
            conditions.push(format!("r.restock_date_res <= '{}'", to_date.to_rfc3339()));
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        let query = format!(
            "SELECT
                r.id_res,
                r.quantity_res,
                r.supplier_id_res,
                r.status_res,
                r.restock_date_res,
                r.created_at,
                r.updated_at,
                lr.product_id_lrs,
                p.name_pro as product_name,
                p.reference_pro as product_reference,
                lr.quantity_lrs,
                lr.unit_price_lrs,
                lr.total_price_lrs,
                s.id_sup as supplier_id,
                s.name_sup as supplier_name,
                s.email_sup as supplier_email,
                rs.name_sup as restock_supplier_name
            FROM restock_res r
            LEFT JOIN line_restock_lrs lr ON lr.restock_id_lrs = r.id_res
            LEFT JOIN products_pro p ON p.id_pro = lr.product_id_lrs
            LEFT JOIN supplier_sup s ON s.id_sup = p.supplier_id_pro
            LEFT JOIN supplier_sup rs ON rs.id_sup = r.supplier_id_res
            {}
            ORDER BY r.restock_date_res DESC, r.id_res, lr.product_id_lrs
            LIMIT {} OFFSET {}",
            where_clause, limit, offset
        );

        let rows = sqlx::query(&query).fetch_all(pool).await?;

        let mut restocks_map: HashMap<i32, RestockWithSupplierResponse> = HashMap::new();

        for row in rows {
            let restock_id: i32 = row.get("id_res");

            let restock = restocks_map.entry(restock_id).or_insert_with(|| {
                let status_str: String = row.get("status_res");
                RestockWithSupplierResponse {
                    id: restock_id,
                    quantity: row.get("quantity_res"),
                    supplier_id: row.get("supplier_id_res"),
                    supplier_name: row.get("restock_supplier_name"),
                    status: RestockStatus::from_str(&status_str).unwrap_or(RestockStatus::Pending),
                    restock_date: row.get("restock_date_res"),
                    created_at: row.get("created_at"),
                    updated_at: row.get("updated_at"),
                    lines: Vec::new(),
                }
            });

            if let Ok(product_id) = row.try_get::<i32, _>("product_id_lrs") {
                restock.lines.push(LineRestockWithSupplierResponse {
                    product_id,
                    product_name: row.get("product_name"),
                    product_reference: row.get("product_reference"),
                    supplier_id: row.get("supplier_id"),
                    supplier_name: row.get("supplier_name"),
                    supplier_email: row.get("supplier_email"),
                    quantity: row.get("quantity_lrs"),
                    unit_price: row.get("unit_price_lrs"),
                    total_price: row.get("total_price_lrs"),
                });
            }
        }

        Ok(restocks_map.into_values().collect())
    }

    /// Create a new restock
    pub async fn create_restock(
        pool: &PgPool,
        req: &CreateRestockRequest,
    ) -> Result<RestockResponse, sqlx::Error> {
        if req.lines.is_empty() {
            return Err(sqlx::Error::Protocol("Restock must have at least one line".into()));
        }

        let restock_date = req.restock_date.unwrap_or_else(chrono::Utc::now);

        // Calculate total quantity
        let total_quantity: i32 = req.lines.iter().map(|l| l.quantity).sum();

        // Start transaction
        let mut tx = pool.begin().await?;

        // Insert restock header
        let status = req.status.as_ref().unwrap_or(&RestockStatus::Pending);
        let restock_row = sqlx::query(
            "INSERT INTO restock_res (quantity_res, supplier_id_res, status_res, restock_date_res, created_at, updated_at)
            VALUES ($1, $2, $3, $4, NOW(), NOW())
            RETURNING id_res, quantity_res, supplier_id_res, status_res, restock_date_res, created_at, updated_at"
        )
        .bind(total_quantity)
        .bind(req.supplier_id)
        .bind(status.as_str())
        .bind(restock_date)
        .fetch_one(&mut *tx)
        .await?;

        let restock_id: i32 = restock_row.get("id_res");

        let mut lines = Vec::new();

        // Insert each line
        for line_req in &req.lines {
            sqlx::query(
                "INSERT INTO line_restock_lrs (restock_id_lrs, product_id_lrs, quantity_lrs, unit_price_lrs)
                VALUES ($1, $2, $3, $4)"
            )
            .bind(restock_id)
            .bind(line_req.product_id)
            .bind(line_req.quantity)
            .bind(line_req.unit_price)
            .execute(&mut *tx)
            .await?;

            // Also insert into price history
            sqlx::query(
                "INSERT INTO productrestockprices_prr (product_ref_prr, buying_price_prr, restock_id_prr, restock_date_prr, created_at, updated_at)
                VALUES ($1, $2, $3, $4, $4, $4)"
            )
            .bind(line_req.product_id)
            .bind(line_req.unit_price)
            .bind(restock_id)
            .bind(restock_date)
            .execute(&mut *tx)
            .await?;

            // Fetch product details for response
            let product_row = sqlx::query(
                "SELECT name_pro, reference_pro FROM products_pro WHERE id_pro = $1"
            )
            .bind(line_req.product_id)
            .fetch_one(&mut *tx)
            .await?;

            let total_price = line_req.unit_price * rust_decimal::Decimal::from(line_req.quantity);

            lines.push(LineRestockResponse {
                product_id: line_req.product_id,
                product_name: product_row.get("name_pro"),
                product_reference: product_row.get("reference_pro"),
                quantity: line_req.quantity,
                unit_price: line_req.unit_price,
                total_price,
            });
        }

        tx.commit().await?;

        let status_str: String = restock_row.get("status_res");
        Ok(RestockResponse {
            id: restock_id,
            quantity: restock_row.get("quantity_res"),
            supplier_id: restock_row.get("supplier_id_res"),
            status: RestockStatus::from_str(&status_str).unwrap_or(RestockStatus::Pending),
            restock_date: restock_row.get("restock_date_res"),
            created_at: restock_row.get("created_at"),
            updated_at: restock_row.get("updated_at"),
            lines,
        })
    }

    /// Update a restock (status, supplier, date)
    pub async fn update_restock(
        pool: &PgPool,
        id: i32,
        req: &UpdateRestockRequest,
    ) -> Result<RestockResponse, sqlx::Error> {
        // Build dynamic UPDATE query
        let mut updates: Vec<String> = vec![];
        let mut param_count = 1;

        if req.supplier_id.is_some() {
            updates.push(format!("supplier_id_res = ${}", param_count));
            param_count += 1;
        }

        if req.status.is_some() {
            updates.push(format!("status_res = ${}", param_count));
            param_count += 1;
        }

        if req.restock_date.is_some() {
            updates.push(format!("restock_date_res = ${}", param_count));
            param_count += 1;
        }

        if updates.is_empty() {
            // Nothing to update, just return current state
            return Self::get_restock_by_id(pool, id).await;
        }

        updates.push("updated_at = NOW()".to_string());

        let query = format!(
            "UPDATE restock_res SET {} WHERE id_res = ${} RETURNING id_res",
            updates.join(", "),
            param_count
        );

        // Build query with parameters
        let mut query_builder = sqlx::query(&query);

        if let Some(supplier_id) = req.supplier_id {
            query_builder = query_builder.bind(supplier_id);
        }

        if let Some(ref status) = req.status {
            query_builder = query_builder.bind(status.as_str());
        }

        if let Some(restock_date) = req.restock_date {
            query_builder = query_builder.bind(restock_date);
        }

        query_builder = query_builder.bind(id);

        // Execute update
        let result = query_builder.fetch_one(pool).await?;
        let updated_id: i32 = result.get("id_res");

        // Return updated restock with lines
        Self::get_restock_by_id(pool, updated_id).await
    }

    /// Get restock statistics
    pub async fn get_restock_stats(
        pool: &PgPool,
        product_id: Option<i32>,
    ) -> Result<RestockStatsResponse, sqlx::Error> {
        let query = if let Some(pid) = product_id {
            sqlx::query(
                "SELECT
                    COUNT(DISTINCT r.id_res) as total_restocks,
                    COALESCE(SUM(lr.quantity_lrs), 0) as total_quantity,
                    COALESCE(AVG(lr.quantity_lrs), 0) as avg_quantity,
                    MAX(r.restock_date_res) as last_restock_date
                FROM restock_res r
                LEFT JOIN line_restock_lrs lr ON lr.restock_id_lrs = r.id_res
                WHERE lr.product_id_lrs = $1"
            )
            .bind(pid)
            .fetch_one(pool)
            .await?
        } else {
            sqlx::query(
                "SELECT
                    COUNT(*) as total_restocks,
                    COALESCE(SUM(quantity_res), 0) as total_quantity,
                    COALESCE(AVG(quantity_res), 0) as avg_quantity,
                    MAX(restock_date_res) as last_restock_date
                FROM restock_res"
            )
            .fetch_one(pool)
            .await?
        };

        Ok(RestockStatsResponse {
            total_restocks: query.get("total_restocks"),
            total_quantity_restocked: query.get("total_quantity"),
            average_quantity: query.get::<f64, _>("avg_quantity"),
            product_id,
            last_restock_date: query.get("last_restock_date"),
        })
    }

    /// Get restocks for a specific product
    pub async fn get_restocks_by_product(
        pool: &PgPool,
        product_id: i32,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> Result<Vec<RestockResponse>, sqlx::Error> {
        let limit = limit.unwrap_or(50).min(500);
        let offset = offset.unwrap_or(0).max(0);

        let rows = sqlx::query(
            "SELECT
                r.id_res,
                r.quantity_res,
                r.supplier_id_res,
                r.status_res,
                r.restock_date_res,
                r.created_at,
                r.updated_at,
                lr.product_id_lrs,
                p.name_pro as product_name,
                p.reference_pro as product_reference,
                lr.quantity_lrs,
                lr.unit_price_lrs,
                lr.total_price_lrs
            FROM restock_res r
            JOIN line_restock_lrs lr ON lr.restock_id_lrs = r.id_res
            LEFT JOIN products_pro p ON p.id_pro = lr.product_id_lrs
            WHERE lr.product_id_lrs = $1
            ORDER BY r.restock_date_res DESC, lr.product_id_lrs
            LIMIT $2 OFFSET $3"
        )
        .bind(product_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;

        let mut restocks_map: HashMap<i32, RestockResponse> = HashMap::new();

        for row in rows {
            let restock_id: i32 = row.get("id_res");

            let restock = restocks_map.entry(restock_id).or_insert_with(|| {
                let status_str: String = row.get("status_res");
                RestockResponse {
                    id: restock_id,
                    quantity: row.get("quantity_res"),
                    supplier_id: row.get("supplier_id_res"),
                    status: RestockStatus::from_str(&status_str).unwrap_or(RestockStatus::Pending),
                    restock_date: row.get("restock_date_res"),
                    created_at: row.get("created_at"),
                    updated_at: row.get("updated_at"),
                    lines: Vec::new(),
                }
            });

            if let Ok(product_id) = row.try_get::<i32, _>("product_id_lrs") {
                restock.lines.push(LineRestockResponse {
                    product_id,
                    product_name: row.get("product_name"),
                    product_reference: row.get("product_reference"),
                    quantity: row.get("quantity_lrs"),
                    unit_price: row.get("unit_price_lrs"),
                    total_price: row.get("total_price_lrs"),
                });
            }
        }

        Ok(restocks_map.into_values().collect())
    }
}
