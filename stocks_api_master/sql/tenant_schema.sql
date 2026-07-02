-- ============================================================================
-- Tenant schema - Applied when creating a new commerce database
-- Consolidated from backend/stocks_api/migrations/V001__init.sql
-- ============================================================================

-- ============================================================================
-- ENUM TYPES
-- ============================================================================
CREATE TYPE product_status_enum AS ENUM (
    'in_stock',
    'out_of_stock',
    'discontinued',
    'ordered'
);

CREATE TYPE notification_category_enum AS ENUM ('alert', 'suggestion');

CREATE TYPE discount_trigger_enum AS ENUM ('product', 'total_amount', 'quantity');
CREATE TYPE discount_action_enum  AS ENUM ('fixed_eur', 'percentage');
CREATE TYPE discount_scope_enum   AS ENUM ('per_product', 'global');

CREATE TYPE notification_status_enum AS ENUM (
    'new',
    'acknowledged',
    'in_progress',
    'resolved',
    'dismissed'
);

-- ============================================================================
-- CORE TABLES
-- ============================================================================

CREATE TABLE IF NOT EXISTS role_rol (
    id_rol     SERIAL PRIMARY KEY,
    name_rol   VARCHAR NOT NULL UNIQUE,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS users_usr (
    id_usr             SERIAL PRIMARY KEY,
    email_usr          VARCHAR NOT NULL UNIQUE,
    lastname_usr       VARCHAR NOT NULL,
    firstname_usr      VARCHAR NOT NULL,
    password_usr       VARCHAR NOT NULL,
    phone_usr          VARCHAR,
    status_usr         VARCHAR DEFAULT 'active',
    fidelity_code_usr  VARCHAR UNIQUE,
    created_at         TIMESTAMPTZ DEFAULT NOW(),
    updated_at         TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_users_fidelity_code ON users_usr(fidelity_code_usr);

CREATE TABLE IF NOT EXISTS role_user_rus (
    id_role_rus INTEGER NOT NULL,
    id_user_rus INTEGER NOT NULL,
    created_at  TIMESTAMPTZ DEFAULT NOW(),
    updated_at  TIMESTAMPTZ DEFAULT NOW(),
    PRIMARY KEY (id_role_rus, id_user_rus),
    FOREIGN KEY (id_role_rus) REFERENCES role_rol(id_rol) ON DELETE CASCADE,
    FOREIGN KEY (id_user_rus) REFERENCES users_usr(id_usr) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS supplier_sup (
    id_sup      SERIAL PRIMARY KEY,
    name_sup    VARCHAR NOT NULL UNIQUE,
    email_sup   VARCHAR NOT NULL UNIQUE,
    phone_sup   VARCHAR,
    address_sup VARCHAR NOT NULL,
    created_at  TIMESTAMPTZ DEFAULT NOW(),
    updated_at  TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS products_pro (
    id_pro                 SERIAL PRIMARY KEY,
    name_pro               VARCHAR NOT NULL UNIQUE,
    category_pro           VARCHAR NOT NULL,
    reference_pro          VARCHAR NOT NULL UNIQUE,
    supplier_id_pro        INTEGER NOT NULL,
    stock_quantity_pro     INTEGER NOT NULL,
    buying_price_pro       NUMERIC NOT NULL,
    status_pro             product_status_enum NOT NULL DEFAULT 'in_stock',
    date_last_reassor_pro  TIMESTAMPTZ NOT NULL,
    created_at             TIMESTAMPTZ DEFAULT NOW(),
    updated_at             TIMESTAMPTZ DEFAULT NOW(),
    FOREIGN KEY (supplier_id_pro) REFERENCES supplier_sup(id_sup) ON DELETE RESTRICT
);

CREATE TABLE IF NOT EXISTS productprices_prp (
    id_prp          SERIAL PRIMARY KEY,
    product_ref_prp INTEGER NOT NULL,
    price_prp       NUMERIC NOT NULL,
    created_at      TIMESTAMPTZ DEFAULT NOW(),
    updated_at      TIMESTAMPTZ DEFAULT NOW(),
    FOREIGN KEY (product_ref_prp) REFERENCES products_pro(id_pro) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS order_ord (
    id_ord               SERIAL PRIMARY KEY,
    user_id_ord          INTEGER NOT NULL,
    order_date_ord       TIMESTAMPTZ NOT NULL,
    status_ord           VARCHAR NOT NULL,
    amount_ord           NUMERIC NOT NULL,
    discount_amount_ord  NUMERIC NOT NULL DEFAULT 0,
    payment_method_ord   VARCHAR,
    created_at           TIMESTAMPTZ DEFAULT NOW(),
    updated_at           TIMESTAMPTZ DEFAULT NOW(),
    FOREIGN KEY (user_id_ord) REFERENCES users_usr(id_usr) ON DELETE RESTRICT
);

CREATE TABLE IF NOT EXISTS line_order_lor (
    id_lor         SERIAL PRIMARY KEY,
    order_id_lor   INTEGER NOT NULL,
    product_id_lor INTEGER NOT NULL,
    quantity_lor   INTEGER NOT NULL,
    unit_price_lor NUMERIC CHECK (unit_price_lor >= 0),
    line_total_lor NUMERIC NOT NULL,
    created_at     TIMESTAMPTZ DEFAULT NOW(),
    updated_at     TIMESTAMPTZ DEFAULT NOW(),
    FOREIGN KEY (order_id_lor)   REFERENCES order_ord(id_ord)   ON DELETE CASCADE,
    FOREIGN KEY (product_id_lor) REFERENCES products_pro(id_pro) ON DELETE RESTRICT
);

-- ============================================================================
-- LOYALTY TABLES
-- ============================================================================

CREATE TABLE IF NOT EXISTS loyalty_config_lco (
    id_lco             SERIAL PRIMARY KEY,
    euros_per_point    NUMERIC(10,2) NOT NULL DEFAULT 2.00  CHECK (euros_per_point > 0),
    points_required    INTEGER       NOT NULL DEFAULT 100   CHECK (points_required > 0),
    discount_percent   NUMERIC(5,2)  NOT NULL DEFAULT 5.00  CHECK (discount_percent > 0 AND discount_percent <= 100),
    created_at         TIMESTAMPTZ DEFAULT NOW(),
    updated_at         TIMESTAMPTZ DEFAULT NOW()
);

-- Idempotent : si la table existait déjà sans ces colonnes, on les ajoute
ALTER TABLE loyalty_config_lco
    ADD COLUMN IF NOT EXISTS points_required  INTEGER      NOT NULL DEFAULT 100,
    ADD COLUMN IF NOT EXISTS discount_percent NUMERIC(5,2) NOT NULL DEFAULT 5.00;

CREATE TABLE IF NOT EXISTS loyalty_points_lpo (
    id_lpo        SERIAL PRIMARY KEY,
    user_id_lpo   INTEGER NOT NULL,
    order_id_lpo  INTEGER,
    points_lpo    INTEGER NOT NULL CHECK (points_lpo <> 0),
    reason_lpo    VARCHAR,
    created_at    TIMESTAMPTZ DEFAULT NOW(),
    FOREIGN KEY (user_id_lpo)  REFERENCES users_usr(id_usr) ON DELETE CASCADE,
    FOREIGN KEY (order_id_lpo) REFERENCES order_ord(id_ord) ON DELETE CASCADE
);

-- Idempotent : si la table existait avec l'ancienne structure (order_id NOT NULL,
-- pas de colonne reason, check > 0), on met à jour
ALTER TABLE loyalty_points_lpo
    ALTER COLUMN order_id_lpo DROP NOT NULL,
    ADD COLUMN IF NOT EXISTS reason_lpo VARCHAR;

DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.check_constraints
        WHERE constraint_name = 'loyalty_points_lpo_points_lpo_check'
    ) THEN
        ALTER TABLE loyalty_points_lpo DROP CONSTRAINT loyalty_points_lpo_points_lpo_check;
        ALTER TABLE loyalty_points_lpo ADD CONSTRAINT loyalty_points_lpo_points_lpo_check CHECK (points_lpo <> 0);
    END IF;
END $$;

-- Backfill : attribue un code de fidélité aux utilisateurs existants qui n'en ont pas
UPDATE users_usr
SET fidelity_code_usr = 'FID-' || LPAD(id_usr::TEXT, 8, '0')
WHERE fidelity_code_usr IS NULL;

-- ============================================================================
-- RESTOCK TABLES
-- ============================================================================

CREATE TABLE IF NOT EXISTS restock_res (
    id_res            SERIAL PRIMARY KEY,
    supplier_id_res   INTEGER,
    quantity_res      INTEGER NOT NULL CHECK (quantity_res > 0),
    status_res        VARCHAR(50) DEFAULT 'pending' CHECK (status_res IN ('pending', 'in_transit', 'received', 'cancelled')),
    restock_date_res  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at        TIMESTAMPTZ DEFAULT NOW(),
    updated_at        TIMESTAMPTZ DEFAULT NOW(),
    CONSTRAINT fk_restock_supplier FOREIGN KEY (supplier_id_res) REFERENCES supplier_sup(id_sup) ON DELETE SET NULL
);

CREATE TABLE IF NOT EXISTS line_restock_lrs (
    id_lrs              SERIAL PRIMARY KEY,
    restock_id_lrs      INTEGER NOT NULL,
    product_id_lrs      INTEGER NOT NULL,
    quantity_lrs        INTEGER NOT NULL CHECK (quantity_lrs > 0),
    unit_price_lrs      NUMERIC NOT NULL CHECK (unit_price_lrs >= 0),
    total_price_lrs     NUMERIC NOT NULL CHECK (total_price_lrs >= 0),
    created_at          TIMESTAMPTZ DEFAULT NOW(),
    updated_at          TIMESTAMPTZ DEFAULT NOW(),
    FOREIGN KEY (restock_id_lrs) REFERENCES restock_res(id_res) ON DELETE CASCADE,
    FOREIGN KEY (product_id_lrs) REFERENCES products_pro(id_pro) ON DELETE RESTRICT
);

CREATE TABLE IF NOT EXISTS productrestockprices_prr (
    id_prr              SERIAL PRIMARY KEY,
    product_ref_prr     INTEGER NOT NULL,
    buying_price_prr    NUMERIC NOT NULL CHECK (buying_price_prr >= 0),
    restock_id_prr      INTEGER NOT NULL,
    restock_date_prr    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at          TIMESTAMPTZ DEFAULT NOW(),
    updated_at          TIMESTAMPTZ DEFAULT NOW(),
    FOREIGN KEY (product_ref_prr) REFERENCES products_pro(id_pro) ON DELETE CASCADE,
    FOREIGN KEY (restock_id_prr) REFERENCES restock_res(id_res) ON DELETE CASCADE
);

-- ============================================================================
-- DISCOUNT TABLES
-- ============================================================================

CREATE TABLE IF NOT EXISTS discounts_dis (
    id_dis                  SERIAL PRIMARY KEY,
    name_dis                VARCHAR NOT NULL,
    trigger_type_dis        discount_trigger_enum NOT NULL,
    trigger_product_id_dis  INTEGER REFERENCES products_pro(id_pro) ON DELETE SET NULL,
    trigger_min_amount_dis  NUMERIC,
    trigger_min_qty_dis     INTEGER,
    action_type_dis         discount_action_enum NOT NULL,
    action_value_dis        NUMERIC NOT NULL CHECK (action_value_dis > 0),
    scope_dis               discount_scope_enum NOT NULL,
    scope_product_id_dis    INTEGER REFERENCES products_pro(id_pro) ON DELETE SET NULL,
    cumulative_dis          BOOLEAN NOT NULL DEFAULT FALSE,
    valid_from_dis          TIMESTAMPTZ,
    valid_until_dis         TIMESTAMPTZ,
    is_active_dis           BOOLEAN NOT NULL DEFAULT TRUE,
    created_at              TIMESTAMPTZ DEFAULT NOW(),
    updated_at              TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS order_discounts_odc (
    order_id_odc      INTEGER NOT NULL REFERENCES order_ord(id_ord)    ON DELETE CASCADE,
    discount_id_odc   INTEGER NOT NULL REFERENCES discounts_dis(id_dis) ON DELETE RESTRICT,
    saving_amount_odc NUMERIC NOT NULL,
    PRIMARY KEY (order_id_odc, discount_id_odc)
);

-- ============================================================================
-- AI SERVICE TABLES
-- ============================================================================

CREATE TABLE IF NOT EXISTS price_suggestions (
    id SERIAL PRIMARY KEY,
    product_id INT NOT NULL REFERENCES products_pro(id_pro),
    suggested_price NUMERIC NOT NULL,
    current_price NUMERIC NOT NULL,
    reason TEXT,
    confidence FLOAT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS price_anomalies (
    id SERIAL PRIMARY KEY,
    product_id INT NOT NULL REFERENCES products_pro(id_pro),
    current_price NUMERIC NOT NULL,
    expected_price NUMERIC NOT NULL,
    anomaly_score FLOAT,
    is_anomaly BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS sales_anomalies (
    id SERIAL PRIMARY KEY,
    product_id INT NOT NULL REFERENCES products_pro(id_pro),
    sales_volume INT NOT NULL,
    expected_sales FLOAT,
    anomaly_score FLOAT,
    is_anomaly BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS notifications (
    id SERIAL PRIMARY KEY,
    product_id INT REFERENCES products_pro(id_pro),
    model_type VARCHAR NOT NULL,
    category notification_category_enum NOT NULL,
    notification_type VARCHAR NOT NULL,
    severity VARCHAR NOT NULL,
    message TEXT NOT NULL,
    action_recommended TEXT,
    related_result_id INT,
    status notification_status_enum NOT NULL DEFAULT 'new',
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS demand_forecasts (
    id SERIAL PRIMARY KEY,
    product_id INT NOT NULL REFERENCES products_pro(id_pro) ON DELETE CASCADE,
    forecast_date TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    forecast_days INT NOT NULL,
    total_predicted_demand NUMERIC(12,2) NOT NULL,
    avg_daily_demand NUMERIC(10,2) NOT NULL,
    current_stock INT,
    recommended_stock INT,
    reorder_quantity INT,
    days_until_stockout INT,
    urgency VARCHAR(20),
    mape NUMERIC(6,2),
    rmse NUMERIC(10,2),
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS product_classifications (
    id SERIAL PRIMARY KEY,
    product_id INT NOT NULL REFERENCES products_pro(id_pro) ON DELETE CASCADE,
    abc_class CHAR(1) NOT NULL,
    xyz_class CHAR(1) NOT NULL,
    combined_class CHAR(2) NOT NULL,
    total_revenue NUMERIC(12,2),
    revenue_contribution_pct NUMERIC(5,2),
    total_units_sold INT,
    coefficient_of_variation NUMERIC(6,3),
    strategy VARCHAR(100),
    priority VARCHAR(20),
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS product_clusters (
    id SERIAL PRIMARY KEY,
    product_id INT NOT NULL REFERENCES products_pro(id_pro) ON DELETE CASCADE,
    cluster_id INT NOT NULL,
    cluster_name VARCHAR(50),
    revenue_score NUMERIC(6,3),
    variability_score NUMERIC(6,3),
    trend_score NUMERIC(6,3),
    seasonality_score NUMERIC(6,3),
    frequency_score NUMERIC(6,3),
    margin_score NUMERIC(6,3),
    distance_to_centroid NUMERIC(10,4),
    n_clusters INT,
    silhouette_score NUMERIC(6,3),
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS supplier_scores (
    id SERIAL PRIMARY KEY,
    supplier_id INT NOT NULL REFERENCES supplier_sup(id_sup) ON DELETE CASCADE,
    overall_score NUMERIC(5,2) NOT NULL,
    delivery_score NUMERIC(5,2),
    quality_score NUMERIC(5,2),
    lead_time_score NUMERIC(5,2),
    fulfillment_score NUMERIC(5,2),
    rating VARCHAR(20),
    total_restocks INT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- ============================================================================
-- INDEXES
-- ============================================================================

CREATE INDEX IF NOT EXISTS idx_discounts_active   ON discounts_dis(is_active_dis);
CREATE INDEX IF NOT EXISTS idx_discounts_validity ON discounts_dis(valid_from_dis, valid_until_dis);
CREATE INDEX IF NOT EXISTS idx_odc_order          ON order_discounts_odc(order_id_odc);
CREATE INDEX IF NOT EXISTS idx_odc_discount       ON order_discounts_odc(discount_id_odc);

CREATE INDEX IF NOT EXISTS idx_lor_order ON line_order_lor(order_id_lor);
CREATE INDEX IF NOT EXISTS idx_lor_product ON line_order_lor(product_id_lor);
CREATE INDEX IF NOT EXISTS idx_products_status ON products_pro(status_pro);

CREATE INDEX IF NOT EXISTS idx_productprices_product_ref ON productprices_prp(product_ref_prp);
CREATE INDEX IF NOT EXISTS idx_productprices_created_at ON productprices_prp(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_productprices_product_date ON productprices_prp(product_ref_prp, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_restock_supplier ON restock_res(supplier_id_res);
CREATE INDEX IF NOT EXISTS idx_restock_status ON restock_res(status_res);
CREATE INDEX IF NOT EXISTS idx_line_restock_restock_id ON line_restock_lrs(restock_id_lrs);
CREATE INDEX IF NOT EXISTS idx_line_restock_product_id ON line_restock_lrs(product_id_lrs);
CREATE INDEX IF NOT EXISTS idx_productrestockprices_product ON productrestockprices_prr(product_ref_prr);
CREATE INDEX IF NOT EXISTS idx_productrestockprices_restock ON productrestockprices_prr(restock_id_prr);
CREATE INDEX IF NOT EXISTS idx_productrestockprices_date ON productrestockprices_prr(restock_date_prr DESC);

CREATE INDEX IF NOT EXISTS idx_demand_forecasts_product ON demand_forecasts(product_id);
CREATE INDEX IF NOT EXISTS idx_demand_forecasts_date ON demand_forecasts(forecast_date);
CREATE INDEX IF NOT EXISTS idx_demand_forecasts_urgency ON demand_forecasts(urgency);
CREATE INDEX IF NOT EXISTS idx_classifications_product ON product_classifications(product_id);
CREATE INDEX IF NOT EXISTS idx_classifications_class ON product_classifications(combined_class);
CREATE INDEX IF NOT EXISTS idx_classifications_date ON product_classifications(created_at);
CREATE INDEX IF NOT EXISTS idx_clusters_product ON product_clusters(product_id);
CREATE INDEX IF NOT EXISTS idx_clusters_cluster ON product_clusters(cluster_id);
CREATE INDEX IF NOT EXISTS idx_clusters_date ON product_clusters(created_at);
CREATE INDEX IF NOT EXISTS idx_supplier_scores_supplier ON supplier_scores(supplier_id);
CREATE INDEX IF NOT EXISTS idx_supplier_scores_rating ON supplier_scores(rating);
CREATE INDEX IF NOT EXISTS idx_supplier_scores_date ON supplier_scores(created_at);

CREATE INDEX IF NOT EXISTS idx_notifications_status ON notifications(status);
CREATE INDEX IF NOT EXISTS idx_notifications_severity ON notifications(severity);
CREATE INDEX IF NOT EXISTS idx_notifications_model_type ON notifications(model_type);
CREATE INDEX IF NOT EXISTS idx_notifications_product ON notifications(product_id);
CREATE INDEX IF NOT EXISTS idx_notifications_created_at ON notifications(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_notifications_status_severity ON notifications(status, severity);

CREATE INDEX IF NOT EXISTS idx_loyalty_points_user  ON loyalty_points_lpo(user_id_lpo);
CREATE INDEX IF NOT EXISTS idx_loyalty_points_order ON loyalty_points_lpo(order_id_lpo);

-- ============================================================================
-- TRIGGER FUNCTIONS
-- ============================================================================

CREATE OR REPLACE FUNCTION update_product_status_on_stock_change()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.status_pro IN ('in_stock', 'out_of_stock') THEN
        IF NEW.stock_quantity_pro = 0 THEN
            IF NEW.status_pro = 'in_stock' THEN
                NEW.status_pro := 'out_of_stock'::product_status_enum;
            END IF;
        ELSIF NEW.stock_quantity_pro > 0 THEN
            IF NEW.status_pro = 'out_of_stock' THEN
                NEW.status_pro := 'in_stock'::product_status_enum;
            END IF;
        END IF;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_update_product_status
BEFORE INSERT OR UPDATE ON products_pro
FOR EACH ROW
EXECUTE FUNCTION update_product_status_on_stock_change();

CREATE OR REPLACE FUNCTION trg_calculate_line_restock_total()
RETURNS TRIGGER AS $$
BEGIN
    NEW.total_price_lrs := NEW.quantity_lrs * NEW.unit_price_lrs;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_line_restock_total
BEFORE INSERT OR UPDATE OF quantity_lrs, unit_price_lrs ON line_restock_lrs
FOR EACH ROW
EXECUTE FUNCTION trg_calculate_line_restock_total();

CREATE OR REPLACE FUNCTION trg_calculate_line_order_total()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.unit_price_lor IS NOT NULL THEN
        NEW.line_total_lor := NEW.quantity_lor * NEW.unit_price_lor;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_line_order_total
BEFORE INSERT OR UPDATE OF quantity_lor, unit_price_lor ON line_order_lor
FOR EACH ROW
EXECUTE FUNCTION trg_calculate_line_order_total();

CREATE OR REPLACE FUNCTION trg_record_restock_price_history()
RETURNS TRIGGER AS $$
BEGIN
    INSERT INTO productrestockprices_prr (
        product_ref_prr, buying_price_prr, restock_id_prr, restock_date_prr
    ) VALUES (
        NEW.product_id_lrs, NEW.unit_price_lrs, NEW.restock_id_lrs, NOW()
    );
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION trg_apply_restock_to_product_on_received()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.status_res = 'received' AND (OLD.status_res IS NULL OR OLD.status_res != 'received') THEN
        UPDATE products_pro p
        SET stock_quantity_pro = stock_quantity_pro + lr.quantity_lrs,
            date_last_reassor_pro = NEW.restock_date_res,
            updated_at = NOW()
        FROM line_restock_lrs lr
        WHERE lr.restock_id_lrs = NEW.id_res
          AND p.id_pro = lr.product_id_lrs;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_restock_update_stock_on_received
AFTER UPDATE OF status_res ON restock_res
FOR EACH ROW
EXECUTE FUNCTION trg_apply_restock_to_product_on_received();

-- ============================================================================
-- VIEWS
-- ============================================================================

CREATE OR REPLACE VIEW v_latest_forecasts AS
SELECT DISTINCT ON (product_id)
    df.id, df.product_id, p.name_pro as product_name,
    df.forecast_date, df.forecast_days, df.total_predicted_demand,
    df.avg_daily_demand, df.current_stock, df.recommended_stock,
    df.reorder_quantity, df.days_until_stockout, df.urgency, df.mape
FROM demand_forecasts df
JOIN products_pro p ON df.product_id = p.id_pro
ORDER BY product_id, forecast_date DESC;

CREATE OR REPLACE VIEW v_latest_classifications AS
SELECT DISTINCT ON (product_id)
    pc.id, pc.product_id, p.name_pro as product_name,
    pc.abc_class, pc.xyz_class, pc.combined_class,
    pc.total_revenue, pc.revenue_contribution_pct,
    pc.strategy, pc.priority, pc.created_at
FROM product_classifications pc
JOIN products_pro p ON pc.product_id = p.id_pro
ORDER BY product_id, created_at DESC;

CREATE OR REPLACE VIEW v_latest_clusters AS
SELECT DISTINCT ON (product_id)
    pcl.id, pcl.product_id, p.name_pro as product_name,
    pcl.cluster_id, pcl.cluster_name,
    pcl.revenue_score, pcl.variability_score, pcl.trend_score,
    pcl.created_at
FROM product_clusters pcl
JOIN products_pro p ON pcl.product_id = p.id_pro
ORDER BY product_id, created_at DESC;

CREATE OR REPLACE VIEW v_latest_supplier_scores AS
SELECT DISTINCT ON (supplier_id)
    ss.id, ss.supplier_id, s.name_sup as supplier_name,
    ss.overall_score, ss.delivery_score, ss.quality_score,
    ss.lead_time_score, ss.fulfillment_score, ss.rating, ss.created_at
FROM supplier_scores ss
JOIN supplier_sup s ON ss.supplier_id = s.id_sup
ORDER BY supplier_id, created_at DESC;

CREATE OR REPLACE VIEW v_urgent_restocks AS
SELECT
    df.product_id, p.name_pro as product_name,
    df.current_stock, df.recommended_stock, df.reorder_quantity,
    df.days_until_stockout, df.urgency, df.avg_daily_demand, df.forecast_date
FROM demand_forecasts df
JOIN products_pro p ON df.product_id = p.id_pro
WHERE df.urgency IN ('URGENT', 'HIGH')
  AND df.forecast_date >= NOW() - INTERVAL '1 day'
ORDER BY
    CASE df.urgency WHEN 'URGENT' THEN 1 WHEN 'HIGH' THEN 2 END,
    df.days_until_stockout;

-- ============================================================================
-- CHATBOT (per-tenant conversations)
-- ============================================================================
-- Conversations are tenant-private, so they live in each tenant DB. Ownership is
-- keyed by owner_email (the authenticated commerce email) — master auth is
-- commerce-level, so there is no numeric per-user id in the token. The shared
-- RAG corpus (rag_documents / rag_chunks) lives in the master DB instead.

CREATE EXTENSION IF NOT EXISTS vector;
CREATE EXTENSION IF NOT EXISTS pgcrypto;

DO $$ BEGIN
    CREATE TYPE chat_role AS ENUM ('user', 'assistant', 'tool', 'system');
EXCEPTION WHEN duplicate_object THEN null; END $$;

DO $$ BEGIN
    CREATE TYPE pending_action_status AS ENUM ('pending', 'confirmed', 'cancelled', 'expired');
EXCEPTION WHEN duplicate_object THEN null; END $$;

CREATE TABLE IF NOT EXISTS chat_sessions (
    session_id   UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_email  TEXT NOT NULL,
    title        TEXT,
    provider     TEXT NOT NULL DEFAULT 'auto',
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS chat_sessions_owner_idx
    ON chat_sessions(owner_email, updated_at DESC);

CREATE TABLE IF NOT EXISTS chat_messages (
    message_id   BIGSERIAL PRIMARY KEY,
    session_id   UUID NOT NULL REFERENCES chat_sessions(session_id) ON DELETE CASCADE,
    role         chat_role NOT NULL,
    content      TEXT NOT NULL,
    tool_calls   JSONB,
    tool_name    TEXT,
    provider     TEXT,
    tokens_in    INTEGER,
    tokens_out   INTEGER,
    latency_ms   INTEGER,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS chat_messages_session_idx
    ON chat_messages(session_id, created_at);

CREATE TABLE IF NOT EXISTS chat_pending_actions (
    action_id    UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id   UUID NOT NULL REFERENCES chat_sessions(session_id) ON DELETE CASCADE,
    message_id   BIGINT REFERENCES chat_messages(message_id) ON DELETE SET NULL,
    tool_name    TEXT NOT NULL,
    tool_args    JSONB NOT NULL,
    status       pending_action_status NOT NULL DEFAULT 'pending',
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    resolved_at  TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS chat_pending_actions_session_idx
    ON chat_pending_actions(session_id, status);

CREATE TABLE IF NOT EXISTS chat_summaries (
    summary_id       BIGSERIAL PRIMARY KEY,
    session_id       UUID NOT NULL REFERENCES chat_sessions(session_id) ON DELETE CASCADE,
    up_to_message_id BIGINT NOT NULL,
    summary          TEXT NOT NULL,
    tokens_in        INTEGER,
    tokens_out       INTEGER,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS chat_summaries_session_idx
    ON chat_summaries(session_id, up_to_message_id);

-- Tenant-isolated response cache (unused while CHAT_RESPONSE_CACHE=false; kept
-- here so a future tenant-aware cache stays inside the tenant DB).
CREATE TABLE IF NOT EXISTS chat_response_cache (
    cache_id        BIGSERIAL PRIMARY KEY,
    query_hash      TEXT UNIQUE NOT NULL,
    query_embedding vector(384),
    response        TEXT NOT NULL,
    provider        TEXT,
    hit_count       INTEGER NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_hit_at     TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS chat_response_cache_embedding_idx
    ON chat_response_cache USING hnsw (query_embedding vector_cosine_ops);

CREATE OR REPLACE VIEW v_chat_usage_daily AS
SELECT
    s.owner_email,
    DATE(m.created_at) AS day,
    m.provider,
    COUNT(*) FILTER (WHERE m.role = 'user') AS user_messages,
    COUNT(*) FILTER (WHERE m.role = 'assistant') AS assistant_messages,
    COALESCE(SUM(m.tokens_in), 0) AS tokens_in,
    COALESCE(SUM(m.tokens_out), 0) AS tokens_out,
    COALESCE(SUM(m.latency_ms), 0) AS total_latency_ms
FROM chat_messages m
JOIN chat_sessions s ON m.session_id = s.session_id
GROUP BY s.owner_email, DATE(m.created_at), m.provider;
