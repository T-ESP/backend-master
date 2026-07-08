# AI Service Improvements

This document details the problems identified in the AI service and the improvements made to address them.

## Overview

The AI service underwent a comprehensive review and enhancement. This document covers:
1. Problems identified in the original codebase
2. Improvements implemented
3. Testing results

---

## 1. Database Layer Improvements

### Problem: No Connection Pooling
**Before:** Each handler created a new database connection and closed it after use.
```python
def get_connection():
    return psycopg2.connect(DATABASE_URL)  # New connection every time
```

**Issues:**
- High connection overhead
- No retry logic for transient failures
- No health check capability

### Solution: Connection Pool with Retry Logic
**File:** `database/connection.py`

**Improvements:**
- Thread-safe connection pool (1-10 connections)
- Automatic retry on connection failures (3 retries with 1s delay)
- Health check endpoint for monitoring
- Context manager for automatic connection return

```python
@contextmanager
def get_db_connection():
    conn = get_connection()
    try:
        yield conn
    finally:
        return_connection(conn)
```

---

## 2. Batch Database Inserts

### Problem: Individual Inserts for Each Prediction
**Before:** Each prediction resulted in a separate database transaction.
```python
for prediction in predictions:
    insert_price_anomaly(conn, prediction)  # N separate transactions
```

**Issues:**
- Poor performance for large datasets
- High transaction overhead

### Solution: Batch Insert Functions
**File:** `database/writers.py`

**New Functions:**
- `insert_price_anomalies_batch()`
- `insert_sales_anomalies_batch()`
- `insert_price_suggestions_batch()`
- `insert_notifications_batch()`
- `insert_demand_forecasts_batch()`
- `insert_classifications_batch()`
- `insert_cluster_results_batch()`
- `insert_supplier_scores_batch()`

**Benefits:**
- Single transaction for all inserts
- Automatic fallback to individual inserts on failure
- Comprehensive logging

---

## 3. Model Caching for Friend's Models

### Problem: Models Retrained on Every Run
**Before:** The anomaly detectors and price suggester trained a new model for every execution.
```python
class PriceAnomalyDetector:
    def predict(self, price_history):
        model = IsolationForest(contamination=0.1)
        model.fit(X)  # Retrained every time!
```

**Issues:**
- Wasted computation
- Inconsistent predictions between runs

### Solution: Model Persistence with Cache
**Files:**
- `models/price_anomaly_detector.py`
- `models/sales_anomaly_detector.py`
- `models/price_suggester.py`

**Features:**
- Models cached to disk (7-day default)
- Automatic cache invalidation
- Cache clearing method

```python
def _get_or_train_model(self, X):
    if self.model_path.exists():
        model_age = (datetime.now() - datetime.fromtimestamp(self.model_path.stat().st_mtime)).days
        if model_age < self.cache_days:
            return pickle.load(open(self.model_path, 'rb'))['model']
    # Train new model...
```

---

## 4. Improved Model Algorithms

### Problem: Global Expected Values
**Before:** The anomaly detectors used global mean as "expected" value.
```python
"expected_price": float(np.mean(X[:, 0]))  # Same expected for all products!
```

**Issues:**
- Product with price $10 and product with price $1000 had the same expected
- Many false positives/negatives

### Solution: Per-Product Statistics
**Improvements:**
- Rolling window average for each product
- Per-product standard deviation
- Z-score based anomaly features

```python
def _calculate_product_stats(self, price_history):
    for pid, prices in product_prices.items():
        recent_prices = prices[-self.rolling_window:]
        self.product_stats[pid] = {
            "mean": float(np.mean(recent_prices)),
            "std": float(np.std(recent_prices)),
        }
```

### Price Suggester Enhancements:
- Model selection (Random Forest vs Gradient Boosting)
- Cross-validation for model quality
- 8 engineered features (trend, volatility, price position, etc.)
- Confidence score based on data quality and model performance

---

## 5. Error Handling in Handlers

### Problem: No Individual Error Handling
**Before:** One bad prediction killed the entire batch.
```python
for prediction in predictions:
    result_id = insert_price_anomaly(conn, prediction)  # Exception = all stop
```

### Solution: Robust Error Handling
**Files:**
- `handlers/price_anomaly_handler.py`
- `handlers/sales_anomaly_handler.py`
- `handlers/price_suggestion_handler.py`

**Features:**
- Try-except around batch operations
- Fallback to individual inserts on batch failure
- Per-prediction error logging
- Proper connection return to pool

```python
try:
    result_ids = insert_price_anomalies_batch(conn, predictions)
except Exception as e:
    self.logger.error(f"Batch insert failed: {e}")
    result_ids = self._insert_individually(conn, predictions)
```

---

## 6. Parallel Handler Execution

### Problem: Sequential Execution Only
**Before:** All handlers ran one after another.
```python
for handler in HANDLERS:
    handler.run()  # Blocks until complete
```

**Issues:**
- Unnecessarily long total runtime
- Independent handlers could run in parallel

### Solution: Parallel Execution with Handler Groups
**File:** `scheduler.py`

**Features:**
- Handlers organized into groups
- Parallel execution within groups (where safe)
- Sequential execution between groups
- Configurable via environment variables

**Handler Groups:**
1. **Anomaly Detection** (parallel): Price & Sales anomaly
2. **Product Analysis** (parallel): Classification, Clustering, Supplier Scoring
3. **Forecasting** (sequential): Demand Forecast
4. **Price Optimization** (sequential): Price Suggestions

**Configuration:**
```bash
PARALLEL_EXECUTION=true
MAX_WORKERS=4
```

---

## 7. Health Check and Metrics

### Problem: No Monitoring Capabilities
**Before:** No way to check service health or track performance.

### Solution: Comprehensive Monitoring
**Files:**
- `database/connection.py` - Health check
- `scheduler.py` - Metrics collection

**Health Check:**
```python
def check_database_health() -> dict:
    return {
        "status": "healthy",
        "database": "connected",
        "version": "PostgreSQL 16.x"
    }
```

**Metrics:**
```python
{
    "run_started": "2024-01-30T14:00:00",
    "duration_seconds": 45.5,
    "total_handlers": 7,
    "successful": 7,
    "failed": 0,
    "groups": [...],
    "handlers": [...]
}
```

---

## 8. Comprehensive Tests

### Problem: Missing Tests for Friend's Models
**Before:** Only 4 models had unit tests.

### Solution: Added Tests for All Models
**File:** `tests/test_models.py`

**New Test Classes:**
- `TestPriceAnomalyDetector`
  - Minimum data requirement
  - Basic anomaly detection
  - Per-product expected price
  - Cache clearing

- `TestSalesAnomalyDetector`
  - Minimum data requirement
  - Basic anomaly detection
  - Cache clearing

- `TestPriceSuggester`
  - No price history handling
  - Insufficient training data
  - Price suggestion generation
  - Confidence calculation
  - Cache clearing

**Total Tests:** 27 (was 14)

---

## Summary of Changes

| Component | Before | After |
|-----------|--------|-------|
| **Connection Management** | New connection per request | Connection pooling (1-10) |
| **Database Inserts** | Individual transactions | Batch inserts with fallback |
| **Model Caching** | None for friend's models | 7-day cache for all models |
| **Expected Values** | Global mean | Per-product rolling average |
| **Error Handling** | Batch fails on single error | Individual error handling |
| **Handler Execution** | Sequential only | Parallel + Sequential groups |
| **Monitoring** | None | Health check + Metrics |
| **Unit Tests** | 14 tests | 27 tests |

---

## Files Modified

### Database Layer
- `database/connection.py` - Connection pooling, retry logic, health check
- `database/writers.py` - Batch insert functions

### Models
- `models/price_anomaly_detector.py` - Complete rewrite with caching
- `models/sales_anomaly_detector.py` - Complete rewrite with caching
- `models/price_suggester.py` - Complete rewrite with model selection
- `models/supplier_scorer.py` - Type conversion fixes

### Handlers
- `handlers/price_anomaly_handler.py` - Error handling, batch inserts
- `handlers/sales_anomaly_handler.py` - Error handling, batch inserts
- `handlers/price_suggestion_handler.py` - Error handling, batch inserts
- `handlers/demand_forecast_handler.py` - Type conversion fixes

### Scheduler
- `scheduler.py` - Parallel execution, metrics, health check

### Tests
- `tests/test_models.py` - Added tests for all 7 models

---

## Testing

Run all tests:
```bash
docker run --rm -v "C:/Users/Chris/Documents/vscode/backend/ai-service:/app" backend-ai-service pip install pytest && python -m pytest tests/test_models.py -v
```

Run all handlers:
```bash
docker run --rm --network backend_default \
  -e DATABASE_URL=postgres://user:pass@db:5432/stocks \
  -v "C:/Users/Chris/Documents/vscode/backend/ai-service:/app" \
  backend-ai-service python -c "from scheduler import run_all_jobs; run_all_jobs()"
```

---

## Configuration

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `DATABASE_URL` | `postgres://user:pass@db:5432/stocks` | Database connection string |
| `CRON_SCHEDULE` | `0 2 * * *` | When to run jobs (cron format) |
| `PARALLEL_EXECUTION` | `true` | Enable parallel handler execution |
| `MAX_WORKERS` | `4` | Max parallel workers |

### Model Thresholds

See `config/settings.py` for configurable model parameters.
