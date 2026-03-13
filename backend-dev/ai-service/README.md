# AI Service

Automated AI/ML service for StockS inventory management. Runs scheduled jobs for demand forecasting, product classification, clustering, and supplier scoring.

## Architecture

```
ai-service/
├── models/                 # ML Models
│   ├── demand_forecaster.py    # Prophet-based demand prediction
│   ├── abc_xyz_classifier.py   # Revenue/variability classification
│   ├── product_clusterer.py    # K-Means product segmentation
│   ├── supplier_scorer.py      # Supplier performance scoring
│   ├── price_suggester.py      # Price optimization
│   ├── price_anomaly_detector.py
│   └── sales_anomaly_detector.py
├── handlers/               # Job Handlers
│   ├── base_handler.py         # Base handler class
│   ├── demand_forecast_handler.py
│   ├── classification_handler.py
│   ├── clustering_handler.py
│   ├── supplier_scoring_handler.py
│   └── ...
├── database/               # Database Layer
│   ├── connection.py           # PostgreSQL connection
│   ├── queries.py              # Read queries
│   └── writers.py              # Insert functions
├── config/                 # Configuration
│   └── settings.py             # Model thresholds
├── tests/                  # Unit Tests
│   └── test_models.py
├── scheduler.py            # APScheduler cron jobs
└── main.py                 # Entry point
```

## Models

### 1. Demand Forecaster

**Algorithm**: Facebook Prophet (time series forecasting)

**Features**:
- Weekly and yearly seasonality detection
- 80% confidence intervals
- Model caching (7 days)
- Stock recommendations with urgency levels

**Output**:
- Daily demand predictions (1-90 days)
- Reorder quantity recommendations
- Days until stockout
- Urgency: URGENT / HIGH / MEDIUM / LOW

**Requirements**:
- Minimum 30 days of sales history
- Optimal: 6-12 months of data

---

### 2. ABC-XYZ Classifier

**Algorithm**: Statistical analysis (Pareto principle + Coefficient of Variation)

**ABC Classification** (Revenue-based):
| Class | Products | Revenue | Description |
|-------|----------|---------|-------------|
| A | Top 20% | ~80% | High value |
| B | Next 30% | ~15% | Moderate value |
| C | Remaining 50% | ~5% | Low value |

**XYZ Classification** (Variability-based):
| Class | CV Threshold | Description |
|-------|-------------|-------------|
| X | CV < 0.5 | Predictable demand |
| Y | 0.5 ≤ CV < 1.0 | Moderate fluctuation |
| Z | CV ≥ 1.0 | Unpredictable demand |

**Combined Matrix** (9 categories):
```
      X           Y           Z
A   AX (JIT)    AY (Weekly)  AZ (High Safety)
B   BX (Std)    BY (Moderate) BZ (Monitor)
C   CX (Bulk)   CY (Simple)  CZ (Discontinue?)
```

---

### 3. Product Clusterer

**Algorithm**: K-Means clustering with auto-detected k

**Features Used** (normalized 0-1):
- Revenue score
- Demand variability
- Trend (growing/declining)
- Seasonality
- Order frequency
- Profit margin

**Cluster Names** (auto-assigned):
- Cash Cows (high revenue, stable)
- Rising Stars (high revenue, growing)
- High-Value Volatile (high revenue, variable)
- Steady Performers (medium, stable)
- Unpredictable Low-Value (low, variable)
- Declining Products
- etc.

**Quality Metric**: Silhouette score (0-1, higher is better)

---

### 4. Supplier Scorer

**Algorithm**: Weighted scoring (0-100)

**Scoring Components**:
| Component | Weight | Description |
|-----------|--------|-------------|
| Delivery Performance | 40% | On-time delivery rate |
| Quality Score | 25% | Defect/cancellation rate |
| Lead Time | 20% | Consistency and speed |
| Fulfillment Rate | 15% | Order completion rate |

**Ratings**:
| Score | Rating | Action |
|-------|--------|--------|
| 90+ | EXCELLENT | Preferred supplier |
| 75-89 | GOOD | Reliable |
| 60-74 | ACCEPTABLE | Monitor |
| 40-59 | POOR | Needs improvement |
| <40 | UNACCEPTABLE | Consider replacing |

---

## Database Tables

| Table | Purpose |
|-------|---------|
| `demand_forecasts` | Forecast predictions and recommendations |
| `product_classifications` | ABC-XYZ classification history |
| `product_clusters` | ML clustering results |
| `supplier_scores` | Supplier performance scores |
| `notifications` | Alerts and suggestions |

**Views** (for frontend):
- `v_latest_forecasts` - Latest forecast per product
- `v_latest_classifications` - Latest classification per product
- `v_latest_clusters` - Latest cluster per product
- `v_latest_supplier_scores` - Latest score per supplier
- `v_urgent_restocks` - Products needing immediate restock

---

## Configuration

**Environment Variables**:
```env
DATABASE_URL=postgres://user:pass@db:5432/stocks
CRON_SCHEDULE=0 2 * * *   # Daily at 2 AM
```

**Model Thresholds** (`config/settings.py`):
```python
MODEL_THRESHOLDS = {
    "demand_forecast": {
        "urgent_days": 7,      # Days for URGENT alert
        "high_days": 14,       # Days for HIGH alert
        "min_data_points": 30,
    },
    "abc_xyz_classifier": {
        "lookback_days": 90,
        "min_transactions": 5,
    },
    # ...
}
```

---

## Running

### Docker
```bash
docker-compose up ai-service
```

### Manual
```bash
cd ai-service
pip install -r requirements.txt
python main.py
```

### Run Jobs Immediately (Testing)
```python
from scheduler import run_all_jobs
run_all_jobs()
```

---

## Tests

```bash
cd ai-service
python -m pytest tests/ -v
```

---

## Scheduled Jobs

All jobs run daily at 2 AM (configurable via `CRON_SCHEDULE`):

1. **DemandForecastHandler** - Forecast all products
2. **ClassificationHandler** - ABC-XYZ classification
3. **ClusteringHandler** - ML clustering
4. **SupplierScoringHandler** - Score suppliers
5. **PriceSuggestionHandler** - Price optimization
6. **PriceAnomalyHandler** - Price anomaly detection
7. **SalesAnomalyHandler** - Sales anomaly detection

---

## Notifications

The system creates notifications for:

| Type | Trigger | Severity |
|------|---------|----------|
| `low_stock_warning` | Stockout in <14 days | CRITICAL/HIGH |
| `classification_change` | ABC class changed | MEDIUM |
| `supplier_performance` | Score < 60 | CRITICAL/HIGH |
| `price_suggestion` | Optimization opportunity | MEDIUM |
| `price_anomaly` | Unusual price | HIGH |
| `sales_anomaly` | Unusual sales | HIGH |
