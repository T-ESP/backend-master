"""
Scheduled report jobs — called by APScheduler in scheduler.py.

Three entry points:
    run_daily_reports()   — called every night (default 23:00)
    run_weekly_reports()  — called every Monday morning (default 08:00)
    run_monthly_reports() — called on the 1st of each month (default 09:00)

Each generates per-tenant PDFs + a platform summary, then emails them via SendGrid.
All results are logged to reports_log in the master DB.
"""

import os
import psycopg2
from datetime import datetime
from typing import Any

from reporting.builder import build_tenant_report, build_platform_report, period_for
from reporting.pdf_renderer import render_tenant_pdf, render_platform_pdf
from reporting.email_sender import send_report_email
from reporting.queries import log_report, get_platform_tenant_list
from utils.logger import get_logger

logger = get_logger("reporting.scheduler")

_PLATFORM_ADMIN_EMAIL = os.getenv("PLATFORM_ADMIN_EMAIL", "")


def _master_dsn() -> str:
    return os.environ["DATABASE_URL"]


def _filename(scope: str, slug_or_platform: str, report_type: str, period_start: datetime) -> str:
    date_str = period_start.strftime("%Y-%m-%d")
    label    = slug_or_platform.replace(" ", "_")
    return f"report_{scope}_{label}_{report_type}_{date_str}.pdf"


def _run_reports_for_type(report_type: str) -> dict[str, Any]:
    """Core logic shared by all three schedule entry points."""
    period_start, period_end = period_for(report_type)
    logger.info("=" * 60)
    logger.info("Starting %s reports (period %s → %s)", report_type, period_start.date(), period_end.date())
    logger.info("=" * 60)

    master_conn = psycopg2.connect(_master_dsn())
    try:
        tenants = get_platform_tenant_list(master_conn)
    finally:
        master_conn.close()

    results = {"tenant": {}, "platform": None}

    # ------------------------------------------------------------------ #
    # 1. Per-tenant reports
    # ------------------------------------------------------------------ #
    for t in tenants:
        if t["status"] != "active":
            continue

        slug    = t["slug"]
        db_name = f"tenant_{slug}"

        try:
            logger.info("[%s] Building tenant report...", slug)
            data = build_tenant_report(
                slug=slug,
                name=t["name"],
                email=t["email"],
                db_name=db_name,
                report_type=report_type,
                period_start=period_start,
                period_end=period_end,
            )

            pdf_bytes = render_tenant_pdf(data)
            fname     = _filename("tenant", slug, report_type, period_start)

            emailed_at = None
            emailed_to = None

            if t["email"] and os.getenv("SENDGRID_API_KEY"):
                send_report_email(
                    to_email=t["email"],
                    report_data=data,
                    pdf_bytes=pdf_bytes,
                    filename=fname,
                )
                emailed_at = datetime.utcnow()
                emailed_to = t["email"]
                logger.info("[%s] Emailed to %s", slug, t["email"])
            else:
                logger.warning("[%s] Skipping email (no address or no SENDGRID_API_KEY)", slug)

            master_conn = psycopg2.connect(_master_dsn())
            try:
                log_report(
                    master_conn,
                    report_type=report_type,
                    scope="tenant",
                    tenant_slug=slug,
                    period_start=period_start,
                    period_end=period_end,
                    emailed_to=emailed_to,
                    emailed_at=emailed_at,
                    status="success",
                    file_size_bytes=len(pdf_bytes),
                )
            finally:
                master_conn.close()

            results["tenant"][slug] = "success"

        except Exception as e:
            logger.error("[%s] Report failed: %s", slug, e, exc_info=True)
            master_conn = psycopg2.connect(_master_dsn())
            try:
                log_report(
                    master_conn,
                    report_type=report_type,
                    scope="tenant",
                    tenant_slug=slug,
                    period_start=period_start,
                    period_end=period_end,
                    emailed_to=None,
                    emailed_at=None,
                    status="failed",
                    error_message=str(e),
                )
            finally:
                master_conn.close()

            results["tenant"][slug] = f"failed: {e}"

    # ------------------------------------------------------------------ #
    # 2. Platform admin report
    # ------------------------------------------------------------------ #
    try:
        logger.info("[platform] Building platform admin report...")
        data = build_platform_report(
            report_type=report_type,
            period_start=period_start,
            period_end=period_end,
        )

        pdf_bytes = render_platform_pdf(data)
        fname     = _filename("platform", "admin", report_type, period_start)

        emailed_at = None
        emailed_to = None

        if _PLATFORM_ADMIN_EMAIL and os.getenv("SENDGRID_API_KEY"):
            send_report_email(
                to_email=_PLATFORM_ADMIN_EMAIL,
                report_data=data,
                pdf_bytes=pdf_bytes,
                filename=fname,
            )
            emailed_at = datetime.utcnow()
            emailed_to = _PLATFORM_ADMIN_EMAIL
            logger.info("[platform] Emailed to %s", _PLATFORM_ADMIN_EMAIL)
        else:
            logger.warning("[platform] Skipping email (no PLATFORM_ADMIN_EMAIL or no SENDGRID_API_KEY)")

        master_conn = psycopg2.connect(_master_dsn())
        try:
            log_report(
                master_conn,
                report_type=report_type,
                scope="platform",
                tenant_slug=None,
                period_start=period_start,
                period_end=period_end,
                emailed_to=emailed_to,
                emailed_at=emailed_at,
                status="success",
                file_size_bytes=len(pdf_bytes),
            )
        finally:
            master_conn.close()

        results["platform"] = "success"

    except Exception as e:
        logger.error("[platform] Report failed: %s", e, exc_info=True)
        master_conn = psycopg2.connect(_master_dsn())
        try:
            log_report(
                master_conn,
                report_type=report_type,
                scope="platform",
                tenant_slug=None,
                period_start=period_start,
                period_end=period_end,
                emailed_to=None,
                emailed_at=None,
                status="failed",
                error_message=str(e),
            )
        finally:
            master_conn.close()

        results["platform"] = f"failed: {e}"

    logger.info("Report run complete: %s", results)
    return results


def run_daily_reports() -> dict[str, Any]:
    return _run_reports_for_type("daily")


def run_weekly_reports() -> dict[str, Any]:
    return _run_reports_for_type("weekly")


def run_monthly_reports() -> dict[str, Any]:
    return _run_reports_for_type("monthly")
