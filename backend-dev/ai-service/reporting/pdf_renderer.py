"""
PDF rendering: report data dict → PDF bytes via WeasyPrint + Jinja2.
"""

import os
from pathlib import Path
from typing import Any

from jinja2 import Environment, FileSystemLoader, select_autoescape
from weasyprint import HTML, CSS

_TEMPLATE_DIR = Path(__file__).parent / "templates"
_jinja_env = Environment(
    loader=FileSystemLoader(str(_TEMPLATE_DIR)),
    autoescape=select_autoescape(["html"]),
)

_BASE_CSS = CSS(string="""
    @page { size: A4; margin: 15mm 12mm; }
    body { font-family: 'Helvetica Neue', Helvetica, Arial, sans-serif; font-size: 11px; color: #222; }
""")


def _add_filters(env: Environment) -> None:
    env.filters["eur"] = lambda v: f"€{float(v):,.2f}"
    env.filters["pct"] = lambda v: f"{float(v):.1f}%"
    env.filters["num"] = lambda v: f"{int(v):,}"


_add_filters(_jinja_env)


def render_tenant_pdf(report_data: dict[str, Any]) -> bytes:
    template = _jinja_env.get_template("tenant_report.html")
    html_str  = template.render(**report_data)
    return HTML(string=html_str, base_url=str(_TEMPLATE_DIR)).write_pdf(stylesheets=[_BASE_CSS])


def render_platform_pdf(report_data: dict[str, Any]) -> bytes:
    template = _jinja_env.get_template("platform_report.html")
    html_str  = template.render(**report_data)
    return HTML(string=html_str, base_url=str(_TEMPLATE_DIR)).write_pdf(stylesheets=[_BASE_CSS])
