-- ============================================================================
-- Reports audit log — stored in the master DB (platform-level table)
-- Tracks every report generated: tenant, type, period, email delivery status.
-- ============================================================================

CREATE TABLE IF NOT EXISTS reports_log (
    id              SERIAL PRIMARY KEY,
    report_type     VARCHAR(20)  NOT NULL,          -- 'daily', 'weekly', 'monthly'
    scope           VARCHAR(20)  NOT NULL,           -- 'tenant' or 'platform'
    tenant_slug     TEXT,                            -- NULL for platform-scope reports
    period_start    TIMESTAMPTZ  NOT NULL,
    period_end      TIMESTAMPTZ  NOT NULL,
    generated_at    TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    emailed_to      TEXT,                            -- recipient address, NULL if not sent
    emailed_at      TIMESTAMPTZ,                     -- NULL until email confirmed sent
    status          VARCHAR(20)  NOT NULL DEFAULT 'success', -- 'success', 'failed', 'skipped'
    error_message   TEXT,
    file_size_bytes INTEGER
);

CREATE INDEX IF NOT EXISTS idx_reports_log_tenant    ON reports_log(tenant_slug);
CREATE INDEX IF NOT EXISTS idx_reports_log_type      ON reports_log(report_type, scope);
CREATE INDEX IF NOT EXISTS idx_reports_log_generated ON reports_log(generated_at DESC);
CREATE INDEX IF NOT EXISTS idx_reports_log_status    ON reports_log(status);
