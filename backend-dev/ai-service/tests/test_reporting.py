"""
Unit tests for the reporting module.

Uses mocks throughout — no real DB or network calls.
Run with: pytest tests/test_reporting.py -v
"""

import pytest
from datetime import datetime
from unittest.mock import MagicMock, patch, call


# ---------------------------------------------------------------------------
# builder.period_for
# ---------------------------------------------------------------------------

from reporting.builder import period_for, _period_label


class TestPeriodFor:
    def test_daily_is_yesterday(self):
        ref = datetime(2026, 1, 15, 14, 30)
        start, end = period_for("daily", reference_dt=ref)
        assert start == datetime(2026, 1, 14, 0, 0)
        assert end   == datetime(2026, 1, 15, 0, 0)

    def test_weekly_is_last_monday_to_sunday(self):
        # 2026-01-15 is a Thursday → last Mon was 2026-01-05, last Sun was 2026-01-11
        ref = datetime(2026, 1, 15)
        start, end = period_for("weekly", reference_dt=ref)
        assert start.weekday() == 0          # Monday
        assert (end - start).days == 7
        assert start == datetime(2026, 1, 5)

    def test_monthly_is_last_full_calendar_month(self):
        ref = datetime(2026, 3, 10)          # March → last month = February
        start, end = period_for("monthly", reference_dt=ref)
        assert start.month == 2
        assert start.day   == 1
        assert start.year  == 2026

    def test_invalid_type_raises(self):
        with pytest.raises(ValueError, match="Unknown report_type"):
            period_for("quarterly")


class TestPeriodLabel:
    def test_daily_label(self):
        label = _period_label("daily", datetime(2026, 1, 14))
        assert "January" in label and "14" in label and "2026" in label

    def test_monthly_label(self):
        label = _period_label("monthly", datetime(2026, 2, 1))
        assert "February" in label and "2026" in label


# ---------------------------------------------------------------------------
# queries — smoke test with a mock connection
# ---------------------------------------------------------------------------

from reporting.queries import get_sales_summary, get_stock_alerts, get_notifications_summary


def _mock_conn(fetchone_values=None, fetchall_values=None):
    """Build a minimal psycopg2 connection mock."""
    cursor = MagicMock()
    cursor.__enter__ = lambda s: s
    cursor.__exit__  = MagicMock(return_value=False)

    fetch_one_iter  = iter(fetchone_values or [])
    fetch_all_iter  = iter(fetchall_values or [])
    cursor.fetchone = lambda: next(fetch_one_iter, None)
    cursor.fetchall = lambda: next(fetch_all_iter, [])

    conn = MagicMock()
    conn.cursor = MagicMock(return_value=cursor)
    return conn


class TestGetSalesSummary:
    def test_returns_expected_keys(self):
        conn = _mock_conn(
            fetchone_values=[(10, 1500.00, 150.00, 8, 1, 1)],
            fetchall_values=[
                [("Widget A", "Electronics", 5, 50, 750.00),
                 ("Widget B", "Clothing",    3, 30, 450.00)]
            ],
        )
        start = datetime(2026, 1, 14)
        end   = datetime(2026, 1, 15)
        result = get_sales_summary(conn, start, end)

        assert result["total_orders"]    == 10
        assert result["total_revenue"]   == 1500.00
        assert result["avg_order_value"] == 150.00
        assert result["completed"]       == 8
        assert len(result["top_products"]) == 2
        assert result["top_products"][0]["name"] == "Widget A"

    def test_empty_period_returns_zeros(self):
        conn = _mock_conn(
            fetchone_values=[(0, 0, 0, 0, 0, 0)],
            fetchall_values=[[]],
        )
        result = get_sales_summary(conn, datetime(2026, 1, 1), datetime(2026, 1, 2))
        assert result["total_orders"]  == 0
        assert result["total_revenue"] == 0.0
        assert result["top_products"]  == []


class TestGetStockAlerts:
    def test_returns_counts_and_urgent_list(self):
        cursor = MagicMock()
        cursor.__enter__ = lambda s: s
        cursor.__exit__  = MagicMock(return_value=False)

        side_effects_fetchone = [(3,), (5,)]
        side_effects_fetchall = [[
            ("Product X", 2, 3, "URGENT", 50),
            ("Product Y", 5, 8, "HIGH",   20),
        ]]

        fetchone_call = 0
        fetchall_call = 0

        def fetchone():
            nonlocal fetchone_call
            val = side_effects_fetchone[fetchone_call]
            fetchone_call += 1
            return val

        def fetchall():
            nonlocal fetchall_call
            val = side_effects_fetchall[fetchall_call]
            fetchall_call += 1
            return val

        cursor.fetchone = fetchone
        cursor.fetchall = fetchall

        conn = MagicMock()
        conn.cursor = MagicMock(return_value=cursor)

        result = get_stock_alerts(conn)

        assert result["out_of_stock"]          == 3
        assert result["low_stock"]             == 5
        assert len(result["urgent_restocks"])  == 2
        assert result["urgent_restocks"][0]["urgency"] == "URGENT"


class TestGetNotificationsSummary:
    def test_structure(self):
        cursor = MagicMock()
        cursor.__enter__ = lambda s: s
        cursor.__exit__  = MagicMock(return_value=False)

        calls = [(5, 2, 1, 3, 2), (4,), (7,)]
        idx   = [0]

        def fetchone():
            val = calls[idx[0]]
            idx[0] += 1
            return val

        cursor.fetchone = fetchone
        conn = MagicMock()
        conn.cursor = MagicMock(return_value=cursor)

        start  = datetime(2026, 1, 14)
        end    = datetime(2026, 1, 15)
        result = get_notifications_summary(conn, start, end)

        assert result["new_in_period"]["total"]       == 5
        assert result["new_in_period"]["critical"]    == 1
        assert result["resolved_in_period"]           == 4
        assert result["total_unacknowledged"]         == 7


# ---------------------------------------------------------------------------
# pdf_renderer — confirm WeasyPrint produces non-empty bytes
# ---------------------------------------------------------------------------

class TestPdfRenderer:
    def test_tenant_pdf_produces_bytes(self):
        from reporting.pdf_renderer import render_tenant_pdf

        data = {
            "scope": "tenant",
            "report_type": "daily",
            "commerce": {"slug": "demo", "name": "Demo Store", "email": "demo@test.com"},
            "period": {"start": "2026-01-14", "end": "2026-01-14", "label": "January 14, 2026"},
            "generated_at": "2026-01-15 00:05 UTC",
            "sales": {
                "total_orders": 10, "total_revenue": 1500.0,
                "avg_order_value": 150.0, "completed": 8, "pending": 1, "cancelled": 1,
                "top_products": [
                    {"name": "Widget A", "category": "Electronics",
                     "order_count": 5, "units_sold": 20, "revenue": 800.0}
                ],
            },
            "stock": {
                "out_of_stock": 2, "low_stock": 3,
                "urgent_restocks": [
                    {"product": "Widget X", "current_stock": 1,
                     "days_to_stockout": 2, "urgency": "URGENT", "reorder_qty": 50}
                ],
            },
            "insights": {
                "price_anomalies": {"detected": 1, "checked": 20},
                "sales_anomalies": {"detected": 0, "checked": 20},
                "abc_distribution": {"A": 5, "B": 12, "C": 33},
                "top_price_suggestions": [
                    {"product": "Widget A", "current_price": 9.99,
                     "suggested_price": 11.49, "confidence": 0.85, "reason": "High demand"}
                ],
            },
            "notifications": {
                "new_in_period": {"total": 3, "high": 1, "critical": 0, "alerts": 2, "suggestions": 1},
                "resolved_in_period": 2,
                "total_unacknowledged": 5,
            },
        }

        pdf = render_tenant_pdf(data)
        assert isinstance(pdf, bytes)
        assert len(pdf) > 1000         # a real PDF is never this small
        assert pdf[:4] == b"%PDF"      # PDF magic bytes

    def test_platform_pdf_produces_bytes(self):
        from reporting.pdf_renderer import render_platform_pdf

        data = {
            "scope": "platform",
            "report_type": "daily",
            "period": {"start": "2026-01-14", "end": "2026-01-14", "label": "January 14, 2026"},
            "generated_at": "2026-01-15 00:05 UTC",
            "platform": {
                "total_tenants": 3, "active_tenants": 2,
                "total_revenue": 4500.0, "total_orders": 30,
            },
            "tenants": [
                {"slug": "acme", "name": "Acme Corp", "email": "acme@test.com",
                 "status": "active", "created_at": "2025-01-01T00:00:00",
                 "orders": 20, "revenue": 3000.0, "open_alerts": 1, "out_of_stock": 0},
                {"slug": "beta", "name": "Beta Shop", "email": "beta@test.com",
                 "status": "active", "created_at": "2025-06-01T00:00:00",
                 "orders": 10, "revenue": 1500.0, "open_alerts": 0, "out_of_stock": 2},
            ],
        }

        pdf = render_platform_pdf(data)
        assert isinstance(pdf, bytes)
        assert len(pdf) > 1000
        assert pdf[:4] == b"%PDF"


# ---------------------------------------------------------------------------
# email_sender — mock SendGrid, assert correct structure
# ---------------------------------------------------------------------------

class TestSendReportEmail:
    @patch("reporting.email_sender._sg_client")
    def test_sends_with_correct_subject_tenant(self, mock_sg_client):
        from reporting.email_sender import send_report_email

        mock_response         = MagicMock()
        mock_response.status_code = 202
        mock_client           = MagicMock()
        mock_client.send      = MagicMock(return_value=mock_response)
        mock_sg_client.return_value = mock_client

        data = {
            "scope": "tenant", "report_type": "daily",
            "commerce": {"name": "Acme Corp"},
            "period": {"label": "January 14, 2026"},
            "generated_at": "2026-01-15 00:05 UTC",
            "sales": {"total_revenue": 1500.0, "total_orders": 10},
            "stock": {"out_of_stock": 0, "urgent_restocks": []},
            "notifications": {"new_in_period": {"total": 1, "critical": 0}},
        }

        send_report_email(
            to_email="owner@acme.com",
            report_data=data,
            pdf_bytes=b"%PDF fake content",
            filename="report.pdf",
        )

        mock_client.send.assert_called_once()
        mail_obj = mock_client.send.call_args[0][0]
        assert "Daily Report" in mail_obj.subject.subject
        assert "Acme Corp"    in mail_obj.subject.subject

    @patch("reporting.email_sender._sg_client")
    def test_raises_on_non_202(self, mock_sg_client):
        from reporting.email_sender import send_report_email

        mock_response         = MagicMock()
        mock_response.status_code = 400
        mock_response.body    = b"Bad Request"
        mock_client           = MagicMock()
        mock_client.send      = MagicMock(return_value=mock_response)
        mock_sg_client.return_value = mock_client

        data = {
            "scope": "tenant", "report_type": "daily",
            "commerce": {"name": "X"},
            "period": {"label": "Jan 1"},
            "generated_at": "now",
            "sales": {"total_revenue": 0, "total_orders": 0},
            "stock": {"out_of_stock": 0, "urgent_restocks": []},
            "notifications": {"new_in_period": {"total": 0, "critical": 0}},
        }

        with pytest.raises(RuntimeError, match="SendGrid"):
            send_report_email(
                to_email="x@x.com",
                report_data=data,
                pdf_bytes=b"fake",
                filename="r.pdf",
            )


# ---------------------------------------------------------------------------
# Flask routes — integration smoke test
# ---------------------------------------------------------------------------

class TestReportRoutes:
    @pytest.fixture
    def client(self):
        import sys, os
        sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))
        os.environ.setdefault("DATABASE_URL", "postgresql://user:pw@localhost/stocks")

        from main import app
        app.config["TESTING"] = True
        with app.test_client() as c:
            yield c

    def test_generate_missing_type(self, client):
        resp = client.post("/ai/reports/generate",
                           json={"report_type": "quarterly", "scope": "tenant",
                                 "tenant_slug": "demo"})
        assert resp.status_code == 400
        assert b"report_type" in resp.data

    def test_generate_tenant_missing_slug(self, client):
        resp = client.post("/ai/reports/generate",
                           json={"report_type": "daily", "scope": "tenant"})
        assert resp.status_code == 400
        assert b"tenant_slug" in resp.data

    @patch("reporting.routes.psycopg2.connect")
    @patch("reporting.routes._generate_and_log")
    def test_generate_calls_generate_and_log(self, mock_gen, mock_conn):
        import sys, os
        sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))
        os.environ.setdefault("DATABASE_URL", "postgresql://user:pw@localhost/stocks")

        mock_cursor = MagicMock()
        mock_cursor.__enter__ = lambda s: s
        mock_cursor.__exit__  = MagicMock(return_value=False)
        mock_cursor.fetchone  = MagicMock(return_value=("demo", "Demo Store", "demo@test.com", "tenant_demo"))
        mock_db = MagicMock()
        mock_db.cursor = MagicMock(return_value=mock_cursor)
        mock_conn.return_value = mock_db

        mock_gen.return_value = {
            "status": "success", "report_log_id": 1, "scope": "tenant",
            "tenant_slug": "demo", "report_type": "daily",
            "period_start": "2026-01-14T00:00:00", "period_end": "2026-01-15T00:00:00",
            "file_size_bytes": 5000, "emailed_to": "demo@test.com",
        }

        from main import app
        app.config["TESTING"] = True
        with app.test_client() as c:
            resp = c.post("/ai/reports/generate",
                          json={"report_type": "daily", "scope": "tenant",
                                "tenant_slug": "demo", "send_email": False})

        assert resp.status_code == 202
        mock_gen.assert_called_once()
