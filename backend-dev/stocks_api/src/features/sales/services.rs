use chrono::{Duration, NaiveDate};
use sqlx::PgPool;
use sqlx::Row;


/// Filtre commun pour exclure les commandes annulées.
const STATUS_EXCLUDE: &str = "cancelled";

pub async fn get_total_revenue(
    pool: &PgPool,
    start: NaiveDate,
    end: NaiveDate,
) -> Result<f64, sqlx::Error> {
    let total: Option<f64> = sqlx::query_scalar(
        r#"
        SELECT COALESCE(SUM(amount_ord)::float8, 0) AS total
        FROM order_ord
        WHERE order_date_ord::date BETWEEN $1 AND $2
          AND (status_ord IS NULL OR status_ord <> $3)
        "#
    )
    .bind(start)
    .bind(end)
    .bind(STATUS_EXCLUDE)
    .fetch_one(pool)
    .await?;
    print!("Total revenue from {} to {}: {:?}", start, end, total);

    Ok(total.unwrap_or(0.0))
}

pub async fn get_revenue_evolution(
    pool: &PgPool,
    start: NaiveDate,
    end: NaiveDate,
) -> Result<f64, sqlx::Error> {
    let days = (end - start).num_days() + 1;
    let prev_end = start - Duration::days(1);
    let prev_start = prev_end - Duration::days(days - 1);

    let current = get_total_revenue(pool, start, end).await?;
    let previous = get_total_revenue(pool, prev_start, prev_end).await?;

    // Évolution % = (current - previous) / previous * 100
    let pct = if previous.abs() < f64::EPSILON {
        if current.abs() < f64::EPSILON { 0.0 } else { 100.0 }
    } else {
        ((current - previous) / previous) * 100.0
    };
    Ok(pct)
}

/// Structure pour représenter un point de données d'évolution
pub struct EvolutionPoint {
    pub date: String,
    pub revenue: f64,
}

/// Calcule l'évolution du chiffre d'affaires avec un grain de temps spécifique
pub async fn get_revenue_evolution_by_grain(
    pool: &PgPool,
    start: NaiveDate,
    end: NaiveDate,
    grain: &str,
) -> Result<Vec<EvolutionPoint>, sqlx::Error> {
    let (date_trunc, format_str) = match grain {
        "week" => ("week", "YYYY-MM-DD"),
        "month" => ("month", "YYYY-MM-01"),
        _ => ("day", "YYYY-MM-DD"), // par défaut: jour
    };

    let query = format!(
        r#"
        SELECT
            TO_CHAR(DATE_TRUNC('{}', order_date_ord), '{}') AS period,
            COALESCE(SUM(amount_ord)::float8, 0) AS revenue
        FROM order_ord
        WHERE order_date_ord::date BETWEEN $1 AND $2
          AND (status_ord IS NULL OR status_ord <> $3)
        GROUP BY period
        ORDER BY period
        "#,
        date_trunc, format_str
    );

    let rows = sqlx::query(&query)
        .bind(start)
        .bind(end)
        .bind(STATUS_EXCLUDE)
        .fetch_all(pool)
        .await?;

    let mut results = Vec::new();
    for row in rows {
        let date: String = row.try_get("period")?;
        let revenue: f64 = row.try_get("revenue")?;
        results.push(EvolutionPoint { date, revenue });
    }

    Ok(results)
}

pub async fn get_forecast_vs_actual(
    pool: &PgPool,
    start: NaiveDate,
    end: NaiveDate,
) -> Result<(f64, f64), sqlx::Error> {
    let days = (end - start).num_days() + 1;
    let prev_end = start - Duration::days(1);
    let prev_start = prev_end - Duration::days(days - 1);

    let actual = get_total_revenue(pool, start, end).await?;
    let forecast = get_total_revenue(pool, prev_start, prev_end).await?;

    Ok((forecast, actual))
}

pub async fn get_average_basket(
    pool: &PgPool,
    start: NaiveDate,
    end: NaiveDate,
) -> Result<(f64, f64), sqlx::Error> {
    let avg: Option<f64> = sqlx::query_scalar(
        r#"
        SELECT COALESCE(AVG(amount_ord)::float8, 0) AS avg_basket
        FROM order_ord
        WHERE order_date_ord::date BETWEEN $1 AND $2
          AND (status_ord IS NULL OR status_ord <> $3)
        "#
    )
    .bind(start)
    .bind(end)
    .bind(STATUS_EXCLUDE)
    .fetch_one(pool)
    .await?;

    let avg = avg.unwrap_or(0.0);

    // évolution vs période précédente
    let days = (end - start).num_days() + 1;
    let prev_end = start - Duration::days(1);
    let prev_start = prev_end - Duration::days(days - 1);

    let prev_avg: Option<f64> = sqlx::query_scalar(
        r#"
        SELECT COALESCE(AVG(amount_ord)::float8, 0) AS avg_basket
        FROM order_ord
        WHERE order_date_ord::date BETWEEN $1 AND $2
          AND (status_ord IS NULL OR status_ord <> $3)
        "#
    )
    .bind(prev_start)
    .bind(prev_end)
    .bind(STATUS_EXCLUDE)
    .fetch_one(pool)
    .await?;

    let prev_avg = prev_avg.unwrap_or(0.0);

    let evo = if prev_avg.abs() < f64::EPSILON {
        if avg.abs() < f64::EPSILON { 0.0 } else { 100.0 }
    } else {
        ((avg - prev_avg) / prev_avg) * 100.0
    };

    Ok((avg, evo))
}

pub async fn get_average_basket_by_client_type(
    pool: &PgPool,
    start: NaiveDate,
    end: NaiveDate,
) -> Result<(f64, f64), sqlx::Error> {
    // Moyenne des paniers des "nouveaux" clients (première commande dans la période)
    let new_clients_avg: Option<f64> = sqlx::query_scalar(
        r#"
        WITH first_orders AS (
            SELECT user_id_ord, MIN(order_date_ord::date) AS first_date
            FROM order_ord
            WHERE (status_ord IS NULL OR status_ord <> $3)
            GROUP BY user_id_ord
        )
        SELECT COALESCE(AVG(o.amount_ord)::float8, 0) AS avg_basket
        FROM order_ord o
        JOIN first_orders f ON f.user_id_ord = o.user_id_ord
        WHERE o.order_date_ord::date BETWEEN $1 AND $2
          AND f.first_date BETWEEN $1 AND $2
          AND (o.status_ord IS NULL OR o.status_ord <> $3)
        "#
    )
    .bind(start)
    .bind(end)
    .bind(STATUS_EXCLUDE)
    .fetch_one(pool)
    .await?;

    let new_clients_avg = new_clients_avg.unwrap_or(0.0);

    // Moyenne des paniers des "fidèles" (première commande avant la période)
    let loyal_clients_avg: Option<f64> = sqlx::query_scalar(
        r#"
        WITH first_orders AS (
            SELECT user_id_ord, MIN(order_date_ord::date) AS first_date
            FROM order_ord
            WHERE (status_ord IS NULL OR status_ord <> $3)
            GROUP BY user_id_ord
        )
        SELECT COALESCE(AVG(o.amount_ord)::float8, 0) AS avg_basket
        FROM order_ord o
        JOIN first_orders f ON f.user_id_ord = o.user_id_ord
        WHERE o.order_date_ord::date BETWEEN $1 AND $2
          AND f.first_date < $1
          AND (o.status_ord IS NULL OR o.status_ord <> $3)
        "#
    )
    .bind(start)
    .bind(end)
    .bind(STATUS_EXCLUDE)
    .fetch_one(pool)
    .await?;

    let loyal_clients_avg = loyal_clients_avg.unwrap_or(0.0);

    Ok((new_clients_avg, loyal_clients_avg))
}
