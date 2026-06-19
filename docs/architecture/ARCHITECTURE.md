# Architecture & Implementation Overview

## System Overview

The platform is a stock management system with integrated AI-powered analytics. It consists of two backend services sharing a PostgreSQL database:

```
                         +------------------+
                         |   PostgreSQL 16   |
                         |   (shared DB)     |
                         +--------+---------+
                                  |
                   +--------------+--------------+
                   |                              |
          +--------v--------+          +----------v---------+
          |   stocks_api    |          |    ai-service       |
          |  (Rust/Axum)    |          |    (Python/Flask)   |
          |  Port 8090      |          |    Port 8001        |
          +-----------------+          +--------------------+
          | REST API         |          | 7 ML models        |
          | JWT auth         |          | Cron scheduler     |
          | 100+ endpoints   |          | HTTP trigger       |
          +-----------------+          +--------------------+
                   |
          +--------v--------+
          |    Frontend     |
          |  (consumes API) |
          +-----------------+
```

**How it works:** The ai-service runs 7 ML models that analyze sales, prices, and inventory data. It writes predictions, anomalies, classifications, and alerts directly into the database. The stocks_api reads those results and exposes them through REST endpoints. The two services never communicate directly - the database is the integration layer.

---

## Docker Compose Services

| Service | Image | Purpose | Port |
|---------|-------|---------|------|
| `db` | postgres:16 | PostgreSQL database | 5432 |
| `migrate` | backend-web | Runs Refinery migrations then exits | - |
| `seed` | backend-web | Populates dev data then exits | - |
| `web` | backend-web | Rust REST API server | 8090 |
| `ai-service` | ai-service | Python ML + scheduler + Flask | 8001 |
| `pgadmin` | pgadmin4:8 | Database admin UI | 5050 |

**Startup order:** `db` (waits for healthy) -> `migrate` -> `seed` -> `web` + `ai-service`

---

## Database Schema

### Core Business Tables

| Table | Purpose | Key Columns |
|-------|---------|-------------|
| `users_usr` | User accounts | email, password (hashed), firstname, lastname, status |
| `role_rol` | Roles (admin, manager, seller, viewer, user) | name |
| `role_user_rus` | User-role assignments | id_role, id_user |
| `supplier_sup` | Suppliers | name, email, phone, address |
| `products_pro` | Product catalog | name, category, reference, supplier_id, stock_quantity, buying_price, status |
| `productprices_prp` | Selling price history | product_ref, price, created_at |
| `productrestockprices_prr` | Buying price history | product_ref, buying_price, restock_id, restock_date |
| `order_ord` | Customer orders | user_id, order_date, status, amount |
| `line_order_lor` | Order line items | order_id, product_id, quantity, unit_price, line_total |
| `restock_res` | Supplier restocks | quantity, supplier_id, status, restock_date |
| `line_restock_lrs` | Restock line items | restock_id, product_id, quantity, unit_price, total_price |

### AI/ML Output Tables

| Table | Written By | Purpose |
|-------|-----------|---------|
| `demand_forecasts` | DemandForecastHandler | Prophet time-series forecasts per product |
| `product_classifications` | ClassificationHandler | ABC-XYZ classification per product |
| `product_clusters` | ClusteringHandler | K-Means cluster assignment per product |
| `supplier_scores` | SupplierScoringHandler | Weighted performance scores per supplier |
| `price_suggestions` | PriceSuggestionHandler | ML-suggested optimal prices |
| `price_anomalies` | PriceAnomalyHandler | Isolation Forest price anomaly flags |
| `sales_anomalies` | SalesAnomalyHandler | Isolation Forest sales anomaly flags |
| `notifications` | All handlers | Alerts and suggestions with lifecycle status |

### Database Views

| View | Purpose |
|------|---------|
| `v_latest_forecasts` | Latest forecast per product (deduped) |
| `v_latest_classifications` | Latest ABC-XYZ class per product |
| `v_latest_clusters` | Latest cluster assignment per product |
| `v_latest_supplier_scores` | Latest score per supplier |
| `v_urgent_restocks` | Products needing urgent/high restock |

### Enums

| Enum | Values |
|------|--------|
| `product_status_enum` | in_stock, out_of_stock, discontinued, ordered |
| `notification_category_enum` | alert, suggestion |
| `notification_status_enum` | new, acknowledged, in_progress, resolved, dismissed |

### Triggers

| Trigger | Table | Purpose |
|---------|-------|---------|
| `trg_update_product_status` | products_pro | Auto-sets status based on stock_quantity changes |
| `trg_line_restock_total` | line_restock_lrs | Auto-calculates total_price = quantity x unit_price |
| `trg_line_order_total` | line_order_lor | Auto-calculates line_total = quantity x unit_price |
| `trg_restock_update_stock_on_received` | restock_res | Adds restock quantities to product stock when status changes to 'received' |

---

## stocks_api (Rust)

### Tech Stack
- **Axum 0.7** - HTTP framework
- **SQLX 0.7** - Async PostgreSQL driver
- **utoipa 4** - OpenAPI/Swagger docs auto-generation
- **jsonwebtoken** - JWT authentication
- **argon2** - Password hashing
- **Refinery** - Database migrations

### Code Structure

```
stocks_api/src/
  bin/
    server.rs         # Main HTTP server, route mounting, middleware
    migrate.rs        # Migration runner binary
    seed.rs           # Dev data seeder binary
  features/
    mod.rs            # Module declarations
    users/            # Auth + user CRUD
    products/         # Product CRUD + 9 KPI endpoints
    suppliers/        # Supplier CRUD
    orders/           # Order CRUD + stats
    stocks/           # Stock monitoring endpoints
    sales/            # Sales analytics endpoints
    restocks/         # Restock CRUD + stats
    kpis/             # Global KPI dashboard endpoints
    alerts/           # Alert lifecycle management (NEW)
    ai_predictions/   # AI model results endpoints (NEW)
  common/
    responses.rs      # SuccessResponse<T>, ErrorResponse
    error_codes.rs    # Constant error code strings
    security.rs       # JWT + password hashing
  openapi.rs          # Swagger/OpenAPI configuration
  lib.rs              # Library root
  migrations/         # SQL migration files (V001-V011)
```

### Feature Module Pattern

Every feature follows the same structure:

```
feature_name/
  mod.rs          # Module exports
  dto.rs          # Request/response structs (Serialize, Deserialize, ToSchema)
  handlers.rs     # HTTP handlers with utoipa annotations
  services.rs     # Database queries (sqlx)
  router.rs       # Axum route definitions
```

### Authentication Flow

1. `POST /auth/login` -> validates credentials -> returns JWT token
2. All protected routes use the `require_auth` middleware layer
3. Middleware extracts and validates JWT from `Authorization: Bearer <token>` header
4. JWT contains: sub (user_id), email, exp (expiration)

---

## ai-service (Python)

### Tech Stack
- **Flask** - HTTP server for manual triggers and health checks
- **APScheduler (BackgroundScheduler)** - Cron job scheduling
- **scikit-learn** - IsolationForest, RandomForest, GradientBoosting, K-Means
- **Prophet** - Facebook's time-series forecasting
- **psycopg2** - PostgreSQL adapter with connection pooling
- **pandas/numpy** - Data manipulation

### Code Structure

```
ai-service/
  main.py               # Flask server (health, status, manual trigger)
  scheduler.py           # APScheduler config, job orchestration
  config/
    settings.py          # Model thresholds and parameters
  models/
    abc_xyz_classifier.py    # Rule-based ABC-XYZ classification
    product_clusterer.py     # K-Means clustering
    supplier_scorer.py       # Weighted scoring system
    demand_forecaster.py     # Prophet time-series forecasting
    price_anomaly_detector.py # Isolation Forest for prices
    sales_anomaly_detector.py # Isolation Forest for sales
    price_suggester.py       # RandomForest/GradientBoosting price optimization
  handlers/
    classification_handler.py
    clustering_handler.py
    supplier_scoring_handler.py
    demand_forecast_handler.py
    price_anomaly_handler.py
    sales_anomaly_handler.py
    price_suggestion_handler.py
  database/
    connection.py        # ThreadedConnectionPool (1-10 connections)
    queries.py           # Read queries (products, prices, sales)
    writers.py           # Insert functions for all AI tables
  requirements.txt
  Dockerfile
```

### Execution Schedule

The AI service runs models in 4 sequential groups:

```
Group 1: Anomaly Detection (parallel)
  - PriceAnomalyHandler
  - SalesAnomalyHandler

Group 2: Product Analysis (parallel)
  - ClassificationHandler
  - ClusteringHandler
  - SupplierScoringHandler

Group 3: Forecasting (sequential)
  - DemandForecastHandler

Group 4: Price Optimization (sequential)
  - PriceSuggestionHandler
```

**When models run:**
1. On startup if `RUN_ON_STARTUP=true` (default)
2. Daily at 2 AM via cron (`CRON_SCHEDULE="0 2 * * *"`)
3. On demand via `POST http://localhost:8001/ai/run`

### Alert Generation

Each handler creates notifications in the `notifications` table when it detects something noteworthy:

| Handler | Creates Alert When | Severity |
|---------|-------------------|----------|
| PriceAnomalyHandler | `is_anomaly == true` | HIGH |
| SalesAnomalyHandler | `is_anomaly == true` | HIGH (score <= -0.8) or MEDIUM |
| ClassificationHandler | ABC-XYZ class changes for Class A products | MEDIUM |
| SupplierScoringHandler | Rating is POOR or UNACCEPTABLE | HIGH or CRITICAL |
| DemandForecastHandler | Stockout <= 7 days (URGENT) or <= 14 days | CRITICAL or HIGH |
| PriceSuggestionHandler | Confidence >= 0.7 | HIGH (>10% change) or MEDIUM |

---

## Seed Data (Development)

The seeder creates realistic dev data for testing:

| Entity | Count | Notes |
|--------|-------|-------|
| Roles | 5 | admin, manager, seller, viewer, user |
| Users | 852 | 1 admin, 1 manager, 850 regular users |
| Suppliers | 20 | Supplier 1-20 |
| Products | 200 | 8 categories, random stock 0-200 |
| Price history | 400-1000 | 2-5 entries per product over 2 years |
| Orders | 5000 | 2-5 lines each, over 2 years |
| Restocks | 500 | Regular, bulk, emergency, seasonal types |

**Login credentials:** `admin@example.com` / `adminpass`

---

## Migration History

| Migration | Purpose |
|-----------|---------|
| V001__init.sql | Core tables (users, roles, products, orders, suppliers) |
| V002__indexes.sql | Performance indexes on line items and products |
| V003__restocks.sql | Restock tables + triggers |
| V004__product_prices.sql | Selling price history table |
| V005__product_restock_prices.sql | Buying price history table |
| V006__remove_unique_productprices.sql | Allow multiple prices per product |
| V007__ai_tables.sql | AI output tables (suggestions, anomalies) |
| V008__notifications.sql | Notifications table + enum |
| V009__demand_forecast_tables.sql | Forecasts, classifications, clusters, supplier scores + views |
| V010__add_buying_price_index.sql | Index on restock price dates |
| V011__alert_management.sql | Alert lifecycle (status enum, nullable product_id, updated_at) |
