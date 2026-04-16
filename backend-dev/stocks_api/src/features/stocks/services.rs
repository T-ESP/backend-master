use sqlx::{PgPool, QueryBuilder, Row};
use crate::features::stocks::dto::{StockResponse, StockAlert, StockSummary, StockParams, OverstockParams};

pub async fn get_out_of_stock_products(pool: &PgPool, limit: i32) -> Result<Vec<StockResponse>, sqlx::Error> {
    let mut query = QueryBuilder::new(
        "SELECT id_pro, name_pro, category_pro, reference_pro, supplier_id_pro,
                stock_quantity_pro, buying_price_pro, date_last_reassor_pro
         FROM products_pro 
         WHERE stock_quantity_pro = 0
         ORDER BY date_last_reassor_pro DESC"
    );
    
    query.push(" LIMIT ");
    query.push_bind(limit);

    let rows = query.build().fetch_all(pool).await?;

    let products: Vec<StockResponse> = rows
        .into_iter()
        .map(|row| StockResponse {
            id: row.get("id_pro"),
            name: row.get("name_pro"),
            category: row.get("category_pro"),
            reference: row.get("reference_pro"),
            supplier_id: row.get("supplier_id_pro"),
            stock_quantity: row.get("stock_quantity_pro"),
            buying_price: row.get("buying_price_pro"),
            date_last_reassor: row.get("date_last_reassor_pro"),
            stock_status: "out_of_stock".to_string(),
        })
        .collect();

    Ok(products)
}

pub async fn get_low_stock_products(pool: &PgPool, limit: i32, threshold: i32) -> Result<Vec<StockResponse>, sqlx::Error> {
    let mut query = QueryBuilder::new(
        "SELECT id_pro, name_pro, category_pro, reference_pro, supplier_id_pro,
                stock_quantity_pro, buying_price_pro, date_last_reassor_pro
         FROM products_pro 
         WHERE stock_quantity_pro > 0 AND stock_quantity_pro <= "
    );
    
    query.push_bind(threshold);
    query.push(" ORDER BY stock_quantity_pro ASC, date_last_reassor_pro DESC LIMIT ");
    query.push_bind(limit);

    let rows = query.build().fetch_all(pool).await?;

    let products: Vec<StockResponse> = rows
        .into_iter()
        .map(|row| StockResponse {
            id: row.get("id_pro"),
            name: row.get("name_pro"),
            category: row.get("category_pro"),
            reference: row.get("reference_pro"),
            supplier_id: row.get("supplier_id_pro"),
            stock_quantity: row.get("stock_quantity_pro"),
            buying_price: row.get("buying_price_pro"),
            date_last_reassor: row.get("date_last_reassor_pro"),
            stock_status: "low_stock".to_string(),
        })
        .collect();

    Ok(products)
}

pub async fn get_soon_out_of_stock_products(pool: &PgPool, limit: i32) -> Result<Vec<StockResponse>, sqlx::Error> {
    let mut query = QueryBuilder::new(
        "SELECT id_pro, name_pro, category_pro, reference_pro, supplier_id_pro,
                stock_quantity_pro, buying_price_pro, date_last_reassor_pro
         FROM products_pro 
         WHERE stock_quantity_pro > 0 AND stock_quantity_pro <= 5
         ORDER BY stock_quantity_pro ASC, date_last_reassor_pro ASC"
    );
    
    query.push(" LIMIT ");
    query.push_bind(limit);

    let rows = query.build().fetch_all(pool).await?;

    let products: Vec<StockResponse> = rows
        .into_iter()
        .map(|row| StockResponse {
            id: row.get("id_pro"),
            name: row.get("name_pro"),
            category: row.get("category_pro"),
            reference: row.get("reference_pro"),
            supplier_id: row.get("supplier_id_pro"),
            stock_quantity: row.get("stock_quantity_pro"),
            buying_price: row.get("buying_price_pro"),
            date_last_reassor: row.get("date_last_reassor_pro"),
            stock_status: "critical".to_string(),
        })
        .collect();

    Ok(products)
}

pub async fn get_overstock_products(pool: &PgPool, limit: i32, multiplier: f64) -> Result<Vec<StockResponse>, sqlx::Error> {
    let base_threshold = (100.0 * multiplier) as i32;
    
    let mut query = QueryBuilder::new(
        "SELECT id_pro, name_pro, category_pro, reference_pro, supplier_id_pro,
                stock_quantity_pro, buying_price_pro, date_last_reassor_pro
         FROM products_pro 
         WHERE stock_quantity_pro >= "
    );
    
    query.push_bind(base_threshold);
    query.push(" ORDER BY stock_quantity_pro DESC, date_last_reassor_pro ASC LIMIT ");
    query.push_bind(limit);

    let rows = query.build().fetch_all(pool).await?;

    let products: Vec<StockResponse> = rows
        .into_iter()
        .map(|row| StockResponse {
            id: row.get("id_pro"),
            name: row.get("name_pro"),
            category: row.get("category_pro"),
            reference: row.get("reference_pro"),
            supplier_id: row.get("supplier_id_pro"),
            stock_quantity: row.get("stock_quantity_pro"),
            buying_price: row.get("buying_price_pro"),
            date_last_reassor: row.get("date_last_reassor_pro"),
            stock_status: "overstock".to_string(),
        })
        .collect();

    Ok(products)
}

pub async fn get_stock_alerts_data(pool: &PgPool, limit: i32) -> Result<Vec<StockAlert>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id_pro, name_pro, category_pro, reference_pro, supplier_id_pro,
                stock_quantity_pro, buying_price_pro, date_last_reassor_pro
         FROM products_pro 
         WHERE stock_quantity_pro <= 10 OR stock_quantity_pro >= 300
         ORDER BY 
             CASE 
                 WHEN stock_quantity_pro = 0 THEN 1
                 WHEN stock_quantity_pro <= 5 THEN 2
                 WHEN stock_quantity_pro <= 10 THEN 3
                 ELSE 4
             END,
             stock_quantity_pro ASC
         LIMIT $1"
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;

    let alerts: Vec<StockAlert> = rows
        .into_iter()
        .map(|row| {
            let stock_quantity: i32 = row.get("stock_quantity_pro");
            let (alert_type, severity, message) = match stock_quantity {
                0 => ("out_of_stock", "critical", "Product is completely out of stock"),
                1..=5 => ("critical_low", "high", "Stock critically low - immediate attention needed"),
                6..=10 => ("low_stock", "medium", "Stock running low - consider reordering"),
                300.. => ("overstock", "low", "High stock levels - consider promotional activities"),
                _ => ("normal", "none", "Stock levels normal"),
            };

            StockAlert {
                product: StockResponse {
                    id: row.get("id_pro"),
                    name: row.get("name_pro"),
                    category: row.get("category_pro"),
                    reference: row.get("reference_pro"),
                    supplier_id: row.get("supplier_id_pro"),
                    stock_quantity,
                    buying_price: row.get("buying_price_pro"),
                    date_last_reassor: row.get("date_last_reassor_pro"),
                    stock_status: alert_type.to_string(),
                },
                alert_type: alert_type.to_string(),
                severity: severity.to_string(),
                message: message.to_string(),
            }
        })
        .collect();

    Ok(alerts)
}

pub async fn get_stock_summary_data(pool: &PgPool) -> Result<StockSummary, sqlx::Error> {
    let summary_row = sqlx::query(
        "SELECT 
             COUNT(*) as total_products,
             COUNT(CASE WHEN stock_quantity_pro = 0 THEN 1 END) as out_of_stock,
             COUNT(CASE WHEN stock_quantity_pro > 0 AND stock_quantity_pro <= 10 THEN 1 END) as low_stock,
             COUNT(CASE WHEN stock_quantity_pro >= 300 THEN 1 END) as overstock,
             COALESCE(SUM(stock_quantity_pro * buying_price_pro), 0) as total_value
         FROM products_pro"
    )
    .fetch_one(pool)
    .await?;

    let categories_rows = sqlx::query(
        "SELECT DISTINCT category_pro 
         FROM products_pro 
         WHERE stock_quantity_pro <= 10 OR stock_quantity_pro >= 300
         ORDER BY category_pro"
    )
    .fetch_all(pool)
    .await?;

    let categories_affected: Vec<String> = categories_rows
        .into_iter()
        .map(|row| row.get("category_pro"))
        .collect();

    Ok(StockSummary {
        total_products: summary_row.get("total_products"),
        out_of_stock_count: summary_row.get("out_of_stock"),
        low_stock_count: summary_row.get("low_stock"),
        overstock_count: summary_row.get("overstock"),
        total_stock_value: summary_row.get("total_value"),
        categories_affected,
    })
}