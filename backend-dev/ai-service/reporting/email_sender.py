"""
Email delivery via SendGrid.

Sends a PDF report as an attachment with a short HTML summary body.
Requires env vars:
    SENDGRID_API_KEY   — SendGrid API key
    REPORT_FROM_EMAIL  — sender address (e.g. reports@stock-s.fr)
    REPORT_FROM_NAME   — sender display name (default: "Stock Platform Reports")
"""

import base64
import os
from datetime import datetime
from typing import Any

import sendgrid
from sendgrid.helpers.mail import (
    Mail, Attachment, FileContent, FileName,
    FileType, Disposition, ContentId, To,
)

from utils.logger import get_logger

logger = get_logger("reporting.email")

_FROM_EMAIL = os.getenv("REPORT_FROM_EMAIL", "reports@stock-s.fr")
_FROM_NAME  = os.getenv("REPORT_FROM_NAME",  "Stock Platform Reports")


def _sg_client() -> sendgrid.SendGridAPIClient:
    api_key = os.environ["SENDGRID_API_KEY"]
    return sendgrid.SendGridAPIClient(api_key=api_key)


def _build_html_body(report_data: dict[str, Any]) -> str:
    scope       = report_data["scope"]
    report_type = report_data["report_type"].capitalize()
    period      = report_data["period"]["label"]
    generated   = report_data["generated_at"]

    if scope == "tenant":
        name    = report_data["commerce"]["name"]
        sales   = report_data["sales"]
        stock   = report_data["stock"]
        notifs  = report_data["notifications"]
        return f"""
        <p>Hello,</p>
        <p>Please find attached the <strong>{report_type} Report</strong> for <strong>{name}</strong>
        covering <strong>{period}</strong>.</p>
        <h3 style="margin-top:16px">Key highlights</h3>
        <ul>
          <li><strong>Revenue:</strong> €{sales['total_revenue']:,.2f} across {sales['total_orders']} orders</li>
          <li><strong>Out-of-stock products:</strong> {stock['out_of_stock']}</li>
          <li><strong>Urgent restocks needed:</strong> {len(stock['urgent_restocks'])}</li>
          <li><strong>New alerts:</strong> {notifs['new_in_period']['total']} ({notifs['new_in_period']['critical']} critical)</li>
        </ul>
        <p style="margin-top:16px;color:#718096;font-size:12px">
          Full details are in the attached PDF report.<br>
          Generated {generated}
        </p>
        """
    else:
        platform = report_data["platform"]
        return f"""
        <p>Hello,</p>
        <p>Please find attached the <strong>Platform {report_type} Report</strong>
        covering <strong>{period}</strong>.</p>
        <h3 style="margin-top:16px">Platform summary</h3>
        <ul>
          <li><strong>Active tenants:</strong> {platform['active_tenants']}</li>
          <li><strong>Total revenue:</strong> €{platform['total_revenue']:,.2f}</li>
          <li><strong>Total orders:</strong> {platform['total_orders']}</li>
        </ul>
        <p style="margin-top:16px;color:#718096;font-size:12px">
          Full details are in the attached PDF.<br>
          Generated {generated}
        </p>
        """


def send_report_email(
    *,
    to_email: str,
    report_data: dict[str, Any],
    pdf_bytes: bytes,
    filename: str,
) -> None:
    """Send a report PDF to `to_email`. Raises on delivery failure."""
    scope       = report_data["scope"]
    report_type = report_data["report_type"].capitalize()
    period      = report_data["period"]["label"]

    if scope == "tenant":
        name    = report_data["commerce"]["name"]
        subject = f"[{report_type} Report] {name} — {period}"
    else:
        subject = f"[Platform {report_type} Report] {period}"

    html_body = _build_html_body(report_data)

    message = Mail(
        from_email=(f"{_FROM_NAME} <{_FROM_EMAIL}>"),
        to_emails=To(to_email),
        subject=subject,
        html_content=html_body,
    )

    encoded = base64.b64encode(pdf_bytes).decode()
    attachment = Attachment(
        file_content=FileContent(encoded),
        file_type=FileType("application/pdf"),
        file_name=FileName(filename),
        disposition=Disposition("attachment"),
        content_id=ContentId("report_pdf"),
    )
    message.attachment = attachment

    client = _sg_client()
    response = client.send(message)

    if response.status_code not in (200, 202):
        raise RuntimeError(
            f"SendGrid returned status {response.status_code}: {response.body}"
        )

    logger.info("Report email sent to %s (status %s)", to_email, response.status_code)
