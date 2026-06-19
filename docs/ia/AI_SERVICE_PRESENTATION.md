# AI Service - Technical Presentation

A comprehensive overview of the AI-powered analytics engine that drives intelligent stock management decisions.

---

## Table of Contents

1. [What Is the AI Service?](#1-what-is-the-ai-service)
2. [Technology Stack](#2-technology-stack)
3. [Architecture & Data Flow](#3-architecture--data-flow)
4. [The 7 Models at a Glance](#4-the-7-models-at-a-glance)
5. [Model Deep Dives](#5-model-deep-dives)
   - [5.1 Demand Forecaster (Prophet)](#51-demand-forecaster)
   - [5.2 Price Suggester (Random Forest / Gradient Boosting)](#52-price-suggester)
   - [5.3 Price Anomaly Detector (Isolation Forest)](#53-price-anomaly-detector)
   - [5.4 Sales Anomaly Detector (Isolation Forest)](#54-sales-anomaly-detector)
   - [5.5 ABC-XYZ Classifier (Statistical)](#55-abc-xyz-classifier)
   - [5.6 Product Clusterer (K-Means)](#56-product-clusterer)
   - [5.7 Supplier Scorer (Weighted Scoring)](#57-supplier-scorer)
6. [Training & Model Lifecycle](#6-training--model-lifecycle)
7. [Scheduling & Execution Pipeline](#7-scheduling--execution-pipeline)
8. [Notification & Alert System](#8-notification--alert-system)
9. [API Endpoints](#9-api-endpoints)
10. [Infrastructure & Deployment](#10-infrastructure--deployment)
11. [Configuration Reference](#11-configuration-reference)

---

## 1. What Is the AI Service?

The AI service is a **standalone Python microservice** that runs alongside the main Rust API. Its role is to analyze historical sales, pricing, and supplier data to produce **actionable business intelligence** automatically.

**In simple terms:** it answers questions like:
- *"When will this product run out of stock?"* → Demand Forecaster
- *"Is this product priced correctly?"* → Price Suggester
- *"Is this price or sales pattern abnormal?"* → Anomaly Detectors
- *"Which products are most important to my business?"* → ABC-XYZ Classifier
- *"What hidden product segments exist?"* → Product Clusterer
- *"Which suppliers are reliable?"* → Supplier Scorer

It runs **7 machine learning models** that write their results directly into the shared PostgreSQL database. The Rust API then reads and exposes those results through REST endpoints.

---

## 2. Technology Stack

### Core Technologies

| Technology | Version | Role |
|------------|---------|------|
| **Python** | 3.13 | Runtime |
| **Flask** | latest | Lightweight HTTP server (health checks, manual trigger) |
| **APScheduler** | latest | Background job scheduling (cron-based) |
| **PostgreSQL** | 16 | Shared database for input data and AI results |
| **Docker** | - | Containerized deployment |

### Machine Learning Libraries

| Library | Models Using It | Purpose |
|---------|----------------|---------|
| **Facebook Prophet** | Demand Forecaster | Time-series forecasting with automatic seasonality detection |
| **scikit-learn** | Price Suggester, Price Anomaly, Sales Anomaly, Product Clusterer | Classical ML algorithms (Random Forest, Gradient Boosting, Isolation Forest, K-Means) |
| **pandas** | All models | Data loading, transformation, and aggregation |
| **numpy** | All models | Numerical computing (statistics, normalization) |

### Data Layer

| Library | Purpose |
|---------|---------|
| **psycopg2** | PostgreSQL adapter with thread-safe connection pooling |
| **python-dotenv** | Environment variable management |
| **pickle** | Model serialization and caching to disk |

---

## 3. Architecture & Data Flow

### System Position

```
                    +--------------------+
                    |    PostgreSQL 16    |
                    |    (shared DB)      |
                    +---------+----------+
                              |
              +---------------+---------------+
              |                               |
     +--------v--------+           +----------v-----------+
     |   Rust API       |           |    AI Service         |
     |   (Axum 0.7)     |           |    (Python/Flask)     |
     |   Port 8090      |           |    Port 8001          |
     +-----------------+           +----------------------+
     | READS AI results |           | WRITES predictions    |
     | Serves frontend  |           | 7 ML models           |
     +-----------------+           | Cron scheduler        |
              ^                     +----------------------+
              |
     +--------+--------+
     |    Frontend      |
     +-----------------+
```

### Data Flow for a Single Model Run

```
  1. READ                    2. PROCESS                  3. WRITE
+------------+          +------------------+          +----------------+
| PostgreSQL |  ------> | Feature          |  ------> | PostgreSQL     |
| Input      |  query   | Engineering      |  insert  | Output         |
| Tables     |          |       +          |          | Tables         |
+------------+          | Model Prediction |          +----------------+
                        +------------------+          | + Notifications |
                                                      +----------------+
```

**Input tables read:** `products_pro`, `productprices_prp`, `line_order_lor`, `order_ord`, `supplier_sup`, `restock_res`

**Output tables written:** `demand_forecasts`, `product_classifications`, `product_clusters`, `supplier_scores`, `price_suggestions`, `price_anomalies`, `sales_anomalies`, `notifications`

### Internal Architecture

```
ai-service/
  main.py                    # Flask server entry point
  scheduler.py               # Job orchestration & parallel execution
  config/
    settings.py              # Thresholds, hyperparameters
  models/                    # ML algorithms (pure logic, no DB access)
    demand_forecaster.py
    price_suggester.py
    price_anomaly_detector.py
    sales_anomaly_detector.py
    abc_xyz_classifier.py
    product_clusterer.py
    supplier_scorer.py
  handlers/                  # Orchestrators (fetch data -> run model -> write results)
    demand_forecast_handler.py
    price_suggestion_handler.py
    price_anomaly_handler.py
    sales_anomaly_handler.py
    classification_handler.py
    clustering_handler.py
    supplier_scoring_handler.py
  database/
    connection.py            # Connection pooling (1-10 threads)
    queries.py               # Read queries
    writers.py               # Batch insert functions
```

**Separation of concerns:** Models contain only algorithm logic. Handlers are responsible for fetching data from the database, calling the model, and writing results back. This makes models independently testable.

---

## 4. The 7 Models at a Glance

| # | Model | Algorithm | Category | What It Answers |
|---|-------|-----------|----------|-----------------|
| 1 | **Demand Forecaster** | Facebook Prophet | Supervised (time-series) | *"When will this product run out of stock?"* |
| 2 | **Price Suggester** | Random Forest / Gradient Boosting | Supervised (regression) | *"What should this product's price be?"* |
| 3 | **Price Anomaly Detector** | Isolation Forest | Unsupervised (anomaly) | *"Is this price abnormal?"* |
| 4 | **Sales Anomaly Detector** | Isolation Forest | Unsupervised (anomaly) | *"Is this sales volume abnormal?"* |
| 5 | **ABC-XYZ Classifier** | Pareto + Coefficient of Variation | Statistical (rule-based) | *"How important and predictable is this product?"* |
| 6 | **Product Clusterer** | K-Means | Unsupervised (clustering) | *"What hidden product segments exist?"* |
| 7 | **Supplier Scorer** | Weighted multi-criteria scoring | Statistical (rule-based) | *"How reliable is this supplier?"* |

### Model Categories Explained

- **Supervised learning** (models 1-2): Trained on historical data where the "correct answer" is known. Given past patterns, predict future values.
- **Unsupervised learning** (models 3-4, 6): No labeled data needed. The algorithm discovers structure (anomalies, clusters) on its own.
- **Statistical / Rule-based** (models 5, 7): Use well-established business rules and formulas rather than machine learning algorithms. Deterministic and interpretable.

---

## 5. Model Deep Dives

---

### 5.1 Demand Forecaster

> **Goal:** Predict future product demand to prevent stockouts and optimize reorder timing.

**Algorithm:** Facebook Prophet

**Why Prophet?**
Prophet is a time-series forecasting library developed by Meta. It was chosen because it:
- Handles missing data gracefully (fills gaps with zero sales)
- Automatically detects weekly and yearly seasonality
- Detects trend changes (sudden shifts in demand patterns)
- Works well with daily granularity data
- Requires minimal hyperparameter tuning

#### How It's Trained

```
Step 1: Load Data
  - Pull up to 2 years of daily sales per product from line_order_lor + order_ord
  - Fill missing dates with 0 (no sale = 0 units)
  - Format as Prophet expects: columns ["ds" (date), "y" (quantity)]

Step 2: Configure & Fit
  - Seasonality mode: additive
  - Weekly seasonality: enabled
  - Yearly seasonality: enabled
  - Changepoint prior scale: 0.05 (controls trend flexibility)
  - Fit the model on the historical daily time series

Step 3: Forecast
  - Generate predictions for the next 30 days (configurable)
  - Each day gets: predicted value + 80% confidence interval (lower/upper bound)

Step 4: Business Logic
  - total_predicted_demand = sum of all predicted daily values
  - avg_daily_demand = total / forecast_days
  - recommended_stock = sum(upper_bounds) x 1.2 (20% safety buffer)
  - reorder_quantity = recommended_stock - current_stock
  - days_until_stockout = current_stock / avg_daily_demand
```

#### Urgency Classification

| Urgency | Days Until Stockout | Recommended Action |
|---------|--------------------|--------------------|
| **URGENT** | <= 7 days | Reorder immediately |
| **HIGH** | <= 14 days | Place order soon |
| **MEDIUM** | <= 30 days | Schedule reorder |
| **LOW** | > 30 days | No action needed |

#### Accuracy Metrics

| Metric | What It Measures | Good Value |
|--------|-----------------|------------|
| **MAPE** (Mean Absolute Percentage Error) | Average % error per prediction | < 20% |
| **RMSE** (Root Mean Squared Error) | Absolute error in units | Context-dependent |

#### Requirements
- Minimum **30 data points** per product to train a meaningful model
- Products with fewer data points are skipped

#### Output → `demand_forecasts` table

| Column | Description |
|--------|-------------|
| product_id | The product being forecasted |
| forecast_date | When the forecast was generated |
| forecast_days | Horizon (default: 30) |
| total_predicted_demand | Total units expected to sell |
| avg_daily_demand | Average units per day |
| current_stock | Stock level at forecast time |
| recommended_stock | How much stock to have |
| reorder_quantity | How many units to order |
| days_until_stockout | Estimated days before stock = 0 |
| urgency | URGENT / HIGH / MEDIUM / LOW |
| mape | Forecast accuracy (%) |
| rmse | Forecast accuracy (units) |

---

### 5.2 Price Suggester

> **Goal:** Recommend optimal selling prices based on historical pricing trends and market positioning.

**Algorithm:** Auto-selected between **Random Forest Regressor** and **Gradient Boosting Regressor** via cross-validation.

**Why this approach?**
- Ensemble methods (Random Forest, Gradient Boosting) handle non-linear price relationships well
- Auto-selection ensures the best-performing algorithm is always used
- Cross-validation prevents overfitting to training data

#### How It's Trained

```
Step 1: Build Training Data
  - For each product with >= 3 price entries:
    - Create sliding window samples: use prices[0:i] to predict prices[i]
    - This teaches the model: "given these historical features, what was the next price?"

Step 2: Feature Engineering (8 features per sample)
  - current_price:   latest known price
  - mean_price:      average of historical prices
  - std_price:       price volatility (standard deviation)
  - min_price:       lowest historical price
  - max_price:       highest historical price
  - price_trend:     direction = (last - first) / first
  - volatility:      coefficient of variation = std / mean
  - price_position:  where current sits in range = (current - min) / (max - min)

Step 3: Model Selection
  - Train Random Forest (max_depth=10, n_estimators=100)
  - Train Gradient Boosting (max_depth=5, n_estimators=100)
  - Run 5-fold cross-validation on both
  - Pick the model with the higher R² score

Step 4: Prediction
  - For each product, predict the optimal price
  - Safety cap: suggested price is clamped to 50%-150% of current price
  - Generate a human-readable reason ("Consider increasing by X%...")

Step 5: Confidence Scoring
  - confidence = 0.4 x data_confidence + 0.6 x model_R²
  - data_confidence = min(1.0, num_price_entries / 10)
  - Only suggestions with confidence >= 0.7 generate alerts
```

#### Output → `price_suggestions` table

| Column | Description |
|--------|-------------|
| product_id | The product |
| current_price | Current selling price |
| suggested_price | Recommended optimal price |
| reason | Human-readable explanation |
| confidence | 0.0 to 1.0 (higher = more reliable) |

#### Example Reasons Generated
- *"Current price is optimal"* (change < 2%)
- *"Consider increasing price by 5.2% based on market trends"*
- *"Consider decreasing price by 3.1% to improve competitiveness"*

---

### 5.3 Price Anomaly Detector

> **Goal:** Flag products whose current price deviates significantly from their expected price, catching pricing errors, data entry mistakes, or unusual market shifts.

**Algorithm:** Isolation Forest (scikit-learn)

**Why Isolation Forest?**
Isolation Forest is ideal for anomaly detection because:
- It doesn't require labeled "anomaly" examples (unsupervised)
- It works by randomly partitioning data — anomalies are easier to isolate (fewer splits needed), so they naturally get higher anomaly scores
- It scales well and is fast to train
- It handles mixed feature distributions

#### How It's Trained

```
Step 1: Calculate Per-Product Statistics
  - For each product, compute a rolling average of the last 30 price entries
  - Calculate mean, standard deviation, min, max per product
  - This gives each product its OWN expected price (not a global average)

Step 2: Feature Engineering (per price record)
  - price:         the actual selling price
  - buying_price:  the supplier cost
  - z_score:       (price - product_mean) / product_std
                   → how many standard deviations away from expected
  - margin_ratio:  (price - buying_price) / price
                   → the profit margin percentage

Step 3: Train Isolation Forest
  - contamination = 0.1 (expect ~10% of records to be anomalous)
  - The model learns the "normal" distribution of the 4 features
  - Records that are hard to group with others = anomalies

Step 4: Score Each Record
  - anomaly_score: negative values indicate anomalies (more negative = more anomalous)
  - is_anomaly: True if the model labels the record as -1
  - expected_price: the per-product rolling average
```

#### Output → `price_anomalies` table

| Column | Description |
|--------|-------------|
| product_id | The product |
| current_price | Actual price flagged |
| expected_price | Per-product rolling average |
| anomaly_score | -1 to 1 (negative = anomalous) |
| is_anomaly | Boolean flag |

---

### 5.4 Sales Anomaly Detector

> **Goal:** Flag products with unusual sales volumes — either unexpectedly high (bulk buy, viral trend, potential fraud) or unexpectedly low (distribution issue, competitor action).

**Algorithm:** Isolation Forest (scikit-learn)

Same principle as the Price Anomaly Detector, applied to sales volumes instead.

#### How It's Trained

```
Step 1: Aggregate Sales Data
  - For each product: total units sold, number of distinct orders
  - Calculate per-product expected values (rolling averages)

Step 2: Feature Engineering (per product)
  - volume:           total units sold
  - order_count:      number of distinct orders
  - z_score:          volume deviation from expected
  - avg_order_size:   volume / order_count
  - orders_z_score:   order count deviation from expected

Step 3: Train & Score
  - Same Isolation Forest approach (contamination = 0.1)
  - Products with unusual volume patterns are flagged
```

#### Severity Classification

| Anomaly Score | Severity | Interpretation |
|---------------|----------|----------------|
| <= -0.8 | **CRITICAL** | Extreme deviation — investigate immediately |
| <= -0.5 | **MEDIUM** | Notable deviation — worth reviewing |
| > -0.5 | Normal | Within expected range |

#### Output → `sales_anomalies` table

| Column | Description |
|--------|-------------|
| product_id | The product |
| sales_volume | Actual sales volume |
| expected_sales | Per-product expected volume |
| anomaly_score | -1 to 1 (negative = anomalous) |
| is_anomaly | Boolean flag |

---

### 5.5 ABC-XYZ Classifier

> **Goal:** Classify every product into a 2-letter code (e.g., AX, BY, CZ) that tells you how important it is and how predictable its demand is. This directly drives inventory strategy.

**Algorithm:** Rule-based (Pareto principle + Coefficient of Variation)

This is not a machine learning model — it uses well-established supply chain management formulas that are deterministic and fully interpretable.

#### How It Works

**ABC Analysis (Revenue Importance — Pareto Principle)**

Products are sorted by total revenue contribution, then classified:

| Class | Revenue Share | Meaning |
|-------|--------------|---------|
| **A** | Top ~80% of cumulative revenue | Most important products |
| **B** | Next ~15% | Moderately important |
| **C** | Remaining ~5% | Low-value products |

This follows the Pareto principle: roughly 20% of products generate 80% of revenue.

**XYZ Analysis (Demand Predictability — Coefficient of Variation)**

The coefficient of variation (CV = standard deviation / mean) of order quantities is calculated:

| Class | CV Range | Meaning |
|-------|----------|---------|
| **X** | CV < 0.5 | Highly predictable, steady demand |
| **Y** | 0.5 <= CV < 1.0 | Moderate fluctuation |
| **Z** | CV >= 1.0 | Unpredictable, erratic demand |

**The 9 Combined Classes**

| Class | Profile | Recommended Strategy |
|-------|---------|---------------------|
| **AX** | High value, predictable | Just-In-Time with safety stock. Highest priority. |
| **AY** | High value, some variability | Regular review with moderate safety stock |
| **AZ** | High value, unpredictable | Large safety stock, frequent monitoring |
| **BX** | Medium value, predictable | Periodic ordering, standard safety stock |
| **BY** | Medium value, some variability | Regular review cycles |
| **BZ** | Medium value, unpredictable | Increase safety stock, review frequently |
| **CX** | Low value, predictable | Automated ordering, minimal attention |
| **CY** | Low value, some variability | Simple min-max system |
| **CZ** | Low value, unpredictable | Keep minimal stock, order on demand |

#### Requirements
- **Lookback period:** 90 days of order history
- **Minimum transactions:** 5 per product to qualify

#### Output → `product_classifications` table

| Column | Description |
|--------|-------------|
| product_id | The product |
| abc_class | A, B, or C |
| xyz_class | X, Y, or Z |
| combined_class | AX, AY, AZ, BX, BY, BZ, CX, CY, CZ |
| total_revenue | Revenue in lookback period |
| revenue_contribution_pct | % of total revenue |
| total_units_sold | Units sold in lookback period |
| coefficient_of_variation | Demand variability metric |
| strategy | Recommended inventory strategy text |
| priority | Numeric priority ranking |

---

### 5.6 Product Clusterer

> **Goal:** Discover hidden product segments by grouping similar products together based on sales behavior, margins, and trends. Useful for portfolio analysis and targeted strategies.

**Algorithm:** K-Means Clustering (scikit-learn)

**Why K-Means?**
- Simple, fast, and well-understood clustering algorithm
- Works well with normalized numerical features
- Combined with silhouette scoring for automatic optimal cluster selection

#### How It's Trained

```
Step 1: Feature Engineering (6 normalized scores per product, scaled 0 to 1)
  - revenue_score:      product revenue / max revenue across all products
  - variability_score:  coefficient of variation / max CV
  - trend_score:        50% activity ratio + 50% recency score
  - seasonality_score:  variability x revenue (interaction term)
  - frequency_score:    order count / max orders
  - margin_score:       (selling_price - buying_price) / selling_price

Step 2: Optimal K Selection
  - Test cluster counts from K=2 to K=10
  - For each K, run K-Means and compute silhouette score
  - Silhouette score measures how well-separated clusters are (-1 to 1, higher = better)
  - Pick the K with the highest silhouette score

Step 3: Final Clustering
  - Run K-Means with the optimal K
  - Each product is assigned to its nearest centroid

Step 4: Cluster Naming
  - Each cluster gets a descriptive name based on its centroid characteristics:
```

#### Auto-Generated Cluster Names

| Name | Characteristics |
|------|----------------|
| **Cash Cows** | High revenue, low variability |
| **Rising Stars** | High trend score, growing demand |
| **High-Value Volatile** | High revenue, high variability |
| **Steady Performers** | Balanced, medium scores |
| **Unpredictable Low-Value** | Low revenue, high variability |
| **Reliable Long Tail** | Low revenue, stable demand |

#### Output → `product_clusters` table

| Column | Description |
|--------|-------------|
| product_id | The product |
| cluster_id | Numeric cluster identifier |
| cluster_name | Descriptive name |
| revenue_score | 0-1 normalized revenue |
| variability_score | 0-1 normalized variability |
| trend_score | 0-1 normalized trend |
| seasonality_score | 0-1 normalized seasonality |
| frequency_score | 0-1 normalized order frequency |
| margin_score | 0-1 normalized profit margin |
| distance_to_centroid | How far from cluster center |
| n_clusters | Total number of clusters found |
| silhouette_score | Quality metric for the clustering |

---

### 5.7 Supplier Scorer

> **Goal:** Rate each supplier on a 0-100 scale based on delivery performance, quality, lead times, and fulfillment. Assign a human-readable rating from EXCELLENT to UNACCEPTABLE.

**Algorithm:** Weighted multi-criteria scoring

This is a deterministic scoring system, not a machine learning model. It uses a weighted formula across 4 dimensions.

#### Scoring Formula

```
overall_score = (delivery x 0.40) + (quality x 0.25) + (lead_time x 0.20) + (fulfillment x 0.15)
```

| Component | Weight | How It's Calculated |
|-----------|--------|-------------------|
| **Delivery Performance** | 40% | % of restocks with status = "received" |
| **Quality Score** | 25% | 100 - (cancellation_rate x 100) |
| **Lead Time Score** | 20% | Speed (60%): 100 if <=3 days, down to 50 at >=14 days. Consistency (40%): based on CV of lead times |
| **Fulfillment Rate** | 15% | Order completion rate |

#### Rating Scale

| Rating | Score | Interpretation |
|--------|-------|----------------|
| **EXCELLENT** | >= 90 | Preferred supplier, prioritize orders |
| **GOOD** | 75 - 89 | Reliable, maintain relationship |
| **ACCEPTABLE** | 60 - 74 | Monitor closely, address issues |
| **POOR** | 40 - 59 | Needs improvement plan |
| **UNACCEPTABLE** | < 40 | Consider replacing |

#### Requirements
- **Lookback period:** 90 days of restock history
- **Minimum restocks:** 3 per supplier to qualify

#### Output → `supplier_scores` table

| Column | Description |
|--------|-------------|
| supplier_id | The supplier |
| overall_score | 0-100 composite score |
| delivery_score | 0-100 delivery component |
| quality_score | 0-100 quality component |
| lead_time_score | 0-100 lead time component |
| fulfillment_score | 0-100 fulfillment component |
| rating | EXCELLENT / GOOD / ACCEPTABLE / POOR / UNACCEPTABLE |
| total_restocks | Number of restocks in period |

---

## 6. Training & Model Lifecycle

### How Models Are Trained

All models follow a common lifecycle:

```
                   +------------------+
                   |  Scheduler Fires  |
                   |  (cron or manual) |
                   +--------+---------+
                            |
                   +--------v---------+
                   |  Handler.run()    |
                   +--------+---------+
                            |
              +-------------+-------------+
              |                           |
     +--------v--------+        +--------v--------+
     | Fetch data from  |        | Check model     |
     | PostgreSQL       |        | cache (7 days)  |
     +--------+--------+        +--------+--------+
              |                           |
              |                  +--------v--------+
              |                  | Cache valid?     |
              |                  +---+----------+---+
              |                  Yes |          | No
              |              +---v--+    +-----v------+
              |              | Load |    | Train new  |
              |              | from |    | model      |
              |              | disk |    | + save to  |
              |              +---+--+    | disk       |
              |                  |       +-----+------+
              +------------------+-------------+
                            |
                   +--------v---------+
                   |  Run predictions  |
                   +--------+---------+
                            |
                   +--------v---------+
                   |  Write results    |
                   |  (batch insert)   |
                   +--------+---------+
                            |
                   +--------v---------+
                   |  Generate alerts  |
                   +------------------+
```

### Model Caching

Models that require training (Prophet, Isolation Forest, Random Forest/Gradient Boosting) are cached to disk using Python's `pickle` serialization. This avoids retraining on every run.

| Model | Cache File | Cache Duration |
|-------|-----------|---------------|
| Demand Forecaster | `forecast_product_{id}.pkl` (one per product) | 7 days |
| Price Suggester | `price_suggester_model.pkl` | 7 days |
| Price Anomaly Detector | `price_anomaly_model.pkl` | 7 days |
| Sales Anomaly Detector | `sales_anomaly_model.pkl` | 7 days |

**Cache invalidation:** Age-based. When a cached model is older than 7 days, it is discarded and retrained from scratch.

**Models without caching:** ABC-XYZ Classifier, Product Clusterer, and Supplier Scorer are rule-based or fast enough to recompute every time. They don't use caching.

### Training Data Sources

| Model | Data Source | Historical Depth |
|-------|-----------|-----------------|
| Demand Forecaster | Daily sales quantities | Up to 2 years |
| Price Suggester | Price history per product | All available history |
| Price Anomaly Detector | Price history + buying prices | All + 30-day rolling window |
| Sales Anomaly Detector | Aggregated sales volumes | All available |
| ABC-XYZ Classifier | Order line items | Last 90 days |
| Product Clusterer | Sales + prices + orders | Last 90 days |
| Supplier Scorer | Restock records | Last 90 days |

---

## 7. Scheduling & Execution Pipeline

### When Models Run

| Trigger | Description |
|---------|-------------|
| **Startup** | All models run when the service starts (if `RUN_ON_STARTUP=true`) |
| **Cron** | Daily at 2:00 AM by default (configurable via `CRON_SCHEDULE`) |
| **Manual** | On demand via `POST /ai/run` |

### Execution Groups

Models are organized into 4 groups. Groups run **sequentially** (one after another). Within a group, handlers can run **in parallel**.

```
Group 1: Anomaly Detection ── parallel ──> PriceAnomalyHandler
                                           SalesAnomalyHandler
                                           (~5 seconds)
         │
         v
Group 2: Product Analysis ─── parallel ──> ClassificationHandler
                                           ClusteringHandler
                                           SupplierScoringHandler
                                           (~10 seconds)
         │
         v
Group 3: Forecasting ──────── sequential > DemandForecastHandler
                                           (~90 seconds, trains Prophet per product)
         │
         v
Group 4: Price Optimization ─ sequential > PriceSuggestionHandler
                                           (~15 seconds)

Total typical runtime: ~2 minutes
```

**Why this order?** Anomaly detection runs first to flag data quality issues. Product analysis (classification, clustering) runs next to establish product segments. Demand forecasting uses the most compute and runs alone. Price suggestions run last because they can benefit from up-to-date analysis.

### Parallel Execution

- Controlled by `PARALLEL_EXECUTION` env var (default: `true`)
- Uses Python's `ThreadPoolExecutor` with configurable max workers (default: 4)
- A `_run_lock` prevents concurrent full runs from overlapping

### Startup Behavior

On startup, the service:
1. Checks database health
2. Waits for seed data to be stable (up to 300 seconds)
   - Monitors `products_pro` and `order_ord` row counts
   - Requires counts to be non-zero AND unchanged between checks
3. Runs all 4 groups in sequence

---

## 8. Notification & Alert System

Every model can generate notifications when it detects something noteworthy. Notifications are written to the `notifications` table and consumed by the Rust API's alert management system.

### Alert Generation by Model

| Model | Generates Alert When | Severity |
|-------|---------------------|----------|
| **Demand Forecaster** | Stockout <= 7 days | CRITICAL |
| **Demand Forecaster** | Stockout <= 14 days | HIGH |
| **Price Anomaly Detector** | `is_anomaly == true` | HIGH |
| **Sales Anomaly Detector** | Score <= -0.8 | HIGH |
| **Sales Anomaly Detector** | Score <= -0.5 | MEDIUM |
| **Supplier Scorer** | Rating = UNACCEPTABLE | CRITICAL |
| **Supplier Scorer** | Rating = POOR | HIGH |
| **ABC-XYZ Classifier** | Class A product changes class | MEDIUM |
| **Price Suggester** | Confidence >= 0.7, change > 10% | HIGH |
| **Price Suggester** | Confidence >= 0.7, change <= 10% | MEDIUM |

### Notification Structure

| Field | Description |
|-------|-------------|
| product_id | Related product (nullable for supplier alerts) |
| model_type | Which model generated it |
| category | `alert` or `suggestion` |
| notification_type | Specific type (e.g., "stockout_warning") |
| severity | CRITICAL / HIGH / MEDIUM / LOW |
| message | Human-readable description |
| action_recommended | Suggested action to take |
| related_result_id | Link to the source prediction record |

### Alert Lifecycle

Notifications support a full lifecycle managed through the Rust API:

```
NEW  →  ACKNOWLEDGED  →  IN_PROGRESS  →  RESOLVED
                                      →  DISMISSED
```

---

## 9. API Endpoints

The AI service exposes 3 HTTP endpoints via Flask on port **8001**:

### `GET /ai/health`
Check service and database connectivity.

**Response:**
```json
{
  "status": "healthy",
  "service": "ai-service",
  "database": {
    "status": "connected",
    "version": "PostgreSQL 16.x"
  }
}
```

### `GET /ai/status`
Get metrics from the latest model run.

**Response:**
```json
{
  "last_run": {
    "run_started": "2026-02-13T02:00:00",
    "duration_seconds": 120.5,
    "total_handlers": 7,
    "successful": 7,
    "failed": 0,
    "groups": [
      { "name": "Anomaly Detection", "duration": 5.2, "status": "success" },
      { "name": "Product Analysis", "duration": 10.8, "status": "success" },
      { "name": "Forecasting", "duration": 89.3, "status": "success" },
      { "name": "Price Optimization", "duration": 15.2, "status": "success" }
    ]
  },
  "is_running": false
}
```

### `POST /ai/run`
Manually trigger a full model run. Returns immediately (runs in background).

**Response:** `202 Accepted`
```json
{
  "message": "AI jobs triggered",
  "status": "running"
}
```

> **Note:** The frontend should **not** call these endpoints directly. All AI results are exposed through the Rust API (port 8090) with proper authentication, pagination, and filtering.

---

## 10. Infrastructure & Deployment

### Docker Configuration

```dockerfile
FROM python:3.13-slim
WORKDIR /app
COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt
COPY . .
CMD ["python", "main.py"]
```

### Docker Compose Integration

The AI service is orchestrated alongside the Rust API:

```
Startup order:  db (healthy) → migrate → seed → web + ai-service (parallel)
```

- The AI service waits for seed data to be present and stable before running models
- Model cache files are stored in `/app/saved_models/` inside the container
- Database connection via the shared Docker network (`backend_default`)

### Database Connection

| Setting | Value |
|---------|-------|
| Adapter | psycopg2 (thread-safe) |
| Pool type | ThreadedConnectionPool |
| Min connections | 1 |
| Max connections | 10 |
| Retry logic | 3 retries, 1s delay |
| Health check | `SELECT version()` |

---

## 11. Configuration Reference

All model thresholds and hyperparameters are centralized in `ai-service/config/settings.py`:

```python
MODEL_THRESHOLDS = {
    "price_suggester": {
        "min_confidence": 0.7,         # Minimum confidence to generate alerts
    },
    "price_anomaly": {
        "anomaly_score_threshold": -0.5,  # Score below this = anomaly
    },
    "sales_anomaly": {
        "anomaly_score_threshold": -0.5,
        "high_severity_threshold": -0.8,  # Score below this = HIGH severity
    },
    "demand_forecast": {
        "urgent_days": 7,              # Days until stockout for URGENT
        "high_days": 14,               # Days until stockout for HIGH
        "min_data_points": 30,         # Minimum daily records to train
        "forecast_horizon": 30,        # Days to forecast ahead
    },
    "abc_xyz_classifier": {
        "lookback_days": 90,           # Historical period to analyze
        "min_transactions": 5,         # Minimum orders to classify
    },
    "product_clusterer": {
        "lookback_days": 90,
        "min_transactions": 5,
        "max_clusters": 10,            # Upper bound for K search
    },
    "supplier_scorer": {
        "lookback_days": 90,
        "min_restocks": 3,             # Minimum restocks to score
        "poor_threshold": 60,          # Below this = POOR rating
    },
}
```

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `DATABASE_URL` | `postgres://user:pass@db:5432/stocks` | PostgreSQL connection string |
| `AI_SERVICE_PORT` | `8001` | Flask server port |
| `CRON_SCHEDULE` | `0 2 * * *` | Cron expression for scheduled runs |
| `RUN_ON_STARTUP` | `true` | Run all models on service start |
| `PARALLEL_EXECUTION` | `true` | Enable parallel handler execution |
| `MAX_WORKERS` | `4` | Maximum parallel threads |

---

## Typical Run Output (Seed Data)

A full run with the development seed dataset produces approximately:

| Model | Records Generated |
|-------|------------------|
| Price Anomalies | ~73 anomalies detected |
| Sales Anomalies | ~20 anomalies detected |
| Classifications | ~145 products classified |
| Clusters | ~482 products in 2-6 clusters |
| Supplier Scores | 20 suppliers scored |
| Demand Forecasts | Forecasts for eligible products |
| Price Suggestions | ~659 suggestions generated |
| **Notifications** | **~115 alerts/suggestions created** |
