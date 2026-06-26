"""
SQL queries for report data extraction.

All functions receive a psycopg2 connection and period boundaries.
They return plain dicts/lists — no formatting, no rendering logic here.
"""

from datetime import datetime
from typing import Any


def get_sales_summary(conn, period_start: datetime, period_end: datetime) -> dict[str, Any]:
    with conn.cursor() as cur:
        cur.execute("""
            SELECT
                COUNT(*)                                        AS total_orders,
                COALESCE(SUM(amount_ord), 0)                   AS total_revenue,
                COALESCE(AVG(amount_ord), 0)                   AS avg_order_value,
                COUNT(*) FILTER (WHERE status_ord = 'completed')  AS completed,
                COUNT(*) FILTER (WHERE status_ord = 'pending')    AS pending,
                COUNT(*) FILTER (WHERE status_ord = 'cancelled')  AS cancelled
            FROM order_ord
            WHERE order_date_ord >= %s AND order_date_ord < %s
        """, (period_start, period_end))
        row = cur.fetchone()
        summary = {
            "total_orders":    int(row[0]),
            "total_revenue":   float(row[1]),
            "avg_order_value": float(row[2]),
            "completed":       int(row[3]),
            "pending":         int(row[4]),
            "cancelled":       int(row[5]),
        }

        cur.execute("""
            SELECT
                p.name_pro,
                p.category_pro,
                COUNT(DISTINCT lo.order_id_lor)  AS order_count,
                SUM(lo.quantity_lor)             AS units_sold,
                SUM(lo.line_total_lor)           AS revenue
            FROM line_order_lor lo
            JOIN products_pro p ON lo.product_id_lor = p.id_pro
            JOIN order_ord    o ON lo.order_id_lor   = o.id_ord
            WHERE o.order_date_ord >= %s AND o.order_date_ord < %s
            GROUP BY p.id_pro, p.name_pro, p.category_pro
            ORDER BY revenue DESC
            LIMIT 5
        """, (period_start, period_end))
        summary["top_products"] = [
            {
                "name":        r[0],
                "category":    r[1],
                "order_count": int(r[2]),
                "units_sold":  int(r[3]),
                "revenue":     float(r[4]),
            }
            for r in cur.fetchall()
        ]

    return summary


def get_stock_alerts(conn) -> dict[str, Any]:
    with conn.cursor() as cur:
        cur.execute(
            "SELECT COUNT(*) FROM products_pro WHERE status_pro = 'out_of_stock'"
        )
        out_of_stock = int(cur.fetchone()[0])

        cur.execute("""
            SELECT COUNT(*) FROM products_pro
            WHERE stock_quantity_pro > 0
              AND stock_quantity_pro < 10
              AND status_pro != 'discontinued'
        """)
        low_stock = int(cur.fetchone()[0])

        cur.execute("""
            SELECT product_name, current_stock, days_until_stockout, urgency, reorder_quantity
            FROM v_urgent_restocks
            LIMIT 10
        """)
        urgent_restocks = [
            {
                "product":          r[0],
                "current_stock":    int(r[1]) if r[1] is not None else 0,
                "days_to_stockout": int(r[2]) if r[2] is not None else 0,
                "urgency":          r[3],
                "reorder_qty":      int(r[4]) if r[4] is not None else 0,
            }
            for r in cur.fetchall()
        ]

    return {
        "out_of_stock":   out_of_stock,
        "low_stock":      low_stock,
        "urgent_restocks": urgent_restocks,
    }


def get_ai_insights(conn, period_start: datetime, period_end: datetime) -> dict[str, Any]:
    with conn.cursor() as cur:
        cur.execute("""
            SELECT
                COUNT(*) FILTER (WHERE is_anomaly = TRUE) AS anomaly_count,
                COUNT(*)                                   AS total_checked
            FROM price_anomalies
            WHERE created_at >= %s AND created_at < %s
        """, (period_start, period_end))
        r = cur.fetchone()
        price_anomalies = {"detected": int(r[0]), "checked": int(r[1])}

        cur.execute("""
            SELECT
                COUNT(*) FILTER (WHERE is_anomaly = TRUE) AS anomaly_count,
                COUNT(*)                                   AS total_checked
            FROM sales_anomalies
            WHERE created_at >= %s AND created_at < %s
        """, (period_start, period_end))
        r = cur.fetchone()
        sales_anomalies = {"detected": int(r[0]), "checked": int(r[1])}

        cur.execute("""
            SELECT abc_class, COUNT(*) AS cnt
            FROM v_latest_classifications
            GROUP BY abc_class
            ORDER BY abc_class
        """)
        abc_distribution = {row[0]: int(row[1]) for row in cur.fetchall()}

        cur.execute("""
            SELECT p.name_pro, ps.current_price, ps.suggested_price, ps.confidence, ps.reason
            FROM price_suggestions ps
            JOIN products_pro p ON ps.product_id = p.id_pro
            WHERE ps.created_at >= %s AND ps.created_at < %s
              AND ps.confidence IS NOT NULL
            ORDER BY ps.confidence DESC
            LIMIT 5
        """, (period_start, period_end))
        top_suggestions = [
            {
                "product":         r[0],
                "current_price":   float(r[1]),
                "suggested_price": float(r[2]),
                "confidence":      float(r[3]) if r[3] else 0.0,
                "reason":          r[4] or "",
            }
            for r in cur.fetchall()
        ]

    return {
        "price_anomalies": price_anomalies,
        "sales_anomalies": sales_anomalies,
        "abc_distribution": abc_distribution,
        "top_price_suggestions": top_suggestions,
    }


def get_notifications_summary(conn, period_start: datetime, period_end: datetime) -> dict[str, Any]:
    with conn.cursor() as cur:
        cur.execute("""
            SELECT
                COUNT(*)                                               AS total_new,
                COUNT(*) FILTER (WHERE severity = 'HIGH')             AS high,
                COUNT(*) FILTER (WHERE severity = 'CRITICAL')         AS critical,
                COUNT(*) FILTER (WHERE category = 'alert')            AS alerts,
                COUNT(*) FILTER (WHERE category = 'suggestion')       AS suggestions
            FROM notifications
            WHERE created_at >= %s AND created_at < %s
              AND status IN ('new', 'acknowledged')
        """, (period_start, period_end))
        r = cur.fetchone()
        new_counts = {
            "total":       int(r[0]),
            "high":        int(r[1]),
            "critical":    int(r[2]),
            "alerts":      int(r[3]),
            "suggestions": int(r[4]),
        }

        cur.execute("""
            SELECT COUNT(*) FROM notifications
            WHERE status IN ('resolved', 'dismissed')
              AND updated_at >= %s AND updated_at < %s
        """, (period_start, period_end))
        resolved = int(cur.fetchone()[0])

        cur.execute(
            "SELECT COUNT(*) FROM notifications WHERE status = 'new'"
        )
        total_unacknowledged = int(cur.fetchone()[0])

    return {
        "new_in_period":       new_counts,
        "resolved_in_period":  resolved,
        "total_unacknowledged": total_unacknowledged,
    }


def get_platform_tenant_list(master_conn) -> list[dict[str, Any]]:
    with master_conn.cursor() as cur:
        cur.execute("""
            SELECT slug, name, email, status, created_at
            FROM commerces
            ORDER BY name
        """)
        return [
            {
                "slug":       r[0],
                "name":       r[1],
                "email":      r[2],
                "status":     r[3],
                "created_at": r[4].isoformat() if r[4] else None,
            }
            for r in cur.fetchall()
        ]


def get_tenant_quick_summary(conn, period_start: datetime, period_end: datetime) -> dict[str, Any]:
    """Lightweight per-tenant summary for the platform admin report."""
    with conn.cursor() as cur:
        cur.execute("""
            SELECT
                COUNT(*)                          AS orders,
                COALESCE(SUM(amount_ord), 0)      AS revenue
            FROM order_ord
            WHERE order_date_ord >= %s AND order_date_ord < %s
        """, (period_start, period_end))
        r = cur.fetchone()
        orders, revenue = int(r[0]), float(r[1])

        cur.execute(
            "SELECT COUNT(*) FROM notifications WHERE status = 'new'"
        )
        open_alerts = int(cur.fetchone()[0])

        cur.execute(
            "SELECT COUNT(*) FROM products_pro WHERE status_pro = 'out_of_stock'"
        )
        out_of_stock = int(cur.fetchone()[0])

    return {
        "orders":       orders,
        "revenue":      revenue,
        "open_alerts":  open_alerts,
        "out_of_stock": out_of_stock,
    }


def log_report(master_conn, *, report_type: str, scope: str, tenant_slug: str | None,
               period_start: datetime, period_end: datetime,
               emailed_to: str | None, emailed_at: datetime | None,
               status: str, error_message: str | None = None,
               file_size_bytes: int | None = None) -> int:
    with master_conn.cursor() as cur:
        cur.execute("""
            INSERT INTO reports_log
                (report_type, scope, tenant_slug, period_start, period_end,
                 emailed_to, emailed_at, status, error_message, file_size_bytes)
            VALUES (%s, %s, %s, %s, %s, %s, %s, %s, %s, %s)
            RETURNING id
        """, (report_type, scope, tenant_slug, period_start, period_end,
              emailed_to, emailed_at, status, error_message, file_size_bytes))
        master_conn.commit()
        return cur.fetchone()[0]


def get_report_history(master_conn, limit: int = 50, tenant_slug: str | None = None) -> list[dict]:
    with master_conn.cursor() as cur:
        if tenant_slug:
            cur.execute("""
                SELECT id, report_type, scope, tenant_slug, period_start, period_end,
                       generated_at, emailed_to, emailed_at, status, error_message, file_size_bytes
                FROM reports_log
                WHERE tenant_slug = %s
                ORDER BY generated_at DESC
                LIMIT %s
            """, (tenant_slug, limit))
        else:
            cur.execute("""
                SELECT id, report_type, scope, tenant_slug, period_start, period_end,
                       generated_at, emailed_to, emailed_at, status, error_message, file_size_bytes
                FROM reports_log
                ORDER BY generated_at DESC
                LIMIT %s
            """, (limit,))

        cols = ["id", "report_type", "scope", "tenant_slug", "period_start", "period_end",
                "generated_at", "emailed_to", "emailed_at", "status", "error_message", "file_size_bytes"]
        return [
            {k: (v.isoformat() if hasattr(v, "isoformat") else v) for k, v in zip(cols, row)}
            for row in cur.fetchall()
        ]
