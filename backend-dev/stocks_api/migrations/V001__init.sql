-- ============================================================================
-- V001__init.sql - Complete database schema (consolidated from V001-V011)
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

-- Roles
CREATE TABLE IF NOT EXISTS role_rol (
    id_rol     SERIAL PRIMARY KEY,
    name_rol   VARCHAR NOT NULL UNIQUE,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Users
CREATE TABLE IF NOT EXISTS users_usr (
    id_usr        SERIAL PRIMARY KEY,
    email_usr     VARCHAR NOT NULL UNIQUE,
    lastname_usr  VARCHAR NOT NULL,
    firstname_usr VARCHAR NOT NULL,
    password_usr  VARCHAR NOT NULL,
    phone_usr     VARCHAR,
    status_usr    VARCHAR DEFAULT 'active',
    created_at    TIMESTAMPTZ DEFAULT NOW(),
    updated_at    TIMESTAMPTZ DEFAULT NOW()
);

-- Role-User junction
CREATE TABLE IF NOT EXISTS role_user_rus (
    id_role_rus INTEGER NOT NULL,
    id_user_rus INTEGER NOT NULL,
    created_at  TIMESTAMPTZ DEFAULT NOW(),
    updated_at  TIMESTAMPTZ DEFAULT NOW(),
    PRIMARY KEY (id_role_rus, id_user_rus),
    FOREIGN KEY (id_role_rus) REFERENCES role_rol(id_rol) ON DELETE CASCADE,
    FOREIGN KEY (id_user_rus) REFERENCES users_usr(id_usr) ON DELETE CASCADE
);

-- Suppliers
CREATE TABLE IF NOT EXISTS supplier_sup (
    id_sup      SERIAL PRIMARY KEY,
    name_sup    VARCHAR NOT NULL UNIQUE,
    email_sup   VARCHAR NOT NULL UNIQUE,
    phone_sup   VARCHAR,
    address_sup VARCHAR NOT NULL,
    created_at  TIMESTAMPTZ DEFAULT NOW(),
    updated_at  TIMESTAMPTZ DEFAULT NOW()
);

-- Products (with ENUM status from the start)
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

-- Product prices (NO unique constraint - allows price history)
CREATE TABLE IF NOT EXISTS productprices_prp (
    id_prp          SERIAL PRIMARY KEY,
    product_ref_prp INTEGER NOT NULL,
    price_prp       NUMERIC NOT NULL,
    created_at      TIMESTAMPTZ DEFAULT NOW(),
    updated_at      TIMESTAMPTZ DEFAULT NOW(),
    FOREIGN KEY (product_ref_prp) REFERENCES products_pro(id_pro) ON DELETE CASCADE
);

COMMENT ON TABLE productprices_prp IS 'Price history for products. Each row represents a price change.';
COMMENT ON COLUMN productprices_prp.product_ref_prp IS 'Product reference (multiple entries for history)';
COMMENT ON COLUMN productprices_prp.price_prp IS 'Selling price at this date';
COMMENT ON COLUMN productprices_prp.created_at IS 'Date this price took effect';

-- Orders
CREATE TABLE IF NOT EXISTS order_ord (
    id_ord         SERIAL PRIMARY KEY,
    user_id_ord    INTEGER NOT NULL,
    order_date_ord TIMESTAMPTZ NOT NULL,
    status_ord     VARCHAR NOT NULL,
    amount_ord     NUMERIC NOT NULL,
    created_at     TIMESTAMPTZ DEFAULT NOW(),
    updated_at     TIMESTAMPTZ DEFAULT NOW(),
    FOREIGN KEY (user_id_ord) REFERENCES users_usr(id_usr) ON DELETE RESTRICT
);

-- Line items (order lines)
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

COMMENT ON COLUMN line_order_lor.line_total_lor IS 'Total price for this line (unit_price_lor * quantity_lor)';
COMMENT ON COLUMN line_order_lor.unit_price_lor IS 'Unit price of the product at order time';

-- ============================================================================
-- RESTOCK TABLES
-- ============================================================================

-- Restocks
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

COMMENT ON COLUMN restock_res.supplier_id_res IS 'Main supplier for this restock';
COMMENT ON COLUMN restock_res.status_res IS 'Restock status: pending, in_transit, received, cancelled';

-- Restock line items
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

-- Restock price history
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
-- AI SERVICE TABLES
-- ============================================================================

-- Price suggestions
CREATE TABLE IF NOT EXISTS price_suggestions (
    id SERIAL PRIMARY KEY,
    product_id INT NOT NULL REFERENCES products_pro(id_pro),
    suggested_price NUMERIC NOT NULL,
    current_price NUMERIC NOT NULL,
    reason TEXT,
    confidence FLOAT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Price anomalies
CREATE TABLE IF NOT EXISTS price_anomalies (
    id SERIAL PRIMARY KEY,
    product_id INT NOT NULL REFERENCES products_pro(id_pro),
    current_price NUMERIC NOT NULL,
    expected_price NUMERIC NOT NULL,
    anomaly_score FLOAT,
    is_anomaly BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Sales anomalies
CREATE TABLE IF NOT EXISTS sales_anomalies (
    id SERIAL PRIMARY KEY,
    product_id INT NOT NULL REFERENCES products_pro(id_pro),
    sales_volume INT NOT NULL,
    expected_sales FLOAT,
    anomaly_score FLOAT,
    is_anomaly BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Notifications / Alerts
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

-- Demand forecasts
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

-- Product classifications (ABC-XYZ)
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

-- Product clusters (K-Means)
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

-- Supplier scores
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

-- Core indexes
CREATE INDEX IF NOT EXISTS idx_lor_order ON line_order_lor(order_id_lor);
CREATE INDEX IF NOT EXISTS idx_lor_product ON line_order_lor(product_id_lor);
CREATE INDEX IF NOT EXISTS idx_products_status ON products_pro(status_pro);

-- Product prices indexes
CREATE INDEX IF NOT EXISTS idx_productprices_product_ref ON productprices_prp(product_ref_prp);
CREATE INDEX IF NOT EXISTS idx_productprices_created_at ON productprices_prp(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_productprices_product_date ON productprices_prp(product_ref_prp, created_at DESC);

-- Restock indexes
CREATE INDEX IF NOT EXISTS idx_restock_supplier ON restock_res(supplier_id_res);
CREATE INDEX IF NOT EXISTS idx_restock_status ON restock_res(status_res);
CREATE INDEX IF NOT EXISTS idx_line_restock_restock_id ON line_restock_lrs(restock_id_lrs);
CREATE INDEX IF NOT EXISTS idx_line_restock_product_id ON line_restock_lrs(product_id_lrs);
CREATE INDEX IF NOT EXISTS idx_productrestockprices_product ON productrestockprices_prr(product_ref_prr);
CREATE INDEX IF NOT EXISTS idx_productrestockprices_restock ON productrestockprices_prr(restock_id_prr);
CREATE INDEX IF NOT EXISTS idx_productrestockprices_date ON productrestockprices_prr(restock_date_prr DESC);

-- AI service indexes
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

-- Notification indexes
CREATE INDEX IF NOT EXISTS idx_notifications_status ON notifications(status);
CREATE INDEX IF NOT EXISTS idx_notifications_severity ON notifications(severity);
CREATE INDEX IF NOT EXISTS idx_notifications_model_type ON notifications(model_type);
CREATE INDEX IF NOT EXISTS idx_notifications_product ON notifications(product_id);
CREATE INDEX IF NOT EXISTS idx_notifications_created_at ON notifications(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_notifications_status_severity ON notifications(status, severity);

-- ============================================================================
-- TRIGGER FUNCTIONS
-- ============================================================================

-- Auto-update product status based on stock level
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

-- Auto-calculate line restock total
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

-- Auto-calculate line order total
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

-- Record restock price history (disabled for seeding - enable in production)
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

-- NOTE: Trigger disabled to allow varied historical dates during seeding
-- CREATE TRIGGER trg_restock_price_history
-- AFTER INSERT ON line_restock_lrs
-- FOR EACH ROW
-- EXECUTE FUNCTION trg_record_restock_price_history();

-- Apply restock to product stock when status changes to 'received'
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

-- Latest forecast per product
CREATE OR REPLACE VIEW v_latest_forecasts AS
SELECT DISTINCT ON (product_id)
    df.id, df.product_id, p.name_pro as product_name,
    df.forecast_date, df.forecast_days, df.total_predicted_demand,
    df.avg_daily_demand, df.current_stock, df.recommended_stock,
    df.reorder_quantity, df.days_until_stockout, df.urgency, df.mape
FROM demand_forecasts df
JOIN products_pro p ON df.product_id = p.id_pro
ORDER BY product_id, forecast_date DESC;

-- Latest classification per product
CREATE OR REPLACE VIEW v_latest_classifications AS
SELECT DISTINCT ON (product_id)
    pc.id, pc.product_id, p.name_pro as product_name,
    pc.abc_class, pc.xyz_class, pc.combined_class,
    pc.total_revenue, pc.revenue_contribution_pct,
    pc.strategy, pc.priority, pc.created_at
FROM product_classifications pc
JOIN products_pro p ON pc.product_id = p.id_pro
ORDER BY product_id, created_at DESC;

-- Latest cluster per product
CREATE OR REPLACE VIEW v_latest_clusters AS
SELECT DISTINCT ON (product_id)
    pcl.id, pcl.product_id, p.name_pro as product_name,
    pcl.cluster_id, pcl.cluster_name,
    pcl.revenue_score, pcl.variability_score, pcl.trend_score,
    pcl.created_at
FROM product_clusters pcl
JOIN products_pro p ON pcl.product_id = p.id_pro
ORDER BY product_id, created_at DESC;

-- Latest score per supplier
CREATE OR REPLACE VIEW v_latest_supplier_scores AS
SELECT DISTINCT ON (supplier_id)
    ss.id, ss.supplier_id, s.name_sup as supplier_name,
    ss.overall_score, ss.delivery_score, ss.quality_score,
    ss.lead_time_score, ss.fulfillment_score, ss.rating, ss.created_at
FROM supplier_scores ss
JOIN supplier_sup s ON ss.supplier_id = s.id_sup
ORDER BY supplier_id, created_at DESC;

-- Urgent restock alerts
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