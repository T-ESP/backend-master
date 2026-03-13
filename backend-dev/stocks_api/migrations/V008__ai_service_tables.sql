CREATE TYPE notification_category_enum AS ENUM ('alert', 'suggestion');

CREATE TABLE price_suggestions (
    id SERIAL PRIMARY KEY,
    product_id INT NOT NULL REFERENCES products_pro(id_pro),
    suggested_price NUMERIC NOT NULL,
    current_price NUMERIC NOT NULL,
    reason TEXT,
    confidence FLOAT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE price_anomalies (
    id SERIAL PRIMARY KEY,
    product_id INT NOT NULL REFERENCES products_pro(id_pro),
    current_price NUMERIC NOT NULL,
    expected_price NUMERIC NOT NULL,
    anomaly_score FLOAT,
    is_anomaly BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE sales_anomalies (
    id SERIAL PRIMARY KEY,
    product_id INT NOT NULL REFERENCES products_pro(id_pro),
    sales_volume INT NOT NULL,
    expected_sales FLOAT,
    anomaly_score FLOAT,
    is_anomaly BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE notifications (
    id SERIAL PRIMARY KEY,
    product_id INT NOT NULL REFERENCES products_pro(id_pro),
    model_type VARCHAR NOT NULL,
    category notification_category_enum NOT NULL,
    notification_type VARCHAR NOT NULL,
    severity VARCHAR NOT NULL,
    message TEXT NOT NULL,
    action_recommended TEXT,
    related_result_id INT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    status VARCHAR DEFAULT 'unresolved'
);
