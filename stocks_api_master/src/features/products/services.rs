use sqlx::PgPool;
use super::dto::{ProductResponse, ProductPriceResponse, SearchParams, ProductStatus};

pub struct ProductService;

impl ProductService {
    pub async fn get_products(pool: &PgPool, params: &SearchParams) -> Result<Vec<ProductResponse>, sqlx::Error> {
        let limit = params.limit.unwrap_or(50);
        let offset = params.offset.unwrap_or(0);

        println!("🔍 Search params: {:?}", params);

        let mut query_builder = sqlx::QueryBuilder::new(
            "SELECT id_pro, name_pro, category_pro, reference_pro, supplier_id_pro,
                    stock_quantity_pro, buying_price_pro, status_pro, date_last_reassor_pro, 
                    created_at, updated_at 
             FROM products_pro"
        );

        // Vérifier si on a des conditions
        let has_conditions = params.q.is_some() || 
                            params.category.is_some() || 
                            params.supplier_id.is_some() ||
                            params.min_price.is_some() || 
                            params.max_price.is_some() ||
                            params.min_stock.is_some() || 
                            params.max_stock.is_some();

        if has_conditions {
            query_builder.push(" WHERE ");
            println!("✅ Adding WHERE clause");
        }

        let mut first_condition = true;

        // 🔍 Search by name avec nettoyage et recherche étendue
        if let Some(ref q) = params.q {
            let cleaned_search = q.trim();
            
            if !cleaned_search.is_empty() {
                println!("🔍 Adding name filter: '{}'", cleaned_search);
                
                if !first_condition {
                    query_builder.push(" AND ");
                }
                
                // ✅ Multiple colonnes pour une recherche plus large
                query_builder.push("(");
                query_builder.push("name_pro ILIKE ");
                let search_term = format!("%{}%", cleaned_search);
                query_builder.push_bind(search_term.clone());
                
                // Chercher aussi dans la référence
                query_builder.push(" OR reference_pro ILIKE ");
                query_builder.push_bind(search_term);
                query_builder.push(")");
                
                println!("✅ Name filter added with pattern: %{}%", cleaned_search);
                first_condition = false;
            } else {
                println!("⚠️ Empty search term after cleaning, skipping");
            }
        }

        // Filter by category
        if let Some(ref category) = params.category {
            println!("🔍 Adding category filter: '{}'", category);
            
            if !first_condition {
                query_builder.push(" AND ");
            }
            
            query_builder.push("LOWER(category_pro) = LOWER(");
            query_builder.push_bind(category);
            query_builder.push(")");
            
            first_condition = false;
        }

        // Filter by supplier
        if let Some(supplier_id) = params.supplier_id {
            println!("🔍 Adding supplier filter: {}", supplier_id);
            
            if !first_condition {
                query_builder.push(" AND ");
            }
            
            query_builder.push("supplier_id_pro = ");
            query_builder.push_bind(supplier_id);
            
            first_condition = false;
        }

        // Filter by price range
        if let Some(min_price) = params.min_price {
            println!("🔍 Adding min price filter: {}", min_price);
            
            if !first_condition {
                query_builder.push(" AND ");
            }
            
            query_builder.push("buying_price_pro >= ");
            query_builder.push_bind(min_price);
            
            first_condition = false;
        }

        if let Some(max_price) = params.max_price {
            println!("🔍 Adding max price filter: {}", max_price);
            
            if !first_condition {
                query_builder.push(" AND ");
            }
            
            query_builder.push("buying_price_pro <= ");
            query_builder.push_bind(max_price);
            
            first_condition = false;
        }

        // Filter by stock range
        if let Some(min_stock) = params.min_stock {
            println!("🔍 Adding min stock filter: {}", min_stock);
            
            if !first_condition {
                query_builder.push(" AND ");
            }
            
            query_builder.push("stock_quantity_pro >= ");
            query_builder.push_bind(min_stock);
            
            first_condition = false;
        }

        if let Some(max_stock) = params.max_stock {
            println!("🔍 Adding max stock filter: {}", max_stock);
            
            if !first_condition {
                query_builder.push(" AND ");
            }
            
            query_builder.push("stock_quantity_pro <= ");
            query_builder.push_bind(max_stock);
            
            first_condition = false;
        }

        // Order by created_at DESC
        query_builder.push(" ORDER BY created_at DESC");
        
        // Add LIMIT and OFFSET
        query_builder.push(" LIMIT ");
        query_builder.push_bind(limit);
        query_builder.push(" OFFSET ");
        query_builder.push_bind(offset);

        println!("✅ Query built with LIMIT {} OFFSET {}", limit, offset);

        let query = query_builder.build();
        let rows = query.fetch_all(pool).await?;

        let products: Vec<ProductResponse> = rows
            .iter()
            .map(|row| ProductResponse::from_row(row))
            .collect();

        println!("✅ Retrieved {} products", products.len());
        Ok(products)
    }

    pub async fn search_products_light(pool: &PgPool, q: Option<String>) -> Result<Vec<super::dto::ProductLightResponse>, sqlx::Error> {
        let mut query_builder = sqlx::QueryBuilder::new(
            "SELECT id_pro, name_pro, category_pro FROM products_pro"
        );

        if let Some(ref search) = q {
            let cleaned_search = search.trim();
            if !cleaned_search.is_empty() {
                query_builder.push(" WHERE name_pro ILIKE ");
                let search_term = format!("%{}%", cleaned_search);
                query_builder.push_bind(search_term.clone());
                query_builder.push(" OR category_pro ILIKE ");
                query_builder.push_bind(search_term);
            }
        }

        query_builder.push(" ORDER BY name_pro ASC LIMIT 50");

        let rows = query_builder.build().fetch_all(pool).await?;

        let products = rows
            .iter()
            .map(|row| {
                use sqlx::Row;
                super::dto::ProductLightResponse {
                    id: row.get("id_pro"),
                    name: row.get("name_pro"),
                    category: row.get("category_pro"),
                }
            })
            .collect();

        Ok(products)
    }

    pub async fn get_product_by_id(pool: &PgPool, id: i32) -> Result<Option<ProductResponse>, sqlx::Error> {
        let row = sqlx::query(
            "SELECT id_pro, name_pro, category_pro, reference_pro, supplier_id_pro,
                    stock_quantity_pro, buying_price_pro, status_pro, date_last_reassor_pro, 
                    created_at, updated_at 
             FROM products_pro 
             WHERE id_pro = $1"
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;

        Ok(row.map(|row| ProductResponse::from_row(&row)))
    }

    pub async fn get_product_by_reference(pool: &PgPool, reference: &str) -> Result<Option<ProductResponse>, sqlx::Error> {
        let row = sqlx::query(
            "SELECT id_pro, name_pro, category_pro, reference_pro, supplier_id_pro,
                    stock_quantity_pro, buying_price_pro, status_pro, date_last_reassor_pro, 
                    created_at, updated_at 
             FROM products_pro 
             WHERE reference_pro = $1"
        )
        .bind(reference)
        .fetch_optional(pool)
        .await?;

        Ok(row.map(|row| ProductResponse::from_row(&row)))
    }

    pub async fn check_reference_exists(pool: &PgPool, reference: &str) -> Result<bool, sqlx::Error> {
        let existing = sqlx::query("SELECT id_pro FROM products_pro WHERE reference_pro = $1")
            .bind(reference)
            .fetch_optional(pool)
            .await?;

        Ok(existing.is_some())
    }

    pub async fn check_reference_exists_excluding_id(pool: &PgPool, reference: &str, id: i32) -> Result<bool, sqlx::Error> {
        let existing = sqlx::query("SELECT id_pro FROM products_pro WHERE reference_pro = $1 AND id_pro != $2")
            .bind(reference)
            .bind(id)
            .fetch_optional(pool)
            .await?;

        Ok(existing.is_some())
    }

    pub async fn create_product(
        pool: &PgPool,
        name: &str,
        category: &str,
        reference: &str,
        supplier_id: i32,
        stock_quantity: i32,
        buying_price: f64,
        status: Option<ProductStatus>,
    ) -> Result<ProductResponse, sqlx::Error> {
        let status_value = status.unwrap_or(ProductStatus::InStock);
        
        let row = sqlx::query(
            "INSERT INTO products_pro (name_pro, category_pro, reference_pro, supplier_id_pro, 
                                      stock_quantity_pro, buying_price_pro, status_pro, date_last_reassor_pro)
             VALUES ($1, $2, $3, $4, $5, $6, $7::product_status_enum, NOW())
             RETURNING id_pro, name_pro, category_pro, reference_pro, supplier_id_pro,
                       stock_quantity_pro, buying_price_pro, status_pro, date_last_reassor_pro, 
                       created_at, updated_at"
        )
        .bind(name)
        .bind(category)
        .bind(reference)
        .bind(supplier_id)
        .bind(stock_quantity)
        .bind(buying_price)
        .bind(status_value)
        .fetch_one(pool)
        .await?;

        Ok(ProductResponse::from_row(&row))
    }

    pub async fn add_product_price(
        pool: &PgPool,
        product_id: i32,
        selling_price: f64,
    ) -> Result<ProductPriceResponse, sqlx::Error> {
        let row = sqlx::query(
            "INSERT INTO productprices_prp (product_ref_prp, price_prp)
             VALUES ($1, $2)
             RETURNING id_prp, product_ref_prp, price_prp, created_at"
        )
        .bind(product_id)
        .bind(selling_price)
        .fetch_one(pool)
        .await?;

        use sqlx::Row;
        use rust_decimal::prelude::ToPrimitive;
        Ok(ProductPriceResponse {
            id: row.get("id_prp"),
            product_ref: row.get("product_ref_prp"),
            selling_price: row.get::<rust_decimal::Decimal, _>("price_prp").to_f64().unwrap_or(0.0),
            created_at: row.get("created_at"),
        })
    }

    pub async fn update_product(
        pool: &PgPool,
        id: i32,
        name: Option<&str>,
        category: Option<&str>,
        reference: Option<&str>,
        supplier_id: Option<i32>,
        stock_quantity: Option<i32>,
        buying_price: Option<f64>,
        status: Option<ProductStatus>,
        update_stock_date: bool,
    ) -> Result<ProductResponse, sqlx::Error> {
        let query = if update_stock_date {
            "UPDATE products_pro 
             SET name_pro = COALESCE($2, name_pro),
                 category_pro = COALESCE($3, category_pro),
                 reference_pro = COALESCE($4, reference_pro),
                 supplier_id_pro = COALESCE($5, supplier_id_pro),
                 stock_quantity_pro = COALESCE($6, stock_quantity_pro),
                 buying_price_pro = COALESCE($7, buying_price_pro),
                 status_pro = COALESCE($8, status_pro),
                 date_last_reassor_pro = NOW(),
                 updated_at = NOW()
             WHERE id_pro = $1
             RETURNING id_pro, name_pro, category_pro, reference_pro, supplier_id_pro,
                       stock_quantity_pro, buying_price_pro, status_pro, date_last_reassor_pro, 
                       created_at, updated_at"
        } else {
            "UPDATE products_pro 
             SET name_pro = COALESCE($2, name_pro),
                 category_pro = COALESCE($3, category_pro),
                 reference_pro = COALESCE($4, reference_pro),
                 supplier_id_pro = COALESCE($5, supplier_id_pro),
                 stock_quantity_pro = COALESCE($6, stock_quantity_pro),
                 buying_price_pro = COALESCE($7, buying_price_pro),
                 status_pro = COALESCE($8, status_pro),
                 updated_at = NOW()
             WHERE id_pro = $1
             RETURNING id_pro, name_pro, category_pro, reference_pro, supplier_id_pro,
                       stock_quantity_pro, buying_price_pro, status_pro, date_last_reassor_pro, 
                       created_at, updated_at"
        };

        let row = sqlx::query(query)
            .bind(id)
            .bind(name)
            .bind(category)
            .bind(reference)
            .bind(supplier_id)
            .bind(stock_quantity)
            .bind(buying_price)
            .bind(status)
            .fetch_one(pool)
            .await?;

        Ok(ProductResponse::from_row(&row))
    }

    pub async fn delete_product(pool: &PgPool, id: i32) -> Result<u64, sqlx::Error> {
        let result = sqlx::query("DELETE FROM products_pro WHERE id_pro = $1")
            .bind(id)
            .execute(pool)
            .await?;

        Ok(result.rows_affected())
    }

    pub async fn product_exists(pool: &PgPool, id: i32) -> Result<bool, sqlx::Error> {
        let exists = sqlx::query("SELECT id_pro FROM products_pro WHERE id_pro = $1")
            .bind(id)
            .fetch_optional(pool)
            .await?;

        Ok(exists.is_some())
    }

    pub async fn update_stock(pool: &PgPool, id: i32, quantity: i32) -> Result<ProductResponse, sqlx::Error> {
        let status = if quantity <= 0 {
            ProductStatus::OutOfStock
        } else {
            ProductStatus::InStock
        };

        let row = sqlx::query(
            "UPDATE products_pro 
             SET stock_quantity_pro = $2,
                 status_pro = $3::product_status_enum,
                 date_last_reassor_pro = NOW(),
                 updated_at = NOW()
             WHERE id_pro = $1
             RETURNING id_pro, name_pro, category_pro, reference_pro, supplier_id_pro,
                       stock_quantity_pro, buying_price_pro, status_pro, date_last_reassor_pro, 
                       created_at, updated_at"
        )
        .bind(id)
        .bind(std::cmp::max(0, quantity))
        .bind(status)
        .fetch_one(pool)
        .await?;

        Ok(ProductResponse::from_row(&row))
    }
}
