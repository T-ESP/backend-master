"""
Report builder: pulls data from the DB and assembles a structured report dict.

Returns plain Python dicts — no rendering, no I/O side effects.
"""

import os
from datetime import datetime, timedelta
from calendar import monthrange
from typing import Any
from urllib.parse import urlparse, urlunparse

import psycopg2

from reporting.queries import (
    get_sales_summary,
    get_stock_alerts,
    get_ai_insights,
    get_notifications_summary,
    get_platform_tenant_list,
    get_tenant_quick_summary,
)
from utils.logger import get_logger

logger = get_logger("reporting.builder")


# ---------------------------------------------------------------------------
# Period helpers
# ---------------------------------------------------------------------------

def period_for(report_type: str, reference_dt: datetime | None = None) -> tuple[datetime, datetime]:
    """Return (period_start, period_end) for the given report type.

    - daily:   yesterday midnight → today midnight (exclusive)
    - weekly:  last Monday 00:00 → last Sunday 23:59:59
    - monthly: first day of last month 00:00 → last day of last month 23:59:59
    """
    now = reference_dt or datetime.utcnow()
    today = now.replace(hour=0, minute=0, second=0, microsecond=0)

    if report_type == "daily":
        start = today - timedelta(days=1)
        end   = today
        return start, end

    if report_type == "weekly":
        # Last complete Monday–Sunday week
        last_monday = today - timedelta(days=today.weekday() + 7)
        last_sunday = last_monday + timedelta(days=7)
        return last_monday, last_sunday

    if report_type == "monthly":
        first_of_this_month = today.replace(day=1)
        last_month_end      = first_of_this_month - timedelta(seconds=1)
        last_month_start    = last_month_end.replace(day=1, hour=0, minute=0, second=0, microsecond=0)
        last_month_last_day = monthrange(last_month_end.year, last_month_end.month)[1]
        end = last_month_start.replace(day=last_month_last_day) + timedelta(days=1)
        return last_month_start, end

    raise ValueError(f"Unknown report_type: {report_type!r}")


def _master_dsn() -> str:
    return os.environ["DATABASE_URL"]


def _tenant_dsn(db_name: str) -> str:
    parsed = urlparse(_master_dsn())
    return urlunparse(parsed._replace(path=f"/{db_name}"))


# ---------------------------------------------------------------------------
# Per-tenant report
# ---------------------------------------------------------------------------

def build_tenant_report(
    *,
    slug: str,
    name: str,
    email: str,
    db_name: str,
    report_type: str,
    period_start: datetime,
    period_end: datetime,
) -> dict[str, Any]:
    """Build a complete per-tenant report dict ready for rendering."""
    dsn = _tenant_dsn(db_name)
    conn = psycopg2.connect(dsn)
    try:
        sales        = get_sales_summary(conn, period_start, period_end)
        stock        = get_stock_alerts(conn)
        insights     = get_ai_insights(conn, period_start, period_end)
        notifs       = get_notifications_summary(conn, period_start, period_end)
    finally:
        conn.close()

    return {
        "scope":        "tenant",
        "report_type":  report_type,
        "commerce": {
            "slug":  slug,
            "name":  name,
            "email": email,
        },
        "period": {
            "start": period_start.strftime("%Y-%m-%d"),
            "end":   (period_end - timedelta(seconds=1)).strftime("%Y-%m-%d"),
            "label": _period_label(report_type, period_start),
        },
        "generated_at": datetime.utcnow().strftime("%Y-%m-%d %H:%M UTC"),
        "sales":        sales,
        "stock":        stock,
        "insights":     insights,
        "notifications": notifs,
    }


# ---------------------------------------------------------------------------
# Platform admin report
# ---------------------------------------------------------------------------

def build_platform_report(
    *,
    report_type: str,
    period_start: datetime,
    period_end: datetime,
) -> dict[str, Any]:
    """Build a platform-wide summary report dict."""
    master_conn = psycopg2.connect(_master_dsn())
    try:
        tenants = get_platform_tenant_list(master_conn)
    finally:
        master_conn.close()

    tenant_summaries = []
    total_revenue = 0.0
    total_orders  = 0

    for t in tenants:
        if t["status"] != "active":
            continue
        try:
            db_name  = f"tenant_{t['slug']}"
            dsn      = _tenant_dsn(db_name)
            conn     = psycopg2.connect(dsn)
            try:
                summary = get_tenant_quick_summary(conn, period_start, period_end)
            finally:
                conn.close()

            total_revenue += summary["revenue"]
            total_orders  += summary["orders"]
            tenant_summaries.append({**t, **summary})
        except Exception as e:
            logger.warning("Could not fetch summary for tenant %s: %s", t["slug"], e)
            tenant_summaries.append({**t, "orders": 0, "revenue": 0.0,
                                      "open_alerts": 0, "out_of_stock": 0, "error": str(e)})

    tenant_summaries.sort(key=lambda x: x.get("revenue", 0), reverse=True)

    return {
        "scope":       "platform",
        "report_type": report_type,
        "period": {
            "start": period_start.strftime("%Y-%m-%d"),
            "end":   (period_end - timedelta(seconds=1)).strftime("%Y-%m-%d"),
            "label": _period_label(report_type, period_start),
        },
        "generated_at": datetime.utcnow().strftime("%Y-%m-%d %H:%M UTC"),
        "platform": {
            "total_tenants":  len(tenants),
            "active_tenants": sum(1 for t in tenants if t["status"] == "active"),
            "total_revenue":  total_revenue,
            "total_orders":   total_orders,
        },
        "tenants": tenant_summaries,
    }


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _period_label(report_type: str, period_start: datetime) -> str:
    if report_type == "daily":
        return period_start.strftime("%B %d, %Y")
    if report_type == "weekly":
        week_end = period_start + timedelta(days=6)
        return f"{period_start.strftime('%b %d')} – {week_end.strftime('%b %d, %Y')}"
    if report_type == "monthly":
        return period_start.strftime("%B %Y")
    return period_start.strftime("%Y-%m-%d")
