use sqlx::{PgPool, Row};
use super::dto::*;

// ==================== Demand Forecasts ====================

pub async fn get_latest_forecasts(
    pool: &PgPool,
    limit: i32,
    offset: i32,
) -> Result<Vec<ForecastResponse>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id, product_id, product_name, forecast_date, forecast_days,
                total_predicted_demand, avg_daily_demand, current_stock,
                recommended_stock, reorder_quantity, days_until_stockout,
                urgency, mape
         FROM v_latest_forecasts
         ORDER BY
            CASE urgency
                WHEN 'URGENT' THEN 1
                WHEN 'HIGH' THEN 2
                WHEN 'MEDIUM' THEN 3
                WHEN 'LOW' THEN 4
                ELSE 5
            END
         LIMIT $1 OFFSET $2"
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    Ok(rows.iter().map(|row| ForecastResponse {
        id: row.get("id"),
        product_id: row.get("product_id"),
        product_name: row.get("product_name"),
        forecast_date: row.get("forecast_date"),
        forecast_days: row.get("forecast_days"),
        total_predicted_demand: row.get("total_predicted_demand"),
        avg_daily_demand: row.get("avg_daily_demand"),
        current_stock: row.get("current_stock"),
        recommended_stock: row.get("recommended_stock"),
        reorder_quantity: row.get("reorder_quantity"),
        days_until_stockout: row.get("days_until_stockout"),
        urgency: row.get("urgency"),
        mape: row.get("mape"),
        rmse: None,
        created_at: row.get("forecast_date"),
    }).collect())
}

pub async fn get_forecast_by_product(
    pool: &PgPool,
    product_id: i32,
) -> Result<Option<ForecastResponse>, sqlx::Error> {
    let row = sqlx::query(
        "SELECT df.id, df.product_id, p.name_pro as product_name,
                df.forecast_date, df.forecast_days, df.total_predicted_demand,
                df.avg_daily_demand, df.current_stock, df.recommended_stock,
                df.reorder_quantity, df.days_until_stockout, df.urgency,
                df.mape, df.rmse, df.created_at
         FROM demand_forecasts df
         JOIN products_pro p ON df.product_id = p.id_pro
         WHERE df.product_id = $1
         ORDER BY df.forecast_date DESC
         LIMIT 1"
    )
    .bind(product_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|row| ForecastResponse {
        id: row.get("id"),
        product_id: row.get("product_id"),
        product_name: row.get("product_name"),
        forecast_date: row.get("forecast_date"),
        forecast_days: row.get("forecast_days"),
        total_predicted_demand: row.get("total_predicted_demand"),
        avg_daily_demand: row.get("avg_daily_demand"),
        current_stock: row.get("current_stock"),
        recommended_stock: row.get("recommended_stock"),
        reorder_quantity: row.get("reorder_quantity"),
        days_until_stockout: row.get("days_until_stockout"),
        urgency: row.get("urgency"),
        mape: row.get("mape"),
        rmse: row.get("rmse"),
        created_at: row.get("created_at"),
    }))
}

// ==================== Classifications ====================

pub async fn get_latest_classifications(
    pool: &PgPool,
    limit: i32,
    offset: i32,
) -> Result<Vec<ClassificationResponse>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id, product_id, product_name, abc_class, xyz_class,
                combined_class, total_revenue, revenue_contribution_pct,
                strategy, priority, created_at
         FROM v_latest_classifications
         ORDER BY
            CASE priority
                WHEN 'CRITICAL' THEN 1
                WHEN 'HIGH' THEN 2
                WHEN 'MEDIUM' THEN 3
                WHEN 'LOW' THEN 4
                ELSE 5
            END
         LIMIT $1 OFFSET $2"
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    Ok(rows.iter().map(|row| ClassificationResponse {
        id: row.get("id"),
        product_id: row.get("product_id"),
        product_name: row.get("product_name"),
        abc_class: row.get("abc_class"),
        xyz_class: row.get("xyz_class"),
        combined_class: row.get("combined_class"),
        total_revenue: row.get("total_revenue"),
        revenue_contribution_pct: row.get("revenue_contribution_pct"),
        total_units_sold: None,
        coefficient_of_variation: None,
        strategy: row.get("strategy"),
        priority: row.get("priority"),
        created_at: row.get("created_at"),
    }).collect())
}

pub async fn get_classification_by_product(
    pool: &PgPool,
    product_id: i32,
) -> Result<Option<ClassificationResponse>, sqlx::Error> {
    let row = sqlx::query(
        "SELECT pc.id, pc.product_id, p.name_pro as product_name,
                pc.abc_class, pc.xyz_class, pc.combined_class,
                pc.total_revenue, pc.revenue_contribution_pct,
                pc.total_units_sold, pc.coefficient_of_variation,
                pc.strategy, pc.priority, pc.created_at
         FROM product_classifications pc
         JOIN products_pro p ON pc.product_id = p.id_pro
         WHERE pc.product_id = $1
         ORDER BY pc.created_at DESC
         LIMIT 1"
    )
    .bind(product_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|row| ClassificationResponse {
        id: row.get("id"),
        product_id: row.get("product_id"),
        product_name: row.get("product_name"),
        abc_class: row.get("abc_class"),
        xyz_class: row.get("xyz_class"),
        combined_class: row.get("combined_class"),
        total_revenue: row.get("total_revenue"),
        revenue_contribution_pct: row.get("revenue_contribution_pct"),
        total_units_sold: row.get("total_units_sold"),
        coefficient_of_variation: row.get("coefficient_of_variation"),
        strategy: row.get("strategy"),
        priority: row.get("priority"),
        created_at: row.get("created_at"),
    }))
}

// ==================== Clusters ====================

pub async fn get_latest_clusters(
    pool: &PgPool,
    limit: i32,
    offset: i32,
) -> Result<Vec<ClusterResponse>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id, product_id, product_name, cluster_id, cluster_name,
                revenue_score, variability_score, trend_score, created_at
         FROM v_latest_clusters
         ORDER BY cluster_id, revenue_score DESC
         LIMIT $1 OFFSET $2"
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    Ok(rows.iter().map(|row| ClusterResponse {
        id: row.get("id"),
        product_id: row.get("product_id"),
        product_name: row.get("product_name"),
        cluster_id: row.get("cluster_id"),
        cluster_name: row.get("cluster_name"),
        revenue_score: row.get("revenue_score"),
        variability_score: row.get("variability_score"),
        trend_score: row.get("trend_score"),
        seasonality_score: None,
        frequency_score: None,
        margin_score: None,
        distance_to_centroid: None,
        n_clusters: None,
        silhouette_score: None,
        created_at: row.get("created_at"),
    }).collect())
}

pub async fn get_cluster_by_product(
    pool: &PgPool,
    product_id: i32,
) -> Result<Option<ClusterResponse>, sqlx::Error> {
    let row = sqlx::query(
        "SELECT pcl.id, pcl.product_id, p.name_pro as product_name,
                pcl.cluster_id, pcl.cluster_name, pcl.revenue_score,
                pcl.variability_score, pcl.trend_score, pcl.seasonality_score,
                pcl.frequency_score, pcl.margin_score, pcl.distance_to_centroid,
                pcl.n_clusters, pcl.silhouette_score, pcl.created_at
         FROM product_clusters pcl
         JOIN products_pro p ON pcl.product_id = p.id_pro
         WHERE pcl.product_id = $1
         ORDER BY pcl.created_at DESC
         LIMIT 1"
    )
    .bind(product_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|row| ClusterResponse {
        id: row.get("id"),
        product_id: row.get("product_id"),
        product_name: row.get("product_name"),
        cluster_id: row.get("cluster_id"),
        cluster_name: row.get("cluster_name"),
        revenue_score: row.get("revenue_score"),
        variability_score: row.get("variability_score"),
        trend_score: row.get("trend_score"),
        seasonality_score: row.get("seasonality_score"),
        frequency_score: row.get("frequency_score"),
        margin_score: row.get("margin_score"),
        distance_to_centroid: row.get("distance_to_centroid"),
        n_clusters: row.get("n_clusters"),
        silhouette_score: row.get("silhouette_score"),
        created_at: row.get("created_at"),
    }))
}

// ==================== Supplier Scores ====================

pub async fn get_latest_supplier_scores(
    pool: &PgPool,
    limit: i32,
    offset: i32,
) -> Result<Vec<SupplierScoreResponse>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id, supplier_id, supplier_name, overall_score,
                delivery_score, quality_score, lead_time_score,
                fulfillment_score, rating, created_at
         FROM v_latest_supplier_scores
         ORDER BY overall_score DESC
         LIMIT $1 OFFSET $2"
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    Ok(rows.iter().map(|row| SupplierScoreResponse {
        id: row.get("id"),
        supplier_id: row.get("supplier_id"),
        supplier_name: row.get("supplier_name"),
        overall_score: row.get("overall_score"),
        delivery_score: row.get("delivery_score"),
        quality_score: row.get("quality_score"),
        lead_time_score: row.get("lead_time_score"),
        fulfillment_score: row.get("fulfillment_score"),
        rating: row.get("rating"),
        total_restocks: None,
        created_at: row.get("created_at"),
    }).collect())
}

pub async fn get_supplier_score(
    pool: &PgPool,
    supplier_id: i32,
) -> Result<Option<SupplierScoreResponse>, sqlx::Error> {
    let row = sqlx::query(
        "SELECT ss.id, ss.supplier_id, s.name_sup as supplier_name,
                ss.overall_score, ss.delivery_score, ss.quality_score,
                ss.lead_time_score, ss.fulfillment_score, ss.rating,
                ss.total_restocks, ss.created_at
         FROM supplier_scores ss
         JOIN supplier_sup s ON ss.supplier_id = s.id_sup
         WHERE ss.supplier_id = $1
         ORDER BY ss.created_at DESC
         LIMIT 1"
    )
    .bind(supplier_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|row| SupplierScoreResponse {
        id: row.get("id"),
        supplier_id: row.get("supplier_id"),
        supplier_name: row.get("supplier_name"),
        overall_score: row.get("overall_score"),
        delivery_score: row.get("delivery_score"),
        quality_score: row.get("quality_score"),
        lead_time_score: row.get("lead_time_score"),
        fulfillment_score: row.get("fulfillment_score"),
        rating: row.get("rating"),
        total_restocks: row.get("total_restocks"),
        created_at: row.get("created_at"),
    }))
}

// ==================== Price Suggestions ====================

pub async fn get_price_suggestions(
    pool: &PgPool,
    limit: i32,
) -> Result<Vec<PriceSuggestionResponse>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT ps.id, ps.product_id, p.name_pro as product_name,
                ps.suggested_price, ps.current_price, ps.reason,
                ps.confidence, ps.created_at
         FROM price_suggestions ps
         JOIN products_pro p ON ps.product_id = p.id_pro
         ORDER BY ps.confidence DESC NULLS LAST, ps.created_at DESC
         LIMIT $1"
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows.iter().map(|row| {
        let confidence: Option<f64> = row.get::<Option<f64>, _>("confidence");
        PriceSuggestionResponse {
            id: row.get("id"),
            product_id: row.get("product_id"),
            product_name: row.get("product_name"),
            suggested_price: row.get("suggested_price"),
            current_price: row.get("current_price"),
            reason: row.get("reason"),
            confidence,
            created_at: row.get("created_at"),
        }
    }).collect())
}

// ==================== Price Anomalies ====================

pub async fn get_price_anomalies(
    pool: &PgPool,
    limit: i32,
    only_anomalies: bool,
) -> Result<Vec<PriceAnomalyResponse>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT pa.id, pa.product_id, p.name_pro as product_name,
                pa.current_price, pa.expected_price, pa.anomaly_score,
                pa.is_anomaly, pa.created_at
         FROM price_anomalies pa
         JOIN products_pro p ON pa.product_id = p.id_pro
         WHERE ($1::bool = false OR pa.is_anomaly = true)
         ORDER BY pa.created_at DESC
         LIMIT $2"
    )
    .bind(only_anomalies)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows.iter().map(|row| {
        let anomaly_score: Option<f64> = row.get::<Option<f64>, _>("anomaly_score");
        PriceAnomalyResponse {
            id: row.get("id"),
            product_id: row.get("product_id"),
            product_name: row.get("product_name"),
            current_price: row.get("current_price"),
            expected_price: row.get("expected_price"),
            anomaly_score,
            is_anomaly: row.get("is_anomaly"),
            created_at: row.get("created_at"),
        }
    }).collect())
}

// ==================== Sales Anomalies ====================

pub async fn get_sales_anomalies(
    pool: &PgPool,
    limit: i32,
    only_anomalies: bool,
) -> Result<Vec<SalesAnomalyResponse>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT sa.id, sa.product_id, p.name_pro as product_name,
                sa.sales_volume, sa.expected_sales, sa.anomaly_score,
                sa.is_anomaly, sa.created_at
         FROM sales_anomalies sa
         JOIN products_pro p ON sa.product_id = p.id_pro
         WHERE ($1::bool = false OR sa.is_anomaly = true)
         ORDER BY sa.created_at DESC
         LIMIT $2"
    )
    .bind(only_anomalies)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows.iter().map(|row| {
        let expected_sales: Option<f64> = row.get::<Option<f64>, _>("expected_sales");
        let anomaly_score: Option<f64> = row.get::<Option<f64>, _>("anomaly_score");
        SalesAnomalyResponse {
            id: row.get("id"),
            product_id: row.get("product_id"),
            product_name: row.get("product_name"),
            sales_volume: row.get("sales_volume"),
            expected_sales,
            anomaly_score,
            is_anomaly: row.get("is_anomaly"),
            created_at: row.get("created_at"),
        }
    }).collect())
}

// ==================== Urgent Restocks ====================

pub async fn get_urgent_restocks(
    pool: &PgPool,
    limit: i32,
) -> Result<Vec<UrgentRestockResponse>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT product_id, product_name, current_stock, recommended_stock,
                reorder_quantity, days_until_stockout, urgency,
                avg_daily_demand, forecast_date
         FROM v_urgent_restocks
         LIMIT $1"
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows.iter().map(|row| UrgentRestockResponse {
        product_id: row.get("product_id"),
        product_name: row.get("product_name"),
        current_stock: row.get("current_stock"),
        recommended_stock: row.get("recommended_stock"),
        reorder_quantity: row.get("reorder_quantity"),
        days_until_stockout: row.get("days_until_stockout"),
        urgency: row.get("urgency"),
        avg_daily_demand: row.get("avg_daily_demand"),
        forecast_date: row.get("forecast_date"),
    }).collect())
}
