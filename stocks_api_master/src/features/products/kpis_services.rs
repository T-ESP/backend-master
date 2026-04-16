use sqlx::{PgPool, Row};
use rust_decimal::prelude::ToPrimitive;
use super::{dto::ProductStatus, kpis_dto::*};

pub struct ProductKpisService;

impl ProductKpisService {
    // ====================== 1. PRIX & MARGE ======================

    pub async fn get_pricing_margin_kpis(
        pool: &PgPool,
        product_id: i32,
        params: &KpiPeriodParams,
    ) -> Result<PricingMarginKpis, sqlx::Error> {
        let start_date = params.start_date.unwrap_or_else(|| {
            (chrono::Utc::now() - chrono::Duration::days(30)).date_naive()
        });
        let end_date = params.end_date.unwrap_or_else(|| chrono::Utc::now().date_naive());

        let current_product = sqlx::query(
            "SELECT buying_price_pro FROM products_pro WHERE id_pro = $1"
        )
        .bind(product_id)
        .fetch_one(pool)
        .await?;

        let current_buying_price: f64 = current_product.try_get::<rust_decimal::Decimal, _>("buying_price_pro")?
            .to_f64()
            .unwrap_or(0.0);

        let current_selling_price_row = sqlx::query(
            "SELECT price_prp
             FROM productprices_prp
             WHERE product_ref_prp = $1
             ORDER BY created_at DESC
             LIMIT 1"
        )
        .bind(product_id)
        .fetch_optional(pool)
        .await?;

        let current_selling_price = current_selling_price_row.map(|r| {
            r.try_get::<rust_decimal::Decimal, _>("price_prp")
                .ok()
                .and_then(|v| v.to_f64())
                .unwrap_or(0.0)
        });

        let (gross_margin, margin_rate) = if let Some(selling_price) = current_selling_price {
            let margin = selling_price - current_buying_price;
            let rate = if current_buying_price > 0.0 {
                (margin / current_buying_price) * 100.0
            } else {
                0.0
            };
            (Some(margin), Some(rate))
        } else {
            (None, None)
        };

        let buying_stats = sqlx::query(
            "SELECT
                MIN(buying_price_prr) as min_price,
                MAX(buying_price_prr) as max_price,
                AVG(buying_price_prr) as avg_price,
                STDDEV(buying_price_prr) as volatility,
                COUNT(*) as changes_count
            FROM productrestockprices_prr
            WHERE product_ref_prr = $1
              AND restock_date_prr >= $2
              AND restock_date_prr <= $3"
        )
        .bind(product_id)
        .bind(start_date)
        .bind(end_date)
        .fetch_one(pool)
        .await?;

        let buying_price_min = buying_stats.try_get::<Option<rust_decimal::Decimal>, _>("min_price")?
            .and_then(|v| v.to_f64());
        let buying_price_max = buying_stats.try_get::<Option<rust_decimal::Decimal>, _>("max_price")?
            .and_then(|v| v.to_f64());
        let buying_price_avg = buying_stats.try_get::<Option<rust_decimal::Decimal>, _>("avg_price")?
            .and_then(|v| v.to_f64());
        let buying_price_volatility = buying_stats.try_get::<Option<rust_decimal::Decimal>, _>("volatility")?
            .and_then(|v| v.to_f64());
        let buying_price_changes_count: i64 = buying_stats.try_get("changes_count")?;

        let buying_price_variation = if let (Some(min), Some(max)) = (buying_price_min, buying_price_max) {
            if min > 0.0 {
                Some(((max - min) / min) * 100.0)
            } else {
                None
            }
        } else {
            None
        };

        let selling_stats = sqlx::query(
            "SELECT
                MIN(price_prp) as min_price,
                MAX(price_prp) as max_price,
                AVG(price_prp) as avg_price,
                STDDEV(price_prp) as volatility,
                COUNT(*) as changes_count
            FROM productprices_prp
            WHERE product_ref_prp = $1
              AND created_at >= $2
              AND created_at <= $3"
        )
        .bind(product_id)
        .bind(start_date)
        .bind(end_date)
        .fetch_one(pool)
        .await?;

        let selling_price_min = selling_stats.try_get::<Option<rust_decimal::Decimal>, _>("min_price")?
            .and_then(|v| v.to_f64());
        let selling_price_max = selling_stats.try_get::<Option<rust_decimal::Decimal>, _>("max_price")?
            .and_then(|v| v.to_f64());
        let selling_price_avg = selling_stats.try_get::<Option<rust_decimal::Decimal>, _>("avg_price")?
            .and_then(|v| v.to_f64());
        let selling_price_volatility = selling_stats.try_get::<Option<rust_decimal::Decimal>, _>("volatility")?
            .and_then(|v| v.to_f64());
        let selling_price_changes_count: i64 = selling_stats.try_get("changes_count")?;

        let selling_price_variation = if let (Some(min), Some(max)) = (selling_price_min, selling_price_max) {
            if min > 0.0 {
                Some(((max - min) / min) * 100.0)
            } else {
                None
            }
        } else {
            None
        };

        Ok(PricingMarginKpis {
            current_buying_price,
            current_selling_price,
            gross_margin,
            margin_rate,
            buying_price_min,
            buying_price_max,
            buying_price_avg,
            buying_price_variation,
            buying_price_volatility,
            buying_price_changes_count: buying_price_changes_count as i32,
            selling_price_min,
            selling_price_max,
            selling_price_avg,
            selling_price_variation,
            selling_price_volatility,
            selling_price_changes_count: selling_price_changes_count as i32,
            repercussion_rate: None,
            repercussion_delay_days: None,
        })
    }

    // ====================== 2. STOCK & DISPONIBILITÉ ======================

    pub async fn get_stock_availability_kpis(
        pool: &PgPool,
        product_id: i32,
        params: &KpiPeriodParams,
    ) -> Result<StockAvailabilityKpis, sqlx::Error> {
        let start_date = params.start_date.unwrap_or_else(|| {
            (chrono::Utc::now() - chrono::Duration::days(30)).date_naive()
        });
        let end_date = params.end_date.unwrap_or_else(|| chrono::Utc::now().date_naive());

        let product = sqlx::query(
            "SELECT stock_quantity_pro, status_pro, date_last_reassor_pro
             FROM products_pro
             WHERE id_pro = $1"
        )
        .bind(product_id)
        .fetch_one(pool)
        .await?;

        let current_stock: i32 = product.try_get("stock_quantity_pro")?;
        let product_status: ProductStatus = product.try_get("status_pro")?;
        let product_status_str = match product_status {
            ProductStatus::InStock => "in_stock",
            ProductStatus::OutOfStock => "out_of_stock",
            ProductStatus::Discontinued => "discontinued",
            ProductStatus::Ordered => "ordered",
        }
        .to_string();
        let date_last_reassor: chrono::DateTime<chrono::Utc> = product.try_get("date_last_reassor_pro")?;

        let days_since_last_restock = (chrono::Utc::now() - date_last_reassor).num_days() as i32;

        let stockout_stats = sqlx::query(
            "SELECT COUNT(DISTINCT r.id_res) as stockout_count
             FROM restock_res r
             JOIN line_restock_lrs lr ON r.id_res = lr.restock_id_lrs
             WHERE lr.product_id_lrs = $1
               AND r.status_res = 'received'
               AND r.restock_date_res >= $2
               AND r.restock_date_res <= $3"
        )
        .bind(product_id)
        .bind(start_date)
        .bind(end_date)
        .fetch_one(pool)
        .await?;

        let stockout_count: i64 = stockout_stats.try_get("stockout_count")?;

        let avg_duration_row = sqlx::query(
            "SELECT AVG(days_between) as avg_days
             FROM (
                 SELECT
                     r.restock_date_res - LAG(r.restock_date_res) OVER (ORDER BY r.restock_date_res) as days_between
                 FROM restock_res r
                 JOIN line_restock_lrs lr ON r.id_res = lr.restock_id_lrs
                 WHERE lr.product_id_lrs = $1
                   AND r.status_res = 'received'
                   AND r.restock_date_res >= $2
                   AND r.restock_date_res <= $3
             ) as gaps
             WHERE days_between IS NOT NULL"
        )
        .bind(product_id)
        .bind(start_date)
        .bind(end_date)
        .fetch_optional(pool)
        .await?;

        let avg_stockout_duration_days = if let Some(row) = avg_duration_row {
            row.try_get::<Option<f64>, _>("avg_days").ok().flatten()
        } else {
            None
        };

        let total_days = (end_date - start_date).num_days() as f64;
        let stockout_rate = if stockout_count > 0 && total_days > 0.0 {
            let estimated_stockout_days = stockout_count as f64 * avg_stockout_duration_days.unwrap_or(3.0);
            Some((estimated_stockout_days / total_days) * 100.0)
        } else {
            Some(0.0)
        };

        let sales_velocity = sqlx::query(
            "SELECT COALESCE(SUM(lor.quantity_lor), 0) as total_sold
             FROM line_order_lor lor
             JOIN order_ord o ON lor.order_id_lor = o.id_ord
             WHERE lor.product_id_lor = $1
               AND o.order_date_ord >= $2
               AND o.order_date_ord <= $3"
        )
        .bind(product_id)
        .bind(start_date)
        .bind(end_date)
        .fetch_one(pool)
        .await?;

        let total_sold: i64 = sales_velocity.try_get("total_sold")?;
        let daily_sales_velocity = if total_days > 0.0 {
            total_sold as f64 / total_days
        } else {
            0.0
        };

        let safety_stock_recommended = if daily_sales_velocity > 0.0 {
            Some((daily_sales_velocity * 7.0).ceil() as i32)
        } else {
            Some(0)
        };

        Ok(StockAvailabilityKpis {
            current_stock,
            product_status: product_status_str,
            stockout_rate,
            stockout_count: stockout_count as i32,
            avg_stockout_duration_days,
            safety_stock_recommended,
            days_since_last_restock: Some(days_since_last_restock),
            last_restock_date: Some(date_last_reassor),
        })
    }

    // ====================== 3. VENTES & ROTATION ======================

    pub async fn get_sales_rotation_kpis(
        pool: &PgPool,
        product_id: i32,
        params: &KpiPeriodParams,
    ) -> Result<SalesRotationKpis, sqlx::Error> {
        let start_date = params.start_date.unwrap_or_else(|| {
            (chrono::Utc::now() - chrono::Duration::days(30)).date_naive()
        });
        let end_date = params.end_date.unwrap_or_else(|| chrono::Utc::now().date_naive());

        let sales_stats = sqlx::query(
            "SELECT
                COALESCE(SUM(lor.quantity_lor), 0) as quantity_sold,
                COALESCE(SUM(lor.line_total_lor), 0) as revenue,
                COUNT(DISTINCT lor.order_id_lor) as order_count,
                AVG(lor.quantity_lor) as avg_quantity_per_order
            FROM line_order_lor lor
            JOIN order_ord o ON lor.order_id_lor = o.id_ord
            WHERE lor.product_id_lor = $1
              AND o.order_date_ord >= $2
              AND o.order_date_ord <= $3"
        )
        .bind(product_id)
        .bind(start_date)
        .bind(end_date)
        .fetch_one(pool)
        .await?;

        let quantity_sold: i64 = sales_stats.try_get("quantity_sold")?;
        let revenue = sales_stats.try_get::<rust_decimal::Decimal, _>("revenue")?
            .to_f64()
            .unwrap_or(0.0);
        let order_count: i64 = sales_stats.try_get("order_count")?;
        let avg_quantity_per_order = sales_stats.try_get::<Option<rust_decimal::Decimal>, _>("avg_quantity_per_order")?
            .and_then(|v| v.to_f64());

        let avg_basket_value = if order_count > 0 {
            Some(revenue / order_count as f64)
        } else {
            None
        };

        let stock = sqlx::query("SELECT stock_quantity_pro FROM products_pro WHERE id_pro = $1")
            .bind(product_id)
            .fetch_one(pool)
            .await?;

        let stock_quantity: i32 = stock.try_get("stock_quantity_pro")?;

        let stock_turnover_rate = if stock_quantity > 0 {
            Some(quantity_sold as f64 / stock_quantity as f64)
        } else {
            None
        };

        let avg_storage_duration_days = if let Some(turnover) = stock_turnover_rate {
            if turnover > 0.0 {
                Some(360.0 / turnover)
            } else {
                None
            }
        } else {
            None
        };

        let days_in_period = (end_date - start_date).num_days() as f64;
        let sales_velocity_per_day = if days_in_period > 0.0 {
            Some(quantity_sold as f64 / days_in_period)
        } else {
            None
        };

        let mid_date = start_date + chrono::Duration::days((days_in_period / 2.0) as i64);
        let first_half = sqlx::query(
            "SELECT COALESCE(SUM(lor.quantity_lor), 0) as qty
             FROM line_order_lor lor
             JOIN order_ord o ON lor.order_id_lor = o.id_ord
             WHERE lor.product_id_lor = $1
               AND o.order_date_ord >= $2 AND o.order_date_ord < $3"
        )
        .bind(product_id)
        .bind(start_date)
        .bind(mid_date)
        .fetch_one(pool)
        .await?;
        let second_half = sqlx::query(
            "SELECT COALESCE(SUM(lor.quantity_lor), 0) as qty
             FROM line_order_lor lor
             JOIN order_ord o ON lor.order_id_lor = o.id_ord
             WHERE lor.product_id_lor = $1
               AND o.order_date_ord >= $2 AND o.order_date_ord <= $3"
        )
        .bind(product_id)
        .bind(mid_date)
        .bind(end_date)
        .fetch_one(pool)
        .await?;
        let first_qty: i64 = first_half.try_get("qty").unwrap_or(0);
        let second_qty: i64 = second_half.try_get("qty").unwrap_or(0);
        let sales_trend = if first_qty == 0 && second_qty == 0 {
            "no_data"
        } else if second_qty > (first_qty as f64 * 1.1) as i64 {
            "growing"
        } else if second_qty < (first_qty as f64 * 0.9) as i64 {
            "declining"
        } else {
            "stable"
        }.to_string();

        Ok(SalesRotationKpis {
            quantity_sold: quantity_sold as i32,
            revenue,
            order_count: order_count as i32,
            avg_quantity_per_order,
            avg_basket_value,
            stock_turnover_rate,
            avg_storage_duration_days,
            sales_velocity_per_day,
            sales_trend,
            sales_variation_percent: None,
        })
    }

    // ====================== 4. RENTABILITÉ ======================

    pub async fn get_profitability_kpis(
        pool: &PgPool,
        product_id: i32,
        params: &KpiPeriodParams,
    ) -> Result<ProfitabilityKpis, sqlx::Error> {
        let start_date = params.start_date.unwrap_or_else(|| {
            (chrono::Utc::now() - chrono::Duration::days(30)).date_naive()
        });
        let end_date = params.end_date.unwrap_or_else(|| chrono::Utc::now().date_naive());

        let product = sqlx::query("SELECT buying_price_pro FROM products_pro WHERE id_pro = $1")
            .bind(product_id)
            .fetch_one(pool)
            .await?;

        let buying_price = product.try_get::<rust_decimal::Decimal, _>("buying_price_pro")?
            .to_f64()
            .unwrap_or(0.0);

        let sales = sqlx::query(
            "SELECT
                COALESCE(SUM(lor.quantity_lor), 0) as quantity_sold,
                COALESCE(SUM(lor.line_total_lor), 0) as revenue
            FROM line_order_lor lor
            JOIN order_ord o ON lor.order_id_lor = o.id_ord
            WHERE lor.product_id_lor = $1
              AND o.order_date_ord >= $2
              AND o.order_date_ord <= $3"
        )
        .bind(product_id)
        .bind(start_date)
        .bind(end_date)
        .fetch_one(pool)
        .await?;

        let quantity_sold: i64 = sales.try_get("quantity_sold")?;
        let revenue = sales.try_get::<rust_decimal::Decimal, _>("revenue")?
            .to_f64()
            .unwrap_or(0.0);

        let cost = quantity_sold as f64 * buying_price;
        let total_profit = revenue - cost;

        let avg_profit_per_sale = if quantity_sold > 0 {
            Some(total_profit / quantity_sold as f64)
        } else {
            None
        };

        let roi = if cost > 0.0 {
            Some((total_profit / cost) * 100.0)
        } else {
            None
        };

        let total_revenue_row = sqlx::query(
            "SELECT COALESCE(SUM(lor.line_total_lor), 0) as total_revenue
             FROM line_order_lor lor
             JOIN order_ord o ON lor.order_id_lor = o.id_ord
             WHERE o.order_date_ord >= $1
               AND o.order_date_ord <= $2"
        )
        .bind(start_date)
        .bind(end_date)
        .fetch_one(pool)
        .await?;

        let total_revenue_all = total_revenue_row.try_get::<rust_decimal::Decimal, _>("total_revenue")?
            .to_f64()
            .unwrap_or(0.0);

        let contribution_to_total_revenue_percent = if total_revenue_all > 0.0 {
            Some((revenue / total_revenue_all) * 100.0)
        } else {
            None
        };

        let total_profit_row = sqlx::query(
            "SELECT
                COALESCE(SUM(lor.line_total_lor), 0) as total_revenue,
                COALESCE(SUM(lor.quantity_lor * p.buying_price_pro), 0) as total_cost
             FROM line_order_lor lor
             JOIN order_ord o ON lor.order_id_lor = o.id_ord
             JOIN products_pro p ON lor.product_id_lor = p.id_pro
             WHERE o.order_date_ord >= $1
               AND o.order_date_ord <= $2"
        )
        .bind(start_date)
        .bind(end_date)
        .fetch_one(pool)
        .await?;

        let total_revenue_all_for_profit = total_profit_row.try_get::<rust_decimal::Decimal, _>("total_revenue")?
            .to_f64()
            .unwrap_or(0.0);
        let total_cost_all = total_profit_row.try_get::<rust_decimal::Decimal, _>("total_cost")?
            .to_f64()
            .unwrap_or(0.0);

        let total_profit_all = total_revenue_all_for_profit - total_cost_all;

        let contribution_to_total_profit_percent = if total_profit_all > 0.0 {
            Some((total_profit / total_profit_all) * 100.0)
        } else {
            None
        };

        Ok(ProfitabilityKpis {
            total_profit,
            avg_profit_per_sale,
            roi,
            contribution_to_total_revenue_percent,
            contribution_to_total_profit_percent,
        })
    }

    // ====================== 5. RÉAPPROVISIONNEMENT ======================

    pub async fn get_restock_kpis(
        pool: &PgPool,
        product_id: i32,
        params: &KpiPeriodParams,
    ) -> Result<RestockKpis, sqlx::Error> {
        let start_date = params.start_date.unwrap_or_else(|| {
            (chrono::Utc::now() - chrono::Duration::days(30)).date_naive()
        });
        let end_date = params.end_date.unwrap_or_else(|| chrono::Utc::now().date_naive());

        let restock_stats = sqlx::query(
            "SELECT
                COUNT(DISTINCT lr.restock_id_lrs) as restock_count,
                COALESCE(SUM(lr.quantity_lrs), 0) as total_quantity,
                AVG(lr.quantity_lrs) as avg_quantity,
                COALESCE(SUM(lr.total_price_lrs), 0) as total_cost,
                AVG(lr.total_price_lrs) as avg_cost
            FROM line_restock_lrs lr
            JOIN restock_res r ON lr.restock_id_lrs = r.id_res
            WHERE lr.product_id_lrs = $1
              AND r.restock_date_res >= $2
              AND r.restock_date_res <= $3"
        )
        .bind(product_id)
        .bind(start_date)
        .bind(end_date)
        .fetch_one(pool)
        .await?;

        let restock_count: i64 = restock_stats.try_get("restock_count")?;
        let total_restocked_quantity: i64 = restock_stats.try_get("total_quantity")?;
        let avg_quantity_per_restock = restock_stats.try_get::<Option<rust_decimal::Decimal>, _>("avg_quantity")?
            .and_then(|v| v.to_f64());
        let total_restock_cost = restock_stats.try_get::<rust_decimal::Decimal, _>("total_cost")?
            .to_f64()
            .unwrap_or(0.0);
        let avg_restock_cost = restock_stats.try_get::<Option<rust_decimal::Decimal>, _>("avg_cost")?
            .and_then(|v| v.to_f64());

        let days_in_period = (end_date - start_date).num_days() as f64;
        let restock_frequency_days = if restock_count > 1 {
            Some(days_in_period / restock_count as f64)
        } else {
            None
        };

        let status_stats = sqlx::query(
            "SELECT
                COUNT(CASE WHEN r.status_res = 'received' THEN 1 END) as received_count,
                COUNT(CASE WHEN r.status_res = 'cancelled' THEN 1 END) as cancelled_count,
                COUNT(*) as total_count
            FROM line_restock_lrs lr
            JOIN restock_res r ON lr.restock_id_lrs = r.id_res
            WHERE lr.product_id_lrs = $1
              AND r.restock_date_res >= $2
              AND r.restock_date_res <= $3"
        )
        .bind(product_id)
        .bind(start_date)
        .bind(end_date)
        .fetch_one(pool)
        .await?;

        let received_count: i64 = status_stats.try_get("received_count")?;
        let cancelled_count: i64 = status_stats.try_get("cancelled_count")?;
        let total_count: i64 = status_stats.try_get("total_count")?;

        let reception_rate = if total_count > 0 {
            Some((received_count as f64 / total_count as f64) * 100.0)
        } else {
            None
        };

        let cancellation_rate = if total_count > 0 {
            Some((cancelled_count as f64 / total_count as f64) * 100.0)
        } else {
            None
        };

        Ok(RestockKpis {
            restock_count: restock_count as i32,
            total_restocked_quantity: total_restocked_quantity as i32,
            avg_quantity_per_restock,
            restock_frequency_days,
            total_restock_cost,
            avg_restock_cost,
            reception_rate,
            cancellation_rate,
            avg_delivery_delay_days: None,
        })
    }

    // ====================== 6. PRÉDICTIONS & ALERTES ======================

    pub async fn get_predictions_alerts_kpis(
        pool: &PgPool,
        product_id: i32,
        params: &KpiPeriodParams,
    ) -> Result<PredictionsAlertsKpis, sqlx::Error> {
        let product = sqlx::query("SELECT stock_quantity_pro FROM products_pro WHERE id_pro = $1")
            .bind(product_id)
            .fetch_one(pool)
            .await?;

        let current_stock: i32 = product.try_get("stock_quantity_pro")?;

        let sales_kpis = Self::get_sales_rotation_kpis(pool, product_id, params).await?;
        let sales_velocity = sales_kpis.sales_velocity_per_day.unwrap_or(0.0);

        let days_of_coverage = if sales_velocity > 0.0 {
            Some(current_stock as f64 / sales_velocity)
        } else {
            None
        };

        let estimated_stockout_date = if let Some(days) = days_of_coverage {
            Some(chrono::Utc::now() + chrono::Duration::days(days as i64))
        } else {
            None
        };

        let alert_status = if let Some(days) = days_of_coverage {
            if days < 7.0 {
                "imminent_stockout"
            } else if days > 90.0 {
                "overstock"
            } else {
                "normal"
            }
        } else {
            "normal"
        }.to_string();

        let optimal_reorder_point = if sales_velocity > 0.0 {
            Some((sales_velocity * 14.0) as i32)
        } else {
            None
        };

        let optimal_reorder_quantity = if sales_velocity > 0.0 {
            Some((sales_velocity * 30.0) as i32)
        } else {
            None
        };

        let ai_forecast = sqlx::query(
            "SELECT total_predicted_demand, avg_daily_demand, recommended_stock,
                    reorder_quantity, days_until_stockout, urgency, mape
             FROM demand_forecasts
             WHERE product_id = $1
             ORDER BY forecast_date DESC
             LIMIT 1"
        )
        .bind(product_id)
        .fetch_optional(pool)
        .await
        .unwrap_or(None);

        let (ai_forecast_available, ai_total_predicted_demand, ai_avg_daily_demand,
             ai_recommended_stock, ai_reorder_quantity, ai_days_until_stockout,
             ai_urgency, ai_forecast_accuracy_mape) = if let Some(row) = ai_forecast {
            use rust_decimal::prelude::ToPrimitive;
            (
                true,
                row.try_get::<rust_decimal::Decimal, _>("total_predicted_demand").ok().and_then(|v| v.to_f64()),
                row.try_get::<rust_decimal::Decimal, _>("avg_daily_demand").ok().and_then(|v| v.to_f64()),
                row.try_get::<Option<i32>, _>("recommended_stock").ok().flatten(),
                row.try_get::<Option<i32>, _>("reorder_quantity").ok().flatten(),
                row.try_get::<Option<i32>, _>("days_until_stockout").ok().flatten(),
                row.try_get::<Option<String>, _>("urgency").ok().flatten(),
                row.try_get::<Option<rust_decimal::Decimal>, _>("mape").ok().flatten().and_then(|v| v.to_f64()),
            )
        } else {
            (false, None, None, None, None, None, None, None)
        };

        Ok(PredictionsAlertsKpis {
            estimated_stockout_date,
            optimal_reorder_quantity,
            optimal_reorder_point,
            days_of_coverage,
            alert_status,
            ai_forecast_available,
            ai_total_predicted_demand,
            ai_avg_daily_demand,
            ai_recommended_stock,
            ai_reorder_quantity,
            ai_days_until_stockout,
            ai_urgency,
            ai_forecast_accuracy_mape,
        })
    }

    // ====================== 7. SCORING & CLASSIFICATION ======================

    pub async fn get_scoring_classification_kpis(
        pool: &PgPool,
        product_id: i32,
        params: &KpiPeriodParams,
    ) -> Result<ScoringClassificationKpis, sqlx::Error> {
        let sales_kpis = Self::get_sales_rotation_kpis(pool, product_id, params).await?;
        let profitability_kpis = Self::get_profitability_kpis(pool, product_id, params).await?;
        let stock_kpis = Self::get_stock_availability_kpis(pool, product_id, params).await?;

        let popularity_score = if sales_kpis.quantity_sold > 0 {
            (sales_kpis.quantity_sold as f64).min(100.0)
        } else {
            0.0
        };

        let profitability_score = if let Some(roi) = profitability_kpis.roi {
            roi.min(100.0).max(0.0)
        } else {
            0.0
        };

        let reliability_score = if stock_kpis.stockout_count == 0 {
            100.0
        } else {
            (100.0 - (stock_kpis.stockout_count as f64 * 10.0)).max(0.0)
        };

        let global_score = (popularity_score + profitability_score + reliability_score) / 3.0;

        let abc_classification = if global_score >= 70.0 {
            "A"
        } else if global_score >= 40.0 {
            "B"
        } else {
            "C"
        }.to_string();

        let performance_category = if sales_kpis.quantity_sold == 0 && sales_kpis.order_count == 0 {
            "new"
        } else if global_score >= 80.0 {
            "star"
        } else if global_score >= 60.0 {
            "rising"
        } else if global_score >= 30.0 {
            "declining"
        } else {
            "dead"
        }.to_string();

        let ai_classification = sqlx::query(
            "SELECT abc_class, xyz_class, combined_class, strategy, priority
             FROM product_classifications
             WHERE product_id = $1
             ORDER BY created_at DESC
             LIMIT 1"
        )
        .bind(product_id)
        .fetch_optional(pool)
        .await
        .unwrap_or(None);

        let (ai_classification_available, ai_abc_class, ai_xyz_class,
             ai_combined_class, ai_strategy, ai_priority) = if let Some(row) = ai_classification {
            (
                true,
                row.try_get::<Option<String>, _>("abc_class").ok().flatten(),
                row.try_get::<Option<String>, _>("xyz_class").ok().flatten(),
                row.try_get::<Option<String>, _>("combined_class").ok().flatten(),
                row.try_get::<Option<String>, _>("strategy").ok().flatten(),
                row.try_get::<Option<String>, _>("priority").ok().flatten(),
            )
        } else {
            (false, None, None, None, None, None)
        };

        let ai_cluster = sqlx::query(
            "SELECT cluster_id, cluster_name
             FROM product_clusters
             WHERE product_id = $1
             ORDER BY created_at DESC
             LIMIT 1"
        )
        .bind(product_id)
        .fetch_optional(pool)
        .await
        .unwrap_or(None);

        let (ai_cluster_name, ai_cluster_id) = if let Some(row) = ai_cluster {
            (
                row.try_get::<Option<String>, _>("cluster_name").ok().flatten(),
                row.try_get::<Option<i32>, _>("cluster_id").ok().flatten(),
            )
        } else {
            (None, None)
        };

        Ok(ScoringClassificationKpis {
            popularity_score,
            profitability_score,
            reliability_score,
            global_score,
            abc_classification,
            performance_category,
            ai_classification_available,
            ai_abc_class,
            ai_xyz_class,
            ai_combined_class,
            ai_strategy,
            ai_priority,
            ai_cluster_name,
            ai_cluster_id,
        })
    }

    // ====================== 8. COMPARAISONS RELATIVES ======================

    pub async fn get_comparative_kpis(
        pool: &PgPool,
        product_id: i32,
        params: &KpiPeriodParams,
    ) -> Result<ComparativeKpis, sqlx::Error> {
        let start_date = params.start_date.unwrap_or_else(|| {
            (chrono::Utc::now() - chrono::Duration::days(30)).date_naive()
        });
        let end_date = params.end_date.unwrap_or_else(|| chrono::Utc::now().date_naive());

        let product = sqlx::query(
            "SELECT category_pro, supplier_id_pro FROM products_pro WHERE id_pro = $1"
        )
        .bind(product_id)
        .fetch_one(pool)
        .await?;

        let category: String = product.try_get("category_pro")?;
        let supplier_id: i32 = product.try_get("supplier_id_pro")?;

        let product_revenue_row = sqlx::query(
            "SELECT COALESCE(SUM(lor.line_total_lor), 0) as revenue
             FROM line_order_lor lor
             JOIN order_ord o ON lor.order_id_lor = o.id_ord
             WHERE lor.product_id_lor = $1
               AND o.order_date_ord >= $2
               AND o.order_date_ord <= $3"
        )
        .bind(product_id)
        .bind(start_date)
        .bind(end_date)
        .fetch_one(pool)
        .await?;

        let product_revenue = product_revenue_row.try_get::<rust_decimal::Decimal, _>("revenue")?
            .to_f64()
            .unwrap_or(0.0);

        let category_revenue_row = sqlx::query(
            "SELECT COALESCE(SUM(lor.line_total_lor), 0) as total_revenue
             FROM line_order_lor lor
             JOIN order_ord o ON lor.order_id_lor = o.id_ord
             JOIN products_pro p ON lor.product_id_lor = p.id_pro
             WHERE p.category_pro = $1
               AND o.order_date_ord >= $2
               AND o.order_date_ord <= $3"
        )
        .bind(&category)
        .bind(start_date)
        .bind(end_date)
        .fetch_one(pool)
        .await?;

        let category_total_revenue = category_revenue_row.try_get::<rust_decimal::Decimal, _>("total_revenue")?
            .to_f64()
            .unwrap_or(0.0);

        let category_avg_revenue_row = sqlx::query(
            "SELECT COALESCE(AVG(product_revenues.revenue), 0) as avg_revenue
             FROM (
                 SELECT p.id_pro, COALESCE(SUM(lor.line_total_lor), 0) as revenue
                 FROM products_pro p
                 LEFT JOIN line_order_lor lor ON p.id_pro = lor.product_id_lor
                 LEFT JOIN order_ord o ON lor.order_id_lor = o.id_ord
                 WHERE p.category_pro = $1
                   AND p.id_pro != $2
                   AND (o.order_date_ord IS NULL OR (o.order_date_ord >= $3 AND o.order_date_ord <= $4))
                 GROUP BY p.id_pro
             ) as product_revenues"
        )
        .bind(&category)
        .bind(product_id)
        .bind(start_date)
        .bind(end_date)
        .fetch_one(pool)
        .await?;

        let category_avg_revenue = category_avg_revenue_row.try_get::<rust_decimal::Decimal, _>("avg_revenue")?
            .to_f64()
            .unwrap_or(0.0);

        let performance_vs_category_percent = if category_avg_revenue > 0.0 {
            Some(((product_revenue - category_avg_revenue) / category_avg_revenue) * 100.0)
        } else {
            None
        };

        let share_in_category_percent = if category_total_revenue > 0.0 {
            Some((product_revenue / category_total_revenue) * 100.0)
        } else {
            None
        };

        let rank_row = sqlx::query(
            "SELECT rank
             FROM (
                 SELECT
                     p.id_pro,
                     RANK() OVER (ORDER BY COALESCE(SUM(lor.line_total_lor), 0) DESC) as rank
                 FROM products_pro p
                 LEFT JOIN line_order_lor lor ON p.id_pro = lor.product_id_lor
                 LEFT JOIN order_ord o ON lor.order_id_lor = o.id_ord
                 WHERE p.category_pro = $1
                   AND (o.order_date_ord IS NULL OR (o.order_date_ord >= $2 AND o.order_date_ord <= $3))
                 GROUP BY p.id_pro
             ) as ranked_products
             WHERE id_pro = $4"
        )
        .bind(&category)
        .bind(start_date)
        .bind(end_date)
        .bind(product_id)
        .fetch_one(pool)
        .await?;

        let rank_in_category: i64 = rank_row.try_get("rank")?;

        let supplier_avg_revenue_row = sqlx::query(
            "SELECT COALESCE(AVG(product_revenues.revenue), 0) as avg_revenue
             FROM (
                 SELECT p.id_pro, COALESCE(SUM(lor.line_total_lor), 0) as revenue
                 FROM products_pro p
                 LEFT JOIN line_order_lor lor ON p.id_pro = lor.product_id_lor
                 LEFT JOIN order_ord o ON lor.order_id_lor = o.id_ord
                 WHERE p.supplier_id_pro = $1
                   AND p.id_pro != $2
                   AND (o.order_date_ord IS NULL OR (o.order_date_ord >= $3 AND o.order_date_ord <= $4))
                 GROUP BY p.id_pro
             ) as product_revenues"
        )
        .bind(supplier_id)
        .bind(product_id)
        .bind(start_date)
        .bind(end_date)
        .fetch_one(pool)
        .await?;

        let supplier_avg_revenue = supplier_avg_revenue_row.try_get::<rust_decimal::Decimal, _>("avg_revenue")?
            .to_f64()
            .unwrap_or(0.0);

        let performance_vs_supplier_percent = if supplier_avg_revenue > 0.0 {
            Some(((product_revenue - supplier_avg_revenue) / supplier_avg_revenue) * 100.0)
        } else {
            None
        };

        Ok(ComparativeKpis {
            performance_vs_category_percent,
            performance_vs_supplier_percent,
            rank_in_category: Some(rank_in_category as i32),
            share_in_category_percent,
        })
    }

    // ====================== 10. TOUS LES KPIS (AGRÉGÉ) ======================

    pub async fn get_all_product_kpis(
        pool: &PgPool,
        product_id: i32,
        params: &KpiPeriodParams,
    ) -> Result<AllProductKpis, sqlx::Error> {
        let pricing_margin = Self::get_pricing_margin_kpis(pool, product_id, params).await?;
        let stock_availability = Self::get_stock_availability_kpis(pool, product_id, params).await?;
        let sales_rotation = Self::get_sales_rotation_kpis(pool, product_id, params).await?;
        let profitability = Self::get_profitability_kpis(pool, product_id, params).await?;
        let restock = Self::get_restock_kpis(pool, product_id, params).await?;
        let predictions_alerts = Self::get_predictions_alerts_kpis(pool, product_id, params).await?;
        let scoring_classification = Self::get_scoring_classification_kpis(pool, product_id, params).await?;
        let comparative = Self::get_comparative_kpis(pool, product_id, params).await?;
        let price_evolution = Self::get_price_evolution(pool, product_id, params).await?;

        Ok(AllProductKpis {
            pricing_margin,
            stock_availability,
            sales_rotation,
            profitability,
            restock,
            predictions_alerts,
            scoring_classification,
            comparative,
            price_evolution,
        })
    }

    // ====================== 9. ÉVOLUTION DES PRIX (GRAPHIQUES) ======================

    pub async fn get_price_evolution(
        pool: &PgPool,
        product_id: i32,
        params: &KpiPeriodParams,
    ) -> Result<PriceEvolution, sqlx::Error> {
        let start_date = params.start_date.unwrap_or_else(|| {
            (chrono::Utc::now() - chrono::Duration::days(30)).date_naive()
        });
        let end_date = params.end_date.unwrap_or_else(|| chrono::Utc::now().date_naive());

        let buying_price_rows = sqlx::query(
            "SELECT prr.buying_price_prr, prr.created_at
             FROM productrestockprices_prr prr
             WHERE prr.product_ref_prr = $1
               AND DATE(prr.created_at) >= $2
               AND DATE(prr.created_at) <= $3
             ORDER BY prr.created_at ASC"
        )
        .bind(product_id)
        .bind(start_date)
        .bind(end_date)
        .fetch_all(pool)
        .await?;

        let buying_price_history: Vec<PricePoint> = buying_price_rows
            .iter()
            .map(|row| {
                let price = row.try_get::<rust_decimal::Decimal, _>("buying_price_prr")
                    .ok()
                    .and_then(|v| v.to_f64())
                    .unwrap_or(0.0);
                let date = row.try_get::<chrono::DateTime<chrono::Utc>, _>("created_at")
                    .unwrap_or_else(|_| chrono::Utc::now());

                PricePoint { date, price }
            })
            .collect();

        let selling_price_rows = sqlx::query(
            "SELECT prp.price_prp, prp.created_at
             FROM productprices_prp prp
             WHERE prp.product_ref_prp = $1
               AND DATE(prp.created_at) >= $2
               AND DATE(prp.created_at) <= $3
             ORDER BY prp.created_at ASC"
        )
        .bind(product_id)
        .bind(start_date)
        .bind(end_date)
        .fetch_all(pool)
        .await?;

        let selling_price_history: Vec<PricePoint> = selling_price_rows
            .iter()
            .map(|row| {
                let price = row.try_get::<rust_decimal::Decimal, _>("price_prp")
                    .ok()
                    .and_then(|v| v.to_f64())
                    .unwrap_or(0.0);
                let date = row.try_get::<chrono::DateTime<chrono::Utc>, _>("created_at")
                    .unwrap_or_else(|_| chrono::Utc::now());

                PricePoint { date, price }
            })
            .collect();

        let mut margin_history: Vec<MarginPoint> = Vec::new();

        for selling_point in &selling_price_history {
            let buying_price_opt = buying_price_history
                .iter()
                .filter(|bp| bp.date <= selling_point.date)
                .max_by_key(|bp| bp.date)
                .map(|bp| bp.price);

            if let Some(buying_price) = buying_price_opt {
                if buying_price > 0.0 {
                    let margin_amount = selling_point.price - buying_price;
                    let margin_rate = (margin_amount / buying_price) * 100.0;

                    margin_history.push(MarginPoint {
                        date: selling_point.date,
                        margin_amount: Some(margin_amount),
                        margin_rate: Some(margin_rate),
                    });
                } else {
                    margin_history.push(MarginPoint {
                        date: selling_point.date,
                        margin_amount: None,
                        margin_rate: None,
                    });
                }
            }
        }

        Ok(PriceEvolution {
            buying_price_history,
            selling_price_history,
            margin_history,
        })
    }
}
