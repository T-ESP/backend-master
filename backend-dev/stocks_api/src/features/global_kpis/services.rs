use sqlx::{PgPool, Row};
use rust_decimal::prelude::ToPrimitive;
use super::dto::*;

pub struct GlobalKpisService;

impl GlobalKpisService {
    // ====================== 1. PERFORMANCE GLOBALE ======================

    pub async fn get_global_performance_kpis(
        pool: &PgPool,
        params: &KpiPeriodParams,
    ) -> Result<GlobalPerformanceKpis, sqlx::Error> {
        let start_date = params.start_date.unwrap_or_else(|| {
            (chrono::Utc::now() - chrono::Duration::days(30)).date_naive()
        });
        let end_date = params.end_date.unwrap_or_else(|| chrono::Utc::now().date_naive());

        // Chiffre d'affaires et profit total
        let revenue_profit = sqlx::query(
            "SELECT
                COALESCE(SUM(lor.line_total_lor), 0) as total_revenue,
                COALESCE(SUM(lor.quantity_lor * p.buying_price_pro), 0) as total_cost
            FROM line_order_lor lor
            JOIN order_ord o ON lor.order_id_lor = o.id_ord
            JOIN products_pro p ON lor.product_id_lor = p.id_pro
            WHERE o.order_date_ord >= $1 AND o.order_date_ord <= $2"
        )
        .bind(start_date)
        .bind(end_date)
        .fetch_one(pool)
        .await?;

        let total_revenue: f64 = revenue_profit.try_get::<rust_decimal::Decimal, _>("total_revenue")?
            .to_f64()
            .unwrap_or(0.0);
        let total_cost: f64 = revenue_profit.try_get::<rust_decimal::Decimal, _>("total_cost")?
            .to_f64()
            .unwrap_or(0.0);
        let total_profit = total_revenue - total_cost;
        let avg_margin_rate = if total_cost > 0.0 {
            Some((total_profit / total_cost) * 100.0)
        } else {
            None
        };

        // Nombre de produits par statut
        let products_count = sqlx::query(
            "SELECT
                COUNT(*) as total,
                COUNT(*) FILTER (WHERE status_pro = 'in_stock') as active,
                COUNT(*) FILTER (WHERE status_pro = 'out_of_stock') as inactive,
                COUNT(*) FILTER (WHERE status_pro = 'discontinued') as discontinued
            FROM products_pro"
        )
        .fetch_one(pool)
        .await?;

        let total_products_count: i64 = products_count.try_get("total")?;
        let active_products_count: i64 = products_count.try_get("active")?;
        let inactive_products_count: i64 = products_count.try_get("inactive")?;
        let discontinued_products_count: i64 = products_count.try_get("discontinued")?;

        // Nombre de commandes et panier moyen
        let orders_stats = sqlx::query(
            "SELECT
                COUNT(DISTINCT o.id_ord) as total_orders,
                AVG(order_total.total) as avg_basket
            FROM order_ord o
            LEFT JOIN (
                SELECT order_id_lor, SUM(line_total_lor) as total
                FROM line_order_lor
                GROUP BY order_id_lor
            ) order_total ON o.id_ord = order_total.order_id_lor
            WHERE o.order_date_ord >= $1 AND o.order_date_ord <= $2"
        )
        .bind(start_date)
        .bind(end_date)
        .fetch_one(pool)
        .await?;

        let total_orders_count: i64 = orders_stats.try_get("total_orders")?;
        let global_avg_basket_value = orders_stats.try_get::<Option<rust_decimal::Decimal>, _>("avg_basket")?
            .and_then(|v| v.to_f64());

        // Valeur du stock
        let stock_value = sqlx::query(
            "SELECT
                COALESCE(SUM(p.stock_quantity_pro * p.buying_price_pro), 0) as stock_cost,
                COALESCE(SUM(p.stock_quantity_pro * COALESCE(pp.price_prp, 0)), 0) as stock_potential
            FROM products_pro p
            LEFT JOIN LATERAL (
                SELECT price_prp
                FROM productprices_prp
                WHERE product_ref_prp = p.id_pro
                ORDER BY created_at DESC
                LIMIT 1
            ) pp ON true"
        )
        .fetch_one(pool)
        .await?;

        let total_stock_value_cost: f64 = stock_value.try_get::<rust_decimal::Decimal, _>("stock_cost")?
            .to_f64()
            .unwrap_or(0.0);
        let total_stock_value_potential: f64 = stock_value.try_get::<rust_decimal::Decimal, _>("stock_potential")?
            .to_f64()
            .unwrap_or(0.0);

        Ok(GlobalPerformanceKpis {
            total_revenue,
            total_profit,
            avg_margin_rate,
            total_products_count: total_products_count as i32,
            active_products_count: active_products_count as i32,
            inactive_products_count: inactive_products_count as i32,
            discontinued_products_count: discontinued_products_count as i32,
            total_orders_count: total_orders_count as i32,
            global_avg_basket_value,
            total_stock_value_cost,
            total_stock_value_potential,
        })
    }

    // ====================== 2. ANALYSES PAR CATÉGORIE ======================

    pub async fn get_category_analysis_kpis(
        pool: &PgPool,
        params: &KpiPeriodParams,
    ) -> Result<CategoryAnalysisKpis, sqlx::Error> {
        let start_date = params.start_date.unwrap_or_else(|| {
            (chrono::Utc::now() - chrono::Duration::days(30)).date_naive()
        });
        let end_date = params.end_date.unwrap_or_else(|| chrono::Utc::now().date_naive());

        // Statistiques par catégorie
        let category_rows = sqlx::query(
            "SELECT
                p.category_pro as category,
                COALESCE(SUM(lor.line_total_lor), 0) as revenue,
                COALESCE(SUM(lor.line_total_lor - (lor.quantity_lor * p.buying_price_pro)), 0) as profit,
                COUNT(DISTINCT p.id_pro) as products_count
            FROM products_pro p
            LEFT JOIN line_order_lor lor ON p.id_pro = lor.product_id_lor
            LEFT JOIN order_ord o ON lor.order_id_lor = o.id_ord
                AND o.order_date_ord >= $1 AND o.order_date_ord <= $2
            GROUP BY p.category_pro"
        )
        .bind(start_date)
        .bind(end_date)
        .fetch_all(pool)
        .await?;

        let total_stock: f64 = sqlx::query("SELECT COALESCE(SUM(stock_quantity_pro)::NUMERIC, 0) as total FROM products_pro")
            .fetch_one(pool)
            .await?
            .try_get::<rust_decimal::Decimal, _>("total")?
            .to_f64()
            .unwrap_or(1.0);

        let mut by_category = Vec::new();
        for row in &category_rows {
            let category: String = row.try_get("category")?;
            let revenue: f64 = row.try_get::<rust_decimal::Decimal, _>("revenue")?.to_f64().unwrap_or(0.0);
            let profit: f64 = row.try_get::<rust_decimal::Decimal, _>("profit")?.to_f64().unwrap_or(0.0);
            let products_count: i64 = row.try_get("products_count")?;

            // Calculer le taux de rotation moyen pour cette catégorie
            let avg_turnover = sqlx::query(
                "SELECT AVG(
                    CASE
                        WHEN p.stock_quantity_pro > 0
                        THEN COALESCE(sales.quantity_sold, 0)::float / p.stock_quantity_pro
                        ELSE NULL
                    END
                ) as avg_turnover
                FROM products_pro p
                LEFT JOIN (
                    SELECT product_id_lor, SUM(quantity_lor) as quantity_sold
                    FROM line_order_lor lor
                    JOIN order_ord o ON lor.order_id_lor = o.id_ord
                    WHERE o.order_date_ord >= $1 AND o.order_date_ord <= $2
                    GROUP BY product_id_lor
                ) sales ON p.id_pro = sales.product_id_lor
                WHERE p.category_pro = $3"
            )
            .bind(start_date)
            .bind(end_date)
            .bind(&category)
            .fetch_one(pool)
            .await?
            .try_get::<Option<f64>, _>("avg_turnover")?;

            let cost = revenue - profit;
            let avg_margin_rate = if cost > 0.0 {
                Some((profit / cost) * 100.0)
            } else {
                None
            };

            // Stock distribution pour cette catégorie
            let category_stock: f64 = sqlx::query(
                "SELECT COALESCE(SUM(stock_quantity_pro)::NUMERIC, 0) as total FROM products_pro WHERE category_pro = $1"
            )
            .bind(&category)
            .fetch_one(pool)
            .await?
            .try_get::<rust_decimal::Decimal, _>("total")?
            .to_f64()
            .unwrap_or(0.0);

            let stock_distribution_percent = if total_stock > 0.0 {
                Some((category_stock / total_stock) * 100.0)
            } else {
                None
            };

            by_category.push(CategoryStats {
                category,
                revenue,
                profit,
                avg_margin_rate,
                products_count: products_count as i32,
                avg_turnover_rate: avg_turnover,
                stock_distribution_percent,
            });
        }

        // Top 5 par revenue
        let mut top_5_by_revenue = by_category.clone();
        top_5_by_revenue.sort_by(|a, b| b.revenue.partial_cmp(&a.revenue).unwrap());
        top_5_by_revenue.truncate(5);

        // Top 5 par profit
        let mut top_5_by_profit = by_category.clone();
        top_5_by_profit.sort_by(|a, b| b.profit.partial_cmp(&a.profit).unwrap());
        top_5_by_profit.truncate(5);

        // Top 5 par volume (nombre de produits)
        let mut top_5_by_volume = by_category.clone();
        top_5_by_volume.sort_by(|a, b| b.products_count.cmp(&a.products_count));
        top_5_by_volume.truncate(5);

        Ok(CategoryAnalysisKpis {
            by_category,
            top_5_by_revenue,
            top_5_by_profit,
            top_5_by_volume,
        })
    }

    // ====================== 3. ANALYSES PAR FOURNISSEUR ======================

    pub async fn get_supplier_analysis_kpis(
        pool: &PgPool,
        params: &KpiPeriodParams,
    ) -> Result<SupplierAnalysisKpis, sqlx::Error> {
        let start_date = params.start_date.unwrap_or_else(|| {
            (chrono::Utc::now() - chrono::Duration::days(30)).date_naive()
        });
        let end_date = params.end_date.unwrap_or_else(|| chrono::Utc::now().date_naive());

        let supplier_rows = sqlx::query(
            "SELECT
                s.id_sup as supplier_id,
                s.name_sup as supplier_name,
                COALESCE(SUM(lor.line_total_lor), 0) as revenue,
                COALESCE(SUM(lor.line_total_lor - (lor.quantity_lor * p.buying_price_pro)), 0) as profit,
                COUNT(DISTINCT p.id_pro) as products_count,
                COUNT(DISTINCT r.id_res) as restocks_count,
                COALESCE(SUM(lrs.total_price_lrs), 0) as total_purchase_cost,
                AVG(EXTRACT(DAY FROM (r.updated_at - r.created_at))) FILTER (WHERE r.status_res = 'received') as avg_delivery_delay,
                COUNT(*) FILTER (WHERE r.status_res = 'received')::float / NULLIF(COUNT(*), 0) * 100 as reliability_rate,
                COUNT(*) FILTER (WHERE r.status_res = 'cancelled')::float / NULLIF(COUNT(*), 0) * 100 as cancellation_rate
            FROM supplier_sup s
            LEFT JOIN products_pro p ON s.id_sup = p.supplier_id_pro
            LEFT JOIN line_order_lor lor ON p.id_pro = lor.product_id_lor
            LEFT JOIN order_ord o ON lor.order_id_lor = o.id_ord
                AND o.order_date_ord >= $1 AND o.order_date_ord <= $2
            LEFT JOIN restock_res r ON s.id_sup = r.supplier_id_res
                AND r.restock_date_res >= $1 AND r.restock_date_res <= $2
            LEFT JOIN line_restock_lrs lrs ON r.id_res = lrs.restock_id_lrs
            GROUP BY s.id_sup, s.name_sup"
        )
        .bind(start_date)
        .bind(end_date)
        .fetch_all(pool)
        .await?;

        let mut by_supplier = Vec::new();
        for row in &supplier_rows {
            by_supplier.push(SupplierStats {
                supplier_id: row.try_get("supplier_id")?,
                supplier_name: row.try_get("supplier_name")?,
                revenue: row.try_get::<rust_decimal::Decimal, _>("revenue")?.to_f64().unwrap_or(0.0),
                profit: row.try_get::<rust_decimal::Decimal, _>("profit")?.to_f64().unwrap_or(0.0),
                products_count: row.try_get::<i64, _>("products_count")? as i32,
                restocks_count: row.try_get::<i64, _>("restocks_count")? as i32,
                total_purchase_cost: row.try_get::<rust_decimal::Decimal, _>("total_purchase_cost")?.to_f64().unwrap_or(0.0),
                avg_delivery_delay_days: row.try_get::<Option<rust_decimal::Decimal>, _>("avg_delivery_delay")?.and_then(|v| v.to_f64()),
                reliability_rate: row.try_get::<Option<f64>, _>("reliability_rate")?,
                cancellation_rate: row.try_get::<Option<f64>, _>("cancellation_rate")?,
            });
        }

        // Top 5 par revenue
        let mut top_5_by_revenue = by_supplier.clone();
        top_5_by_revenue.sort_by(|a, b| b.revenue.partial_cmp(&a.revenue).unwrap());
        top_5_by_revenue.truncate(5);

        // Top 5 par fiabilité
        let mut top_5_by_reliability = by_supplier.clone();
        top_5_by_reliability.sort_by(|a, b| {
            b.reliability_rate.unwrap_or(0.0).partial_cmp(&a.reliability_rate.unwrap_or(0.0)).unwrap()
        });
        top_5_by_reliability.truncate(5);

        // Top 5 par coût (inversé pour avoir les plus importants)
        let mut top_5_by_cost = by_supplier.clone();
        top_5_by_cost.sort_by(|a, b| b.total_purchase_cost.partial_cmp(&a.total_purchase_cost).unwrap());
        top_5_by_cost.truncate(5);

        Ok(SupplierAnalysisKpis {
            by_supplier,
            top_5_by_revenue,
            top_5_by_reliability,
            top_5_by_cost,
        })
    }

    // ====================== 4. SANTÉ DU CATALOGUE ======================

    pub async fn get_catalog_health_kpis(
        pool: &PgPool,
        params: &KpiPeriodParams,
    ) -> Result<CatalogHealthKpis, sqlx::Error> {
        let start_date = params.start_date.unwrap_or_else(|| {
            (chrono::Utc::now() - chrono::Duration::days(30)).date_naive()
        });
        let end_date = params.end_date.unwrap_or_else(|| chrono::Utc::now().date_naive());

        let health_stats = sqlx::query(
            "SELECT
                COUNT(*) as total_products,
                COUNT(*) FILTER (WHERE stock_quantity_pro > 0) as in_stock,
                COUNT(*) FILTER (WHERE stock_quantity_pro = 0 AND status_pro != 'discontinued') as out_of_stock,
                COUNT(*) FILTER (WHERE status_pro = 'discontinued') as discontinued,
                COUNT(*) FILTER (WHERE created_at >= $1) as new_products
            FROM products_pro"
        )
        .bind(start_date)
        .fetch_one(pool)
        .await?;

        let total_products: i64 = health_stats.try_get("total_products")?;
        let in_stock: i64 = health_stats.try_get("in_stock")?;
        let out_of_stock: i64 = health_stats.try_get("out_of_stock")?;
        let discontinued: i64 = health_stats.try_get("discontinued")?;
        let new_products: i64 = health_stats.try_get("new_products")?;

        let availability_rate = if total_products > 0 {
            Some((in_stock as f64 / total_products as f64) * 100.0)
        } else {
            None
        };

        let catalog_renewal_rate = if total_products > 0 {
            Some((new_products as f64 / total_products as f64) * 100.0)
        } else {
            None
        };

        // Produits à faible rotation (< 5 ventes par mois)
        let low_rotation = sqlx::query(
            "SELECT COUNT(DISTINCT p.id_pro) as count
            FROM products_pro p
            LEFT JOIN (
                SELECT product_id_lor, COUNT(*) as sales_count
                FROM line_order_lor lor
                JOIN order_ord o ON lor.order_id_lor = o.id_ord
                WHERE o.order_date_ord >= $1 AND o.order_date_ord <= $2
                GROUP BY product_id_lor
            ) sales ON p.id_pro = sales.product_id_lor
            WHERE COALESCE(sales.sales_count, 0) < 5"
        )
        .bind(start_date)
        .bind(end_date)
        .fetch_one(pool)
        .await?;

        let low_rotation_count: i64 = low_rotation.try_get("count")?;
        let low_rotation_products_percent = if total_products > 0 {
            Some((low_rotation_count as f64 / total_products as f64) * 100.0)
        } else {
            None
        };

        // Produits obsolètes (pas de vente depuis 90 jours)
        let obsolete_date = end_date - chrono::Duration::days(90);
        let obsolete = sqlx::query(
            "SELECT COUNT(DISTINCT p.id_pro) as count
            FROM products_pro p
            LEFT JOIN (
                SELECT product_id_lor, MAX(o.order_date_ord) as last_sale
                FROM line_order_lor lor
                JOIN order_ord o ON lor.order_id_lor = o.id_ord
                GROUP BY product_id_lor
            ) sales ON p.id_pro = sales.product_id_lor
            WHERE sales.last_sale < $1 OR sales.last_sale IS NULL"
        )
        .bind(obsolete_date)
        .fetch_one(pool)
        .await?;

        let obsolete_count: i64 = obsolete.try_get("count")?;
        let obsolete_products_percent = if total_products > 0 {
            Some((obsolete_count as f64 / total_products as f64) * 100.0)
        } else {
            None
        };

        // Produits surstockés (stock > 3 mois de ventes)
        let overstocked = sqlx::query(
            "SELECT COUNT(*) as count
            FROM (
                SELECT
                    p.id_pro,
                    p.stock_quantity_pro,
                    COALESCE(SUM(lor.quantity_lor), 0) as total_sold
                FROM products_pro p
                LEFT JOIN line_order_lor lor ON p.id_pro = lor.product_id_lor
                LEFT JOIN order_ord o ON lor.order_id_lor = o.id_ord
                    AND o.order_date_ord >= $1 AND o.order_date_ord <= $2
                GROUP BY p.id_pro, p.stock_quantity_pro
                HAVING p.stock_quantity_pro > (COALESCE(SUM(lor.quantity_lor), 0) * 3)
            ) overstock"
        )
        .bind(start_date)
        .bind(end_date)
        .fetch_one(pool)
        .await?;

        let overstocked_count: i64 = overstocked.try_get("count")?;
        let overstocked_products_percent = if total_products > 0 {
            Some((overstocked_count as f64 / total_products as f64) * 100.0)
        } else {
            None
        };

        Ok(CatalogHealthKpis {
            availability_rate,
            stockout_products_count: out_of_stock as i32,
            discontinued_products_count: discontinued as i32,
            catalog_renewal_rate,
            low_rotation_products_percent,
            obsolete_products_percent,
            overstocked_products_percent,
        })
    }

    // ====================== 5. DISTRIBUTION ABC ======================

    pub async fn get_abc_distribution_kpis(
        pool: &PgPool,
        params: &KpiPeriodParams,
    ) -> Result<AbcDistributionKpis, sqlx::Error> {
        let start_date = params.start_date.unwrap_or_else(|| {
            (chrono::Utc::now() - chrono::Duration::days(30)).date_naive()
        });
        let end_date = params.end_date.unwrap_or_else(|| chrono::Utc::now().date_naive());

        // Récupérer tous les produits avec leur revenue
        let products = sqlx::query(
            "SELECT
                p.id_pro,
                p.name_pro,
                COALESCE(SUM(lor.line_total_lor), 0) as revenue
            FROM products_pro p
            LEFT JOIN line_order_lor lor ON p.id_pro = lor.product_id_lor
            LEFT JOIN order_ord o ON lor.order_id_lor = o.id_ord
                AND o.order_date_ord >= $1 AND o.order_date_ord <= $2
            GROUP BY p.id_pro, p.name_pro
            ORDER BY revenue DESC"
        )
        .bind(start_date)
        .bind(end_date)
        .fetch_all(pool)
        .await?;

        let total_revenue: f64 = products.iter()
            .map(|row| row.try_get::<rust_decimal::Decimal, _>("revenue")
                .unwrap_or_default()
                .to_f64()
                .unwrap_or(0.0))
            .sum();

        let mut cumulative_revenue = 0.0;
        let mut products_a = Vec::new();
        let mut products_b = Vec::new();
        let mut products_c = Vec::new();

        for row in products {
            let product_id: i32 = row.try_get("id_pro")?;
            let product_name: String = row.try_get("name_pro")?;
            let revenue: f64 = row.try_get::<rust_decimal::Decimal, _>("revenue")?.to_f64().unwrap_or(0.0);

            cumulative_revenue += revenue;
            let cumulative_percent = if total_revenue > 0.0 {
                (cumulative_revenue / total_revenue) * 100.0
            } else {
                0.0
            };

            let product_info = AbcProductInfo {
                product_id,
                product_name,
                revenue,
            };

            if cumulative_percent <= 80.0 {
                products_a.push(product_info);
            } else if cumulative_percent <= 95.0 {
                products_b.push(product_info);
            } else {
                products_c.push(product_info);
            }
        }

        let products_a_revenue: f64 = products_a.iter().map(|p| p.revenue).sum();
        let products_b_revenue: f64 = products_b.iter().map(|p| p.revenue).sum();
        let products_c_revenue: f64 = products_c.iter().map(|p| p.revenue).sum();

        let products_a_revenue_percent = if total_revenue > 0.0 {
            Some((products_a_revenue / total_revenue) * 100.0)
        } else {
            None
        };
        let products_b_revenue_percent = if total_revenue > 0.0 {
            Some((products_b_revenue / total_revenue) * 100.0)
        } else {
            None
        };
        let products_c_revenue_percent = if total_revenue > 0.0 {
            Some((products_c_revenue / total_revenue) * 100.0)
        } else {
            None
        };

        // Top 20% concentration
        let top_20_percent_count = (products_a.len() + products_b.len() + products_c.len()) / 5;
        let top_20_percent_revenue: f64 = products_a.iter()
            .chain(products_b.iter())
            .chain(products_c.iter())
            .take(top_20_percent_count)
            .map(|p| p.revenue)
            .sum();
        let top_20_percent_revenue_concentration = if total_revenue > 0.0 {
            Some((top_20_percent_revenue / total_revenue) * 100.0)
        } else {
            None
        };

        Ok(AbcDistributionKpis {
            products_a_count: products_a.len() as i32,
            products_a_revenue_percent,
            products_a_list: products_a,
            products_b_count: products_b.len() as i32,
            products_b_revenue_percent,
            products_b_list: products_b,
            products_c_count: products_c.len() as i32,
            products_c_revenue_percent,
            products_c_list: products_c,
            top_20_percent_revenue_concentration,
        })
    }

    // ====================== 6. ÉVOLUTIONS & TENDANCES ======================

    pub async fn get_trends_kpis(
        pool: &PgPool,
        params: &KpiPeriodParams,
    ) -> Result<TrendsKpis, sqlx::Error> {
        let start_date = params.start_date.unwrap_or_else(|| {
            (chrono::Utc::now() - chrono::Duration::days(30)).date_naive()
        });
        let end_date = params.end_date.unwrap_or_else(|| chrono::Utc::now().date_naive());

        // Période actuelle
        let current_stats = sqlx::query(
            "SELECT
                COALESCE(SUM(lor.line_total_lor), 0) as revenue,
                COALESCE(SUM(lor.line_total_lor - (lor.quantity_lor * p.buying_price_pro)), 0) as profit,
                COUNT(DISTINCT o.id_ord) as orders_count,
                AVG(order_totals.total) as avg_basket
            FROM order_ord o
            LEFT JOIN line_order_lor lor ON o.id_ord = lor.order_id_lor
            LEFT JOIN products_pro p ON lor.product_id_lor = p.id_pro
            LEFT JOIN (
                SELECT order_id_lor, SUM(line_total_lor) as total
                FROM line_order_lor
                GROUP BY order_id_lor
            ) order_totals ON o.id_ord = order_totals.order_id_lor
            WHERE o.order_date_ord >= $1 AND o.order_date_ord <= $2"
        )
        .bind(start_date)
        .bind(end_date)
        .fetch_one(pool)
        .await?;

        let current_revenue: f64 = current_stats.try_get::<rust_decimal::Decimal, _>("revenue")?.to_f64().unwrap_or(0.0);
        let current_profit: f64 = current_stats.try_get::<rust_decimal::Decimal, _>("profit")?.to_f64().unwrap_or(0.0);
        let current_orders: i64 = current_stats.try_get("orders_count")?;
        let current_basket: f64 = current_stats.try_get::<Option<rust_decimal::Decimal>, _>("avg_basket")?
            .and_then(|v| v.to_f64())
            .unwrap_or(0.0);

        // Période précédente (même durée)
        let period_duration = (end_date - start_date).num_days();
        let previous_start = start_date - chrono::Duration::days(period_duration);
        let previous_end = start_date - chrono::Duration::days(1);

        let previous_stats = sqlx::query(
            "SELECT
                COALESCE(SUM(lor.line_total_lor), 0) as revenue,
                COALESCE(SUM(lor.line_total_lor - (lor.quantity_lor * p.buying_price_pro)), 0) as profit,
                COUNT(DISTINCT o.id_ord) as orders_count,
                AVG(order_totals.total) as avg_basket
            FROM order_ord o
            LEFT JOIN line_order_lor lor ON o.id_ord = lor.order_id_lor
            LEFT JOIN products_pro p ON lor.product_id_lor = p.id_pro
            LEFT JOIN (
                SELECT order_id_lor, SUM(line_total_lor) as total
                FROM line_order_lor
                GROUP BY order_id_lor
            ) order_totals ON o.id_ord = order_totals.order_id_lor
            WHERE o.order_date_ord >= $1 AND o.order_date_ord <= $2"
        )
        .bind(previous_start)
        .bind(previous_end)
        .fetch_one(pool)
        .await?;

        let previous_revenue: f64 = previous_stats.try_get::<rust_decimal::Decimal, _>("revenue")?.to_f64().unwrap_or(0.0);
        let previous_profit: f64 = previous_stats.try_get::<rust_decimal::Decimal, _>("profit")?.to_f64().unwrap_or(0.0);
        let previous_orders: i64 = previous_stats.try_get("orders_count")?;
        let previous_basket: f64 = previous_stats.try_get::<Option<rust_decimal::Decimal>, _>("avg_basket")?
            .and_then(|v| v.to_f64())
            .unwrap_or(0.0);

        // Calcul des croissances
        let revenue_growth_percent = if previous_revenue > 0.0 {
            Some(((current_revenue - previous_revenue) / previous_revenue) * 100.0)
        } else {
            None
        };

        let profit_growth_percent = if previous_profit > 0.0 {
            Some(((current_profit - previous_profit) / previous_profit) * 100.0)
        } else {
            None
        };

        let orders_growth_percent = if previous_orders > 0 {
            Some(((current_orders - previous_orders) as f64 / previous_orders as f64) * 100.0)
        } else {
            None
        };

        let basket_value_growth_percent = if previous_basket > 0.0 {
            Some(((current_basket - previous_basket) / previous_basket) * 100.0)
        } else {
            None
        };

        // Déterminer la tendance globale
        let global_trend = if let Some(growth) = revenue_growth_percent {
            if growth > 5.0 {
                "increasing".to_string()
            } else if growth < -5.0 {
                "decreasing".to_string()
            } else {
                "stable".to_string()
            }
        } else {
            "stable".to_string()
        };

        // Détection de saisonnalité (simple: vérifier si variance > moyenne)
        let seasonality_detected = false; // TODO: implémenter analyse temporelle plus sophistiquée

        Ok(TrendsKpis {
            revenue_growth_percent,
            profit_growth_percent,
            orders_growth_percent,
            basket_value_growth_percent,
            global_trend,
            seasonality_detected,
        })
    }

    // ====================== 7. EFFICACITÉ OPÉRATIONNELLE ======================

    pub async fn get_operational_efficiency_kpis(
        pool: &PgPool,
        params: &KpiPeriodParams,
    ) -> Result<OperationalEfficiencyKpis, sqlx::Error> {
        let start_date = params.start_date.unwrap_or_else(|| {
            (chrono::Utc::now() - chrono::Duration::days(30)).date_naive()
        });
        let end_date = params.end_date.unwrap_or_else(|| chrono::Utc::now().date_naive());

        // Taux de rotation moyen
        let turnover_stats = sqlx::query(
            "SELECT
                AVG(CASE
                    WHEN p.stock_quantity_pro > 0
                    THEN sales.quantity_sold::float / p.stock_quantity_pro
                    ELSE NULL
                END) as avg_turnover,
                AVG(CASE
                    WHEN sales.quantity_sold > 0
                    THEN 360.0 / (sales.quantity_sold::float / p.stock_quantity_pro)
                    ELSE NULL
                END) as avg_storage_duration
            FROM products_pro p
            LEFT JOIN (
                SELECT product_id_lor, SUM(quantity_lor) as quantity_sold
                FROM line_order_lor lor
                JOIN order_ord o ON lor.order_id_lor = o.id_ord
                WHERE o.order_date_ord >= $1 AND o.order_date_ord <= $2
                GROUP BY product_id_lor
            ) sales ON p.id_pro = sales.product_id_lor"
        )
        .bind(start_date)
        .bind(end_date)
        .fetch_one(pool)
        .await?;

        let avg_catalog_turnover_rate = turnover_stats.try_get::<Option<f64>, _>("avg_turnover")?;
        let avg_storage_duration_days = turnover_stats.try_get::<Option<f64>, _>("avg_storage_duration")?;

        // Coût de stockage estimé (simplifié: 10% de la valeur du stock par an)
        let stock_value: f64 = sqlx::query(
            "SELECT COALESCE(SUM(stock_quantity_pro * buying_price_pro), 0) as value
            FROM products_pro"
        )
        .fetch_one(pool)
        .await?
        .try_get::<rust_decimal::Decimal, _>("value")?
        .to_f64()
        .unwrap_or(0.0);

        let estimated_storage_cost = stock_value * 0.10;

        // Taux de service (% commandes sans rupture)
        let service_stats = sqlx::query(
            "SELECT
                COUNT(DISTINCT o.id_ord) as total_orders,
                COUNT(DISTINCT CASE
                    WHEN p.stock_quantity_pro >= lor.quantity_lor
                    THEN o.id_ord
                END) as fulfilled_orders
            FROM order_ord o
            JOIN line_order_lor lor ON o.id_ord = lor.order_id_lor
            JOIN products_pro p ON lor.product_id_lor = p.id_pro
            WHERE o.order_date_ord >= $1 AND o.order_date_ord <= $2"
        )
        .bind(start_date)
        .bind(end_date)
        .fetch_one(pool)
        .await?;

        let total_orders: i64 = service_stats.try_get("total_orders")?;
        let fulfilled_orders: i64 = service_stats.try_get("fulfilled_orders")?;
        let service_rate = if total_orders > 0 {
            Some((fulfilled_orders as f64 / total_orders as f64) * 100.0)
        } else {
            None
        };

        // Taux de remplissage moyen (stock / capacité max)
        // Note: nécessite un champ "max_capacity" dans la table products_pro
        let avg_fill_rate = None; // TODO: implémenter si capacité max disponible

        // Fréquence moyenne de réapprovisionnement
        let restock_frequency = sqlx::query(
            "SELECT AVG(days_between) as avg_frequency
            FROM (
                SELECT
                    product_id_lrs,
                    restock_date_res - LAG(restock_date_res) OVER (PARTITION BY product_id_lrs ORDER BY restock_date_res) as days_between
                FROM line_restock_lrs lrs
                JOIN restock_res r ON lrs.restock_id_lrs = r.id_res
                WHERE r.status_res = 'received'
                    AND r.restock_date_res >= $1
                    AND r.restock_date_res <= $2
            ) gaps
            WHERE days_between IS NOT NULL"
        )
        .bind(start_date)
        .bind(end_date)
        .fetch_one(pool)
        .await?;

        let avg_restock_frequency_days = restock_frequency.try_get::<Option<i32>, _>("avg_frequency")?
            .map(|v| v as f64);

        Ok(OperationalEfficiencyKpis {
            avg_catalog_turnover_rate,
            avg_storage_duration_days,
            estimated_storage_cost,
            service_rate,
            avg_fill_rate,
            avg_restock_frequency_days,
        })
    }

    // ====================== 8. ANALYSES DE PRIX ======================

    pub async fn get_price_analysis_kpis(
        pool: &PgPool,
        params: &KpiPeriodParams,
    ) -> Result<PriceAnalysisKpis, sqlx::Error> {
        let start_date = params.start_date.unwrap_or_else(|| {
            (chrono::Utc::now() - chrono::Duration::days(30)).date_naive()
        });
        let end_date = params.end_date.unwrap_or_else(|| chrono::Utc::now().date_naive());

        // Prix moyens
        let avg_prices = sqlx::query(
            "SELECT
                AVG(p.buying_price_pro) as avg_buying,
                AVG(pp.price_prp) as avg_selling
            FROM products_pro p
            LEFT JOIN LATERAL (
                SELECT price_prp
                FROM productprices_prp
                WHERE product_ref_prp = p.id_pro
                ORDER BY created_at DESC
                LIMIT 1
            ) pp ON true"
        )
        .fetch_one(pool)
        .await?;

        let avg_buying_price = avg_prices.try_get::<Option<rust_decimal::Decimal>, _>("avg_buying")?
            .and_then(|v| v.to_f64());
        let avg_selling_price = avg_prices.try_get::<Option<rust_decimal::Decimal>, _>("avg_selling")?
            .and_then(|v| v.to_f64());

        // Marge moyenne pondérée par CA
        let weighted_margin = sqlx::query(
            "SELECT
                SUM(lor.line_total_lor) as total_revenue,
                SUM(lor.line_total_lor - (lor.quantity_lor * p.buying_price_pro)) as total_profit
            FROM line_order_lor lor
            JOIN order_ord o ON lor.order_id_lor = o.id_ord
            JOIN products_pro p ON lor.product_id_lor = p.id_pro
            WHERE o.order_date_ord >= $1 AND o.order_date_ord <= $2"
        )
        .bind(start_date)
        .bind(end_date)
        .fetch_one(pool)
        .await?;

        let total_revenue: f64 = weighted_margin.try_get::<rust_decimal::Decimal, _>("total_revenue")?.to_f64().unwrap_or(0.0);
        let total_profit: f64 = weighted_margin.try_get::<rust_decimal::Decimal, _>("total_profit")?.to_f64().unwrap_or(0.0);
        let weighted_avg_margin = if total_revenue > 0.0 {
            Some((total_profit / (total_revenue - total_profit)) * 100.0)
        } else {
            None
        };

        // Nombre de changements de prix
        let price_changes = sqlx::query(
            "SELECT
                (SELECT COUNT(*) FROM productprices_prp WHERE created_at >= $1 AND created_at <= $2) +
                (SELECT COUNT(*) FROM productrestockprices_prr WHERE created_at >= $1 AND created_at <= $2)
                as total_changes"
        )
        .bind(start_date)
        .bind(end_date)
        .fetch_one(pool)
        .await?;

        let price_changes_count: i64 = price_changes.try_get("total_changes")?;

        // Inflation moyenne prix d'achat
        let buying_inflation = sqlx::query(
            "SELECT
                AVG(first_price) as first_avg,
                AVG(last_price) as last_avg
            FROM (
                SELECT
                    product_ref_prr,
                    FIRST_VALUE(buying_price_prr) OVER (PARTITION BY product_ref_prr ORDER BY restock_date_prr) as first_price,
                    LAST_VALUE(buying_price_prr) OVER (PARTITION BY product_ref_prr ORDER BY restock_date_prr ROWS BETWEEN UNBOUNDED PRECEDING AND UNBOUNDED FOLLOWING) as last_price
                FROM productrestockprices_prr
                WHERE restock_date_prr >= $1 AND restock_date_prr <= $2
            ) prices"
        )
        .bind(start_date)
        .bind(end_date)
        .fetch_one(pool)
        .await?;

        let first_avg: f64 = buying_inflation.try_get::<Option<rust_decimal::Decimal>, _>("first_avg")?
            .and_then(|v| v.to_f64())
            .unwrap_or(0.0);
        let last_avg: f64 = buying_inflation.try_get::<Option<rust_decimal::Decimal>, _>("last_avg")?
            .and_then(|v| v.to_f64())
            .unwrap_or(0.0);
        let buying_price_inflation_percent = if first_avg > 0.0 {
            Some(((last_avg - first_avg) / first_avg) * 100.0)
        } else {
            None
        };

        // Évolution moyenne prix de vente
        let selling_evolution = sqlx::query(
            "SELECT
                AVG(first_price) as first_avg,
                AVG(last_price) as last_avg
            FROM (
                SELECT
                    product_ref_prp,
                    FIRST_VALUE(price_prp) OVER (PARTITION BY product_ref_prp ORDER BY created_at) as first_price,
                    LAST_VALUE(price_prp) OVER (PARTITION BY product_ref_prp ORDER BY created_at ROWS BETWEEN UNBOUNDED PRECEDING AND UNBOUNDED FOLLOWING) as last_price
                FROM productprices_prp
                WHERE created_at >= $1 AND created_at <= $2
            ) prices"
        )
        .bind(start_date)
        .bind(end_date)
        .fetch_one(pool)
        .await?;

        let first_selling: f64 = selling_evolution.try_get::<Option<rust_decimal::Decimal>, _>("first_avg")?
            .and_then(|v| v.to_f64())
            .unwrap_or(0.0);
        let last_selling: f64 = selling_evolution.try_get::<Option<rust_decimal::Decimal>, _>("last_avg")?
            .and_then(|v| v.to_f64())
            .unwrap_or(0.0);
        let selling_price_evolution_percent = if first_selling > 0.0 {
            Some(((last_selling - first_selling) / first_selling) * 100.0)
        } else {
            None
        };

        Ok(PriceAnalysisKpis {
            avg_buying_price,
            avg_selling_price,
            weighted_avg_margin,
            price_changes_count: price_changes_count as i32,
            buying_price_inflation_percent,
            selling_price_evolution_percent,
        })
    }

    // ====================== 9. TOP & FLOP ======================

    pub async fn get_top_flop_kpis(
        pool: &PgPool,
        params: &KpiPeriodParams,
    ) -> Result<TopFlopKpis, sqlx::Error> {
        let start_date = params.start_date.unwrap_or_else(|| {
            (chrono::Utc::now() - chrono::Duration::days(30)).date_naive()
        });
        let end_date = params.end_date.unwrap_or_else(|| chrono::Utc::now().date_naive());

        // Top 10 par CA
        let top_revenue = sqlx::query(
            "SELECT
                p.id_pro,
                p.name_pro,
                p.category_pro,
                COALESCE(SUM(lor.line_total_lor), 0) as value
            FROM products_pro p
            LEFT JOIN line_order_lor lor ON p.id_pro = lor.product_id_lor
            LEFT JOIN order_ord o ON lor.order_id_lor = o.id_ord
                AND o.order_date_ord >= $1 AND o.order_date_ord <= $2
            GROUP BY p.id_pro, p.name_pro, p.category_pro
            ORDER BY value DESC
            LIMIT 10"
        )
        .bind(start_date)
        .bind(end_date)
        .fetch_all(pool)
        .await?;

        let top_10_by_revenue: Vec<RankingProductInfo> = top_revenue.iter()
            .map(|row| RankingProductInfo {
                product_id: row.try_get("id_pro").unwrap(),
                product_name: row.try_get("name_pro").unwrap(),
                category: row.try_get("category_pro").unwrap(),
                value: row.try_get::<rust_decimal::Decimal, _>("value").unwrap().to_f64().unwrap_or(0.0),
            })
            .collect();

        // Top 10 par profit
        let top_profit = sqlx::query(
            "SELECT
                p.id_pro,
                p.name_pro,
                p.category_pro,
                COALESCE(SUM(lor.line_total_lor - (lor.quantity_lor * p.buying_price_pro)), 0) as value
            FROM products_pro p
            LEFT JOIN line_order_lor lor ON p.id_pro = lor.product_id_lor
            LEFT JOIN order_ord o ON lor.order_id_lor = o.id_ord
                AND o.order_date_ord >= $1 AND o.order_date_ord <= $2
            GROUP BY p.id_pro, p.name_pro, p.category_pro
            ORDER BY value DESC
            LIMIT 10"
        )
        .bind(start_date)
        .bind(end_date)
        .fetch_all(pool)
        .await?;

        let top_10_by_profit: Vec<RankingProductInfo> = top_profit.iter()
            .map(|row| RankingProductInfo {
                product_id: row.try_get("id_pro").unwrap(),
                product_name: row.try_get("name_pro").unwrap(),
                category: row.try_get("category_pro").unwrap(),
                value: row.try_get::<rust_decimal::Decimal, _>("value").unwrap().to_f64().unwrap_or(0.0),
            })
            .collect();

        // Top 10 par volume
        let top_volume = sqlx::query(
            "SELECT
                p.id_pro,
                p.name_pro,
                p.category_pro,
                COALESCE(SUM(lor.quantity_lor), 0) as value
            FROM products_pro p
            LEFT JOIN line_order_lor lor ON p.id_pro = lor.product_id_lor
            LEFT JOIN order_ord o ON lor.order_id_lor = o.id_ord
                AND o.order_date_ord >= $1 AND o.order_date_ord <= $2
            GROUP BY p.id_pro, p.name_pro, p.category_pro
            ORDER BY value DESC
            LIMIT 10"
        )
        .bind(start_date)
        .bind(end_date)
        .fetch_all(pool)
        .await?;

        let top_10_by_volume: Vec<RankingProductInfo> = top_volume.iter()
            .map(|row| RankingProductInfo {
                product_id: row.try_get("id_pro").unwrap(),
                product_name: row.try_get("name_pro").unwrap(),
                category: row.try_get("category_pro").unwrap(),
                value: row.try_get::<i64, _>("value").unwrap() as f64,
            })
            .collect();

        // Top 10 par rotation
        let top_turnover = sqlx::query(
            "SELECT
                p.id_pro,
                p.name_pro,
                p.category_pro,
                CASE
                    WHEN p.stock_quantity_pro > 0
                    THEN COALESCE(SUM(lor.quantity_lor), 0)::float / p.stock_quantity_pro
                    ELSE 0
                END as value
            FROM products_pro p
            LEFT JOIN line_order_lor lor ON p.id_pro = lor.product_id_lor
            LEFT JOIN order_ord o ON lor.order_id_lor = o.id_ord
                AND o.order_date_ord >= $1 AND o.order_date_ord <= $2
            GROUP BY p.id_pro, p.name_pro, p.category_pro, p.stock_quantity_pro
            ORDER BY value DESC
            LIMIT 10"
        )
        .bind(start_date)
        .bind(end_date)
        .fetch_all(pool)
        .await?;

        let top_10_by_turnover: Vec<RankingProductInfo> = top_turnover.iter()
            .map(|row| RankingProductInfo {
                product_id: row.try_get("id_pro").unwrap(),
                product_name: row.try_get("name_pro").unwrap(),
                category: row.try_get("category_pro").unwrap(),
                value: row.try_get::<f64, _>("value").unwrap(),
            })
            .collect();

        // Flop 10 par ventes
        let flop_sales = sqlx::query(
            "SELECT
                p.id_pro,
                p.name_pro,
                p.category_pro,
                COALESCE(SUM(lor.quantity_lor), 0) as value
            FROM products_pro p
            LEFT JOIN line_order_lor lor ON p.id_pro = lor.product_id_lor
            LEFT JOIN order_ord o ON lor.order_id_lor = o.id_ord
                AND o.order_date_ord >= $1 AND o.order_date_ord <= $2
            WHERE p.status_pro != 'discontinued'
            GROUP BY p.id_pro, p.name_pro, p.category_pro
            ORDER BY value ASC
            LIMIT 10"
        )
        .bind(start_date)
        .bind(end_date)
        .fetch_all(pool)
        .await?;

        let flop_10_by_sales: Vec<RankingProductInfo> = flop_sales.iter()
            .map(|row| RankingProductInfo {
                product_id: row.try_get("id_pro").unwrap(),
                product_name: row.try_get("name_pro").unwrap(),
                category: row.try_get("category_pro").unwrap(),
                value: row.try_get::<i64, _>("value").unwrap() as f64,
            })
            .collect();

        // Flop 10 par profit
        let flop_profit = sqlx::query(
            "SELECT
                p.id_pro,
                p.name_pro,
                p.category_pro,
                COALESCE(SUM(lor.line_total_lor - (lor.quantity_lor * p.buying_price_pro)), 0) as value
            FROM products_pro p
            LEFT JOIN line_order_lor lor ON p.id_pro = lor.product_id_lor
            LEFT JOIN order_ord o ON lor.order_id_lor = o.id_ord
                AND o.order_date_ord >= $1 AND o.order_date_ord <= $2
            WHERE p.status_pro != 'discontinued'
            GROUP BY p.id_pro, p.name_pro, p.category_pro
            ORDER BY value ASC
            LIMIT 10"
        )
        .bind(start_date)
        .bind(end_date)
        .fetch_all(pool)
        .await?;

        let flop_10_by_profit: Vec<RankingProductInfo> = flop_profit.iter()
            .map(|row| RankingProductInfo {
                product_id: row.try_get("id_pro").unwrap(),
                product_name: row.try_get("name_pro").unwrap(),
                category: row.try_get("category_pro").unwrap(),
                value: row.try_get::<rust_decimal::Decimal, _>("value").unwrap().to_f64().unwrap_or(0.0),
            })
            .collect();

        // Produits à risque (faible rotation + stock élevé)
        let at_risk = sqlx::query(
            "SELECT
                p.id_pro,
                p.name_pro,
                p.category_pro,
                p.stock_quantity_pro as value
            FROM products_pro p
            LEFT JOIN (
                SELECT product_id_lor, SUM(quantity_lor) as sold
                FROM line_order_lor lor
                JOIN order_ord o ON lor.order_id_lor = o.id_ord
                WHERE o.order_date_ord >= $1 AND o.order_date_ord <= $2
                GROUP BY product_id_lor
            ) sales ON p.id_pro = sales.product_id_lor
            WHERE p.stock_quantity_pro > 100
                AND COALESCE(sales.sold, 0) < 10
                AND p.status_pro != 'discontinued'
            ORDER BY p.stock_quantity_pro DESC
            LIMIT 10"
        )
        .bind(start_date)
        .bind(end_date)
        .fetch_all(pool)
        .await?;

        let at_risk_products: Vec<RankingProductInfo> = at_risk.iter()
            .map(|row| RankingProductInfo {
                product_id: row.try_get("id_pro").unwrap(),
                product_name: row.try_get("name_pro").unwrap(),
                category: row.try_get("category_pro").unwrap(),
                value: row.try_get::<i32, _>("value").unwrap() as f64,
            })
            .collect();

        Ok(TopFlopKpis {
            top_10_by_revenue,
            top_10_by_profit,
            top_10_by_volume,
            top_10_by_turnover,
            flop_10_by_sales,
            flop_10_by_profit,
            at_risk_products,
        })
    }

    // ====================== 10. PRÉVISIONS GLOBALES ======================

    pub async fn get_forecast_kpis(
        pool: &PgPool,
        params: &KpiPeriodParams,
    ) -> Result<ForecastKpis, sqlx::Error> {
        let start_date = params.start_date.unwrap_or_else(|| {
            (chrono::Utc::now() - chrono::Duration::days(30)).date_naive()
        });
        let end_date = params.end_date.unwrap_or_else(|| chrono::Utc::now().date_naive());

        // CA actuel sur la période
        let current_revenue: f64 = sqlx::query(
            "SELECT COALESCE(SUM(line_total_lor), 0) as revenue
            FROM line_order_lor lor
            JOIN order_ord o ON lor.order_id_lor = o.id_ord
            WHERE o.order_date_ord >= $1 AND o.order_date_ord <= $2"
        )
        .bind(start_date)
        .bind(end_date)
        .fetch_one(pool)
        .await?
        .try_get::<rust_decimal::Decimal, _>("revenue")?
        .to_f64()
        .unwrap_or(0.0);

        // Durée de la période en jours
        let period_days = (end_date - start_date).num_days() as f64;
        let daily_revenue = if period_days > 0.0 {
            current_revenue / period_days
        } else {
            0.0
        };

        // Prévisions (simple: extrapolation linéaire)
        let forecasted_revenue_next_month = Some(daily_revenue * 30.0);
        let forecasted_revenue_next_3_months = Some(daily_revenue * 90.0);

        // Besoin en trésorerie pour restocks
        let cash_needed = sqlx::query(
            "SELECT COALESCE(SUM(lrs.total_price_lrs), 0) as total
            FROM line_restock_lrs lrs
            JOIN restock_res r ON lrs.restock_id_lrs = r.id_res
            WHERE r.status_res = 'pending'"
        )
        .fetch_one(pool)
        .await?
        .try_get::<rust_decimal::Decimal, _>("total")?
        .to_f64()
        .unwrap_or(0.0);

        // Nombre de ruptures prévisibles (produits avec couverture < 7 jours)
        let predicted_stockouts: i64 = sqlx::query(
            "SELECT COUNT(*) as count
            FROM (
                SELECT
                    p.id_pro,
                    p.stock_quantity_pro,
                    COALESCE(SUM(lor.quantity_lor), 0) / $3 as daily_sales
                FROM products_pro p
                LEFT JOIN line_order_lor lor ON p.id_pro = lor.product_id_lor
                LEFT JOIN order_ord o ON lor.order_id_lor = o.id_ord
                    AND o.order_date_ord >= $1 AND o.order_date_ord <= $2
                GROUP BY p.id_pro, p.stock_quantity_pro
                HAVING (COALESCE(SUM(lor.quantity_lor), 0) / $3) > 0
                    AND p.stock_quantity_pro / (COALESCE(SUM(lor.quantity_lor), 0) / $3) < 7
            ) at_risk"
        )
        .bind(start_date)
        .bind(end_date)
        .bind(period_days)
        .fetch_one(pool)
        .await?
        .try_get("count")?;

        // Opportunités d'optimisation (produits surstockés ou sous-performants)
        let optimization_count: i64 = sqlx::query(
            "SELECT COUNT(*) as count
            FROM (
                SELECT p.id_pro
                FROM products_pro p
                LEFT JOIN (
                    SELECT product_id_lor, SUM(quantity_lor) as sold
                    FROM line_order_lor lor
                    JOIN order_ord o ON lor.order_id_lor = o.id_ord
                    WHERE o.order_date_ord >= $1 AND o.order_date_ord <= $2
                    GROUP BY product_id_lor
                ) sales ON p.id_pro = sales.product_id_lor
                WHERE (p.stock_quantity_pro > COALESCE(sales.sold, 0) * 3)
                    OR (COALESCE(sales.sold, 0) = 0 AND p.stock_quantity_pro > 0)
            ) opportunities"
        )
        .bind(start_date)
        .bind(end_date)
        .fetch_one(pool)
        .await?
        .try_get("count")?;

        Ok(ForecastKpis {
            forecasted_revenue_next_month,
            forecasted_revenue_next_3_months,
            cash_needed_for_restocks: cash_needed,
            predicted_stockouts_count: predicted_stockouts as i32,
            optimization_opportunities_count: optimization_count as i32,
        })
    }

    // ====================== ÉVOLUTIONS TEMPORELLES ======================

    pub async fn get_time_series_kpis(
        pool: &PgPool,
        params: &KpiPeriodParams,
    ) -> Result<TimeSeriesKpis, sqlx::Error> {
        let start_date = params.start_date.unwrap_or_else(|| {
            (chrono::Utc::now() - chrono::Duration::days(30)).date_naive()
        });
        let end_date = params.end_date.unwrap_or_else(|| chrono::Utc::now().date_naive());

        // Historique du CA par jour
        let revenue_rows = sqlx::query(
            "SELECT
                o.order_date_ord as date,
                COALESCE(SUM(lor.line_total_lor), 0) as value
            FROM order_ord o
            LEFT JOIN line_order_lor lor ON o.id_ord = lor.order_id_lor
            WHERE o.order_date_ord >= $1 AND o.order_date_ord <= $2
            GROUP BY o.order_date_ord
            ORDER BY o.order_date_ord"
        )
        .bind(start_date)
        .bind(end_date)
        .fetch_all(pool)
        .await?;

        let revenue_history: Vec<TimeSeriesPoint> = revenue_rows.iter()
            .map(|row| TimeSeriesPoint {
                date: row.try_get("date").unwrap(),
                value: row.try_get::<rust_decimal::Decimal, _>("value").unwrap().to_f64().unwrap_or(0.0),
            })
            .collect();

        // Historique du profit par jour
        let profit_rows = sqlx::query(
            "SELECT
                o.order_date_ord as date,
                COALESCE(SUM(lor.line_total_lor - (lor.quantity_lor * p.buying_price_pro)), 0) as value
            FROM order_ord o
            LEFT JOIN line_order_lor lor ON o.id_ord = lor.order_id_lor
            LEFT JOIN products_pro p ON lor.product_id_lor = p.id_pro
            WHERE o.order_date_ord >= $1 AND o.order_date_ord <= $2
            GROUP BY o.order_date_ord
            ORDER BY o.order_date_ord"
        )
        .bind(start_date)
        .bind(end_date)
        .fetch_all(pool)
        .await?;

        let profit_history: Vec<TimeSeriesPoint> = profit_rows.iter()
            .map(|row| TimeSeriesPoint {
                date: row.try_get("date").unwrap(),
                value: row.try_get::<rust_decimal::Decimal, _>("value").unwrap().to_f64().unwrap_or(0.0),
            })
            .collect();

        // Historique du nombre de commandes par jour
        let orders_rows = sqlx::query(
            "SELECT
                order_date_ord as date,
                COUNT(*) as value
            FROM order_ord
            WHERE order_date_ord >= $1 AND order_date_ord <= $2
            GROUP BY order_date_ord
            ORDER BY order_date_ord"
        )
        .bind(start_date)
        .bind(end_date)
        .fetch_all(pool)
        .await?;

        let orders_history: Vec<TimeSeriesPoint> = orders_rows.iter()
            .map(|row| TimeSeriesPoint {
                date: row.try_get("date").unwrap(),
                value: row.try_get::<i64, _>("value").unwrap() as f64,
            })
            .collect();

        // Historique du panier moyen par jour
        let basket_rows = sqlx::query(
            "SELECT
                o.order_date_ord as date,
                AVG(order_totals.total) as value
            FROM order_ord o
            LEFT JOIN (
                SELECT order_id_lor, SUM(line_total_lor) as total
                FROM line_order_lor
                GROUP BY order_id_lor
            ) order_totals ON o.id_ord = order_totals.order_id_lor
            WHERE o.order_date_ord >= $1 AND o.order_date_ord <= $2
            GROUP BY o.order_date_ord
            ORDER BY o.order_date_ord"
        )
        .bind(start_date)
        .bind(end_date)
        .fetch_all(pool)
        .await?;

        let basket_value_history: Vec<TimeSeriesPoint> = basket_rows.iter()
            .map(|row| TimeSeriesPoint {
                date: row.try_get("date").unwrap(),
                value: row.try_get::<Option<rust_decimal::Decimal>, _>("value")
                    .ok()
                    .flatten()
                    .and_then(|v| v.to_f64())
                    .unwrap_or(0.0),
            })
            .collect();

        Ok(TimeSeriesKpis {
            revenue_history,
            profit_history,
            orders_history,
            basket_value_history,
        })
    }
}
