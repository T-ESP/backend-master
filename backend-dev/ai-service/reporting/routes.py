"""
Flask blueprint for on-demand report generation and history retrieval.

Endpoints:
    POST /ai/reports/generate  — generate (and optionally email) a report immediately
    GET  /ai/reports/history   — list past reports from reports_log
"""

import os
import threading
import psycopg2
from datetime import datetime
from flask import Blueprint, jsonify, request

from reporting.builder import build_tenant_report, build_platform_report, period_for
from reporting.pdf_renderer import render_tenant_pdf, render_platform_pdf
from reporting.email_sender import send_report_email
from reporting.queries import (
    log_report, get_report_history, get_platform_tenant_list,
)
from utils.logger import get_logger

logger = get_logger("reporting.routes")
bp = Blueprint("reporting", __name__)

_VALID_TYPES = {"daily", "weekly", "monthly"}


def _master_dsn() -> str:
    return os.environ["DATABASE_URL"]


def _find_tenant(master_conn, slug: str) -> dict | None:
    with master_conn.cursor() as cur:
        cur.execute(
            "SELECT slug, name, email, db_name FROM commerces WHERE slug = %s",
            (slug,)
        )
        row = cur.fetchone()
    if not row:
        return None
    return {"slug": row[0], "name": row[1], "email": row[2], "db_name": row[3]}


def _generate_and_log(
    *,
    scope: str,
    tenant_info: dict | None,
    report_type: str,
    period_start: datetime,
    period_end: datetime,
    send_email: bool,
) -> dict:
    """Build, optionally email, and log one report. Returns a result dict."""
    master_conn = psycopg2.connect(_master_dsn())

    try:
        if scope == "tenant":
            data      = build_tenant_report(
                slug=tenant_info["slug"],
                name=tenant_info["name"],
                email=tenant_info["email"],
                db_name=tenant_info["db_name"],
                report_type=report_type,
                period_start=period_start,
                period_end=period_end,
            )
            pdf_bytes = render_tenant_pdf(data)
            fname     = (f"report_tenant_{tenant_info['slug']}"
                         f"_{report_type}_{period_start.strftime('%Y-%m-%d')}.pdf")
            to_email  = tenant_info["email"]
        else:
            data      = build_platform_report(
                report_type=report_type,
                period_start=period_start,
                period_end=period_end,
            )
            pdf_bytes = render_platform_pdf(data)
            fname     = (f"report_platform_{report_type}"
                         f"_{period_start.strftime('%Y-%m-%d')}.pdf")
            to_email  = os.getenv("PLATFORM_ADMIN_EMAIL", "")

        emailed_at = None
        emailed_to = None

        if send_email and to_email and os.getenv("SENDGRID_API_KEY"):
            send_report_email(
                to_email=to_email,
                report_data=data,
                pdf_bytes=pdf_bytes,
                filename=fname,
            )
            emailed_at = datetime.utcnow()
            emailed_to = to_email

        log_id = log_report(
            master_conn,
            report_type=report_type,
            scope=scope,
            tenant_slug=tenant_info["slug"] if tenant_info else None,
            period_start=period_start,
            period_end=period_end,
            emailed_to=emailed_to,
            emailed_at=emailed_at,
            status="success",
            file_size_bytes=len(pdf_bytes),
        )

        return {
            "status":          "success",
            "report_log_id":   log_id,
            "scope":           scope,
            "tenant_slug":     tenant_info["slug"] if tenant_info else None,
            "report_type":     report_type,
            "period_start":    period_start.isoformat(),
            "period_end":      period_end.isoformat(),
            "file_size_bytes": len(pdf_bytes),
            "emailed_to":      emailed_to,
        }

    except Exception as e:
        logger.error("Report generation failed: %s", e, exc_info=True)
        log_report(
            master_conn,
            report_type=report_type,
            scope=scope,
            tenant_slug=tenant_info["slug"] if tenant_info else None,
            period_start=period_start,
            period_end=period_end,
            emailed_to=None,
            emailed_at=None,
            status="failed",
            error_message=str(e),
        )
        raise
    finally:
        master_conn.close()


@bp.route("/ai/reports/generate", methods=["POST"])
def generate_report():
    """
    Generate a report on demand.

    Body (JSON):
        report_type  str   "daily" | "weekly" | "monthly"  (required)
        scope        str   "tenant" | "platform"            (default: "tenant")
        tenant_slug  str   required when scope="tenant"
        send_email   bool  send the PDF by email             (default: true)

    Returns 202 with a result dict (runs synchronously — reports are fast enough).
    """
    body = request.get_json(silent=True) or {}

    report_type = body.get("report_type", "daily")
    scope       = body.get("scope", "tenant")
    slug        = body.get("tenant_slug")
    send_email  = bool(body.get("send_email", True))

    if report_type not in _VALID_TYPES:
        return jsonify({"error": f"report_type must be one of {sorted(_VALID_TYPES)}"}), 400
    if scope not in ("tenant", "platform"):
        return jsonify({"error": "scope must be 'tenant' or 'platform'"}), 400
    if scope == "tenant" and not slug:
        return jsonify({"error": "tenant_slug is required when scope='tenant'"}), 400

    period_start, period_end = period_for(report_type)

    master_conn = psycopg2.connect(_master_dsn())
    try:
        if scope == "tenant":
            tenant_info = _find_tenant(master_conn, slug)
            if not tenant_info:
                return jsonify({"error": f"Tenant '{slug}' not found"}), 404
        else:
            tenant_info = None
    finally:
        master_conn.close()

    try:
        result = _generate_and_log(
            scope=scope,
            tenant_info=tenant_info,
            report_type=report_type,
            period_start=period_start,
            period_end=period_end,
            send_email=send_email,
        )
        return jsonify(result), 202

    except Exception as e:
        return jsonify({"error": str(e), "status": "failed"}), 500


@bp.route("/ai/reports/history", methods=["GET"])
def report_history():
    """
    List past generated reports from reports_log.

    Query params:
        tenant_slug  str  filter by tenant (optional)
        limit        int  max rows (default 50)
    """
    slug  = request.args.get("tenant_slug")
    limit = min(int(request.args.get("limit", 50)), 200)

    master_conn = psycopg2.connect(_master_dsn())
    try:
        rows = get_report_history(master_conn, limit=limit, tenant_slug=slug or None)
    finally:
        master_conn.close()

    return jsonify({"reports": rows, "count": len(rows)}), 200
