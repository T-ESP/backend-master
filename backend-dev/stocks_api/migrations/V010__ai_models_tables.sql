-- Migration: V009 - AI Models Tables
-- Tables for demand forecasting, classification, clustering, and supplier scoring

-- ============================================
-- DEMAND FORECASTS
-- Stores demand predictions with stock recommendations
-- ============================================
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
    urgency VARCHAR(20),  -- URGENT, HIGH, MEDIUM, LOW
    mape NUMERIC(6,2),    -- Mean Absolute Percentage Error
    rmse NUMERIC(10,2),   -- Root Mean Square Error
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_demand_forecasts_product ON demand_forecasts(product_id);
CREATE INDEX idx_demand_forecasts_date ON demand_forecasts(forecast_date);
CREATE INDEX idx_demand_forecasts_urgency ON demand_forecasts(urgency);

-- ============================================
-- PRODUCT CLASSIFICATIONS (ABC-XYZ)
-- Stores classification history for inventory strategy
-- ============================================
CREATE TABLE IF NOT EXISTS product_classifications (
    id SERIAL PRIMARY KEY,
    product_id INT NOT NULL REFERENCES products_pro(id_pro) ON DELETE CASCADE,
    abc_class CHAR(1) NOT NULL,  -- A, B, C
    xyz_class CHAR(1) NOT NULL,  -- X, Y, Z
    combined_class CHAR(2) NOT NULL,  -- AX, AY, AZ, BX, BY, BZ, CX, CY, CZ
    total_revenue NUMERIC(12,2),
    revenue_contribution_pct NUMERIC(5,2),
    total_units_sold INT,
    coefficient_of_variation NUMERIC(6,3),
    strategy VARCHAR(100),
    priority VARCHAR(20),  -- CRITICAL, HIGH, MEDIUM, LOW
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_classifications_product ON product_classifications(product_id);
CREATE INDEX idx_classifications_class ON product_classifications(combined_class);
CREATE INDEX idx_classifications_date ON product_classifications(created_at);

-- ============================================
-- PRODUCT CLUSTERS (ML K-Means)
-- Stores ML-based product segmentation results
-- ============================================
CREATE TABLE IF NOT EXISTS product_clusters (
    id SERIAL PRIMARY KEY,
    product_id INT NOT NULL REFERENCES products_pro(id_pro) ON DELETE CASCADE,
    cluster_id INT NOT NULL,
    cluster_name VARCHAR(50),
    -- Feature scores (0-1 normalized)
    revenue_score NUMERIC(6,3),
    variability_score NUMERIC(6,3),
    trend_score NUMERIC(6,3),
    seasonality_score NUMERIC(6,3),
    frequency_score NUMERIC(6,3),
    margin_score NUMERIC(6,3),
    -- Clustering quality metrics
    distance_to_centroid NUMERIC(10,4),
    n_clusters INT,
    silhouette_score NUMERIC(6,3),
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_clusters_product ON product_clusters(product_id);
CREATE INDEX idx_clusters_cluster ON product_clusters(cluster_id);
CREATE INDEX idx_clusters_date ON product_clusters(created_at);

-- ============================================
-- SUPPLIER SCORES
-- Stores supplier performance scoring history
-- ============================================
CREATE TABLE IF NOT EXISTS supplier_scores (
    id SERIAL PRIMARY KEY,
    supplier_id INT NOT NULL REFERENCES supplier_sup(id_sup) ON DELETE CASCADE,
    overall_score NUMERIC(5,2) NOT NULL,
    delivery_score NUMERIC(5,2),
    quality_score NUMERIC(5,2),
    lead_time_score NUMERIC(5,2),
    fulfillment_score NUMERIC(5,2),
    rating VARCHAR(20),  -- EXCELLENT, GOOD, ACCEPTABLE, POOR, UNACCEPTABLE
    total_restocks INT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_supplier_scores_supplier ON supplier_scores(supplier_id);
CREATE INDEX idx_supplier_scores_rating ON supplier_scores(rating);
CREATE INDEX idx_supplier_scores_date ON supplier_scores(created_at);

-- ============================================
-- VIEWS for easy querying
-- ============================================

-- Latest forecast per product
CREATE OR REPLACE VIEW v_latest_forecasts AS
SELECT DISTINCT ON (product_id)
    df.id,
    df.product_id,
    p.name_pro as product_name,
    df.forecast_date,
    df.forecast_days,
    df.total_predicted_demand,
    df.avg_daily_demand,
    df.current_stock,
    df.recommended_stock,
    df.reorder_quantity,
    df.days_until_stockout,
    df.urgency,
    df.mape
FROM demand_forecasts df
JOIN products_pro p ON df.product_id = p.id_pro
ORDER BY product_id, forecast_date DESC;

-- Latest classification per product
CREATE OR REPLACE VIEW v_latest_classifications AS
SELECT DISTINCT ON (product_id)
    pc.id,
    pc.product_id,
    p.name_pro as product_name,
    pc.abc_class,
    pc.xyz_class,
    pc.combined_class,
    pc.total_revenue,
    pc.revenue_contribution_pct,
    pc.strategy,
    pc.priority,
    pc.created_at
FROM product_classifications pc
JOIN products_pro p ON pc.product_id = p.id_pro
ORDER BY product_id, created_at DESC;

-- Latest cluster per product
CREATE OR REPLACE VIEW v_latest_clusters AS
SELECT DISTINCT ON (product_id)
    pcl.id,
    pcl.product_id,
    p.name_pro as product_name,
    pcl.cluster_id,
    pcl.cluster_name,
    pcl.revenue_score,
    pcl.variability_score,
    pcl.trend_score,
    pcl.created_at
FROM product_clusters pcl
JOIN products_pro p ON pcl.product_id = p.id_pro
ORDER BY product_id, created_at DESC;

-- Latest score per supplier
CREATE OR REPLACE VIEW v_latest_supplier_scores AS
SELECT DISTINCT ON (supplier_id)
    ss.id,
    ss.supplier_id,
    s.name_sup as supplier_name,
    ss.overall_score,
    ss.delivery_score,
    ss.quality_score,
    ss.lead_time_score,
    ss.fulfillment_score,
    ss.rating,
    ss.created_at
FROM supplier_scores ss
JOIN supplier_sup s ON ss.supplier_id = s.id_sup
ORDER BY supplier_id, created_at DESC;

-- Urgent restock alerts
CREATE OR REPLACE VIEW v_urgent_restocks AS
SELECT
    df.product_id,
    p.name_pro as product_name,
    df.current_stock,
    df.recommended_stock,
    df.reorder_quantity,
    df.days_until_stockout,
    df.urgency,
    df.avg_daily_demand,
    df.forecast_date
FROM demand_forecasts df
JOIN products_pro p ON df.product_id = p.id_pro
WHERE df.urgency IN ('URGENT', 'HIGH')
  AND df.forecast_date >= NOW() - INTERVAL '1 day'
ORDER BY
    CASE df.urgency
        WHEN 'URGENT' THEN 1
        WHEN 'HIGH' THEN 2
    END,
    df.days_until_stockout;
