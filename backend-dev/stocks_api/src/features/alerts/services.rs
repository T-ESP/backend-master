use sqlx::{PgPool, Row};
use super::dto::*;

const VALID_STATUSES: &[&str] = &["new", "acknowledged", "in_progress", "resolved", "dismissed"];

pub fn is_valid_status(status: &str) -> bool {
    VALID_STATUSES.contains(&status)
}

pub async fn get_alerts(
    pool: &PgPool,
    filters: &AlertFilters,
) -> Result<Vec<AlertResponse>, sqlx::Error> {
    let limit = filters.limit.unwrap_or(50);
    let offset = filters.offset.unwrap_or(0);

    let rows = sqlx::query(
        "SELECT n.id, n.product_id, p.name_pro as product_name,
                n.model_type, n.category::text, n.notification_type,
                n.severity, n.message, n.action_recommended,
                n.related_result_id, n.status::text, n.created_at, n.updated_at
         FROM notifications n
         LEFT JOIN products_pro p ON n.product_id = p.id_pro
         WHERE ($1::text IS NULL OR n.status::text = $1)
           AND ($2::text IS NULL OR n.severity = $2)
           AND ($3::text IS NULL OR n.model_type = $3)
           AND ($4::int IS NULL OR n.product_id = $4)
           AND ($5::text IS NULL OR n.category::text = $5)
           AND ($6::date IS NULL OR n.created_at >= $6::date)
           AND ($7::date IS NULL OR n.created_at <= ($7::date + interval '1 day'))
         ORDER BY
            CASE n.severity
                WHEN 'CRITICAL' THEN 1
                WHEN 'HIGH' THEN 2
                WHEN 'MEDIUM' THEN 3
                WHEN 'LOW' THEN 4
                ELSE 5
            END,
            n.created_at DESC
         LIMIT $8 OFFSET $9"
    )
    .bind(filters.status.as_deref())
    .bind(filters.severity.as_deref())
    .bind(filters.model_type.as_deref())
    .bind(filters.product_id)
    .bind(filters.category.as_deref())
    .bind(filters.from_date)
    .bind(filters.to_date)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    let alerts = rows
        .iter()
        .map(|row| AlertResponse {
            id: row.get("id"),
            product_id: row.get("product_id"),
            product_name: row.get("product_name"),
            model_type: row.get("model_type"),
            category: row.get("category"),
            notification_type: row.get("notification_type"),
            severity: row.get("severity"),
            message: row.get("message"),
            action_recommended: row.get("action_recommended"),
            related_result_id: row.get("related_result_id"),
            status: row.get("status"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
        .collect();

    Ok(alerts)
}

pub async fn get_alert_by_id(
    pool: &PgPool,
    id: i32,
) -> Result<Option<AlertResponse>, sqlx::Error> {
    let row = sqlx::query(
        "SELECT n.id, n.product_id, p.name_pro as product_name,
                n.model_type, n.category::text, n.notification_type,
                n.severity, n.message, n.action_recommended,
                n.related_result_id, n.status::text, n.created_at, n.updated_at
         FROM notifications n
         LEFT JOIN products_pro p ON n.product_id = p.id_pro
         WHERE n.id = $1"
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|row| AlertResponse {
        id: row.get("id"),
        product_id: row.get("product_id"),
        product_name: row.get("product_name"),
        model_type: row.get("model_type"),
        category: row.get("category"),
        notification_type: row.get("notification_type"),
        severity: row.get("severity"),
        message: row.get("message"),
        action_recommended: row.get("action_recommended"),
        related_result_id: row.get("related_result_id"),
        status: row.get("status"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }))
}

pub async fn update_alert_status(
    pool: &PgPool,
    id: i32,
    status: &str,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE notifications SET status = $1::notification_status_enum, updated_at = NOW() WHERE id = $2"
    )
    .bind(status)
    .bind(id)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

pub async fn bulk_update_status(
    pool: &PgPool,
    ids: &[i32],
    status: &str,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE notifications SET status = $1::notification_status_enum, updated_at = NOW() WHERE id = ANY($2)"
    )
    .bind(status)
    .bind(ids)
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}

pub async fn get_alert_summary(pool: &PgPool) -> Result<AlertSummary, sqlx::Error> {
    let total_row = sqlx::query("SELECT COUNT(*) as total FROM notifications")
        .fetch_one(pool)
        .await?;
    let total: i64 = total_row.get("total");

    let status_rows = sqlx::query(
        "SELECT status::text, COUNT(*) as count FROM notifications GROUP BY status"
    )
    .fetch_all(pool)
    .await?;

    let mut by_status = StatusCounts {
        new: 0,
        acknowledged: 0,
        in_progress: 0,
        resolved: 0,
        dismissed: 0,
    };

    for row in &status_rows {
        let status: String = row.get("status");
        let count: i64 = row.get("count");
        match status.as_str() {
            "new" => by_status.new = count,
            "acknowledged" => by_status.acknowledged = count,
            "in_progress" => by_status.in_progress = count,
            "resolved" => by_status.resolved = count,
            "dismissed" => by_status.dismissed = count,
            _ => {}
        }
    }

    let severity_rows = sqlx::query(
        "SELECT severity, COUNT(*) as count FROM notifications GROUP BY severity"
    )
    .fetch_all(pool)
    .await?;

    let mut by_severity = SeverityCounts {
        critical: 0,
        high: 0,
        medium: 0,
        low: 0,
    };

    for row in &severity_rows {
        let severity: String = row.get("severity");
        let count: i64 = row.get("count");
        match severity.to_uppercase().as_str() {
            "CRITICAL" => by_severity.critical = count,
            "HIGH" => by_severity.high = count,
            "MEDIUM" => by_severity.medium = count,
            "LOW" => by_severity.low = count,
            _ => {}
        }
    }

    let model_type_rows = sqlx::query(
        "SELECT model_type, COUNT(*) as count FROM notifications GROUP BY model_type ORDER BY count DESC"
    )
    .fetch_all(pool)
    .await?;

    let by_model_type = model_type_rows
        .iter()
        .map(|row| ModelTypeCounts {
            model_type: row.get("model_type"),
            count: row.get("count"),
        })
        .collect();

    Ok(AlertSummary {
        total,
        by_status,
        by_severity,
        by_model_type,
    })
}

pub async fn cleanup_old_alerts(
    pool: &PgPool,
    older_than_days: i32,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "DELETE FROM notifications
         WHERE status::text IN ('resolved', 'dismissed')
           AND created_at < NOW() - ($1 || ' days')::interval"
    )
    .bind(older_than_days.to_string())
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}
