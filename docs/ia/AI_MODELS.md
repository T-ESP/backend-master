# AI Models - How They Work

The ai-service runs 7 machine learning models that analyze your stock management data. This document explains each model, the algorithm behind it, what data it uses, and what it produces.

---

## Model 1: ABC-XYZ Classifier

**What it does:** Classifies every product into a 2-letter code (e.g., AX, BY, CZ) that tells you how important it is and how predictable its demand is. This drives your inventory strategy.

**Algorithm:** Rule-based (Pareto principle + statistical variance)

### How ABC Works (Revenue Importance)
Products are sorted by revenue contribution, then split:
- **A** = Top 80% of total revenue (your most important products)
- **B** = Next 15% (moderately important)
- **C** = Remaining 5% (low-value products)

### How XYZ Works (Demand Predictability)
The coefficient of variation (CV = standard deviation / mean) of order quantities is calculated:
- **X** = CV < 0.5 (highly predictable, steady demand)
- **Y** = CV between 0.5 and 1.0 (moderate variability)
- **Z** = CV >= 1.0 (unpredictable, erratic demand)

### The 9 Combined Classes

| Class | Meaning | Strategy |
|-------|---------|----------|
| **AX** | High value, predictable | Just-In-Time with safety stock. Highest priority. |
| **AY** | High value, some variability | Regular review with moderate safety stock |
| **AZ** | High value, unpredictable | Large safety stock, frequent monitoring |
| **BX** | Medium value, predictable | Periodic ordering, standard safety stock |
| **BY** | Medium value, some variability | Regular review cycles |
| **BZ** | Medium value, unpredictable | Increase safety stock, review frequently |
| **CX** | Low value, predictable | Automated ordering, minimal attention |
| **CY** | Low value, some variability | Simple min-max system |
| **CZ** | Low value, unpredictable | Keep minimal stock, order on demand |

### Input Data
- Order history from the last 90 days
- Minimum 5 transactions per product to qualify

### Output (written to `product_classifications`)
```
product_id, abc_class, xyz_class, combined_class,
total_revenue, revenue_contribution_pct, total_units_sold,
coefficient_of_variation, strategy, priority
```

### Alerts Generated
- When a product's class changes (especially if it was or becomes Class A)
- Severity: MEDIUM

---

## Model 2: Product Clusterer

**What it does:** Groups similar products together using unsupervised machine learning. Products in the same cluster share similar sales patterns, margins, and behavior. Useful for discovering hidden product segments.

**Algorithm:** K-Means Clustering (scikit-learn)

### How It Works

1. **Feature Engineering** - For each product, 6 normalized scores (0 to 1) are calculated:
   - **Revenue Score** = product revenue / max revenue across all products
   - **Variability Score** = coefficient of variation / max CV
   - **Trend Score** = 50% activity ratio + 50% recency score
   - **Seasonality Score** = variability x revenue (interaction term)
   - **Frequency Score** = order count / max orders
   - **Margin Score** = (selling_price - buying_price) / selling_price

2. **Optimal K Selection** - Tests cluster counts from 2 to 10, picks the one with the highest silhouette score (a measure of how well-separated the clusters are)

3. **Cluster Naming** - Each cluster gets a descriptive name based on its centroid characteristics:
   - "Cash Cows" - High revenue, low variability
   - "High-Value Volatile" - High revenue, high variability
   - "Rising Stars" - High trend score
   - "Steady Performers" - Balanced scores
   - "Unpredictable Low-Value" - Low revenue, high variability
   - etc.

### Input Data
- Daily sales aggregated over 90 days
- Product prices and buying prices for margin calculation

### Output (written to `product_clusters`)
```
product_id, cluster_id, cluster_name,
revenue_score, variability_score, trend_score,
seasonality_score, frequency_score, margin_score,
distance_to_centroid, n_clusters, silhouette_score
```

### Alerts Generated
None (informational only)

---

## Model 3: Supplier Scorer

**What it does:** Rates each supplier on a 0-100 scale based on delivery performance, quality, lead times, and order fulfillment. Assigns a rating from EXCELLENT to UNACCEPTABLE.

**Algorithm:** Weighted scoring system (not ML)

### Scoring Breakdown

| Component | Weight | What It Measures |
|-----------|--------|-----------------|
| **Delivery Performance** | 40% | % of restocks that arrived (status = 'received') |
| **Quality Score** | 25% | 100 minus cancellation rate |
| **Lead Time Score** | 20% | Speed (60%): 100 if <= 3 days, 50 if >= 14 days. Consistency (40%): low CV of lead times |
| **Fulfillment Rate** | 15% | Completion rate of orders |

### Rating Scale

| Rating | Score Range | Meaning |
|--------|------------|---------|
| EXCELLENT | >= 90 | Preferred supplier, prioritize orders |
| GOOD | 75 - 89 | Reliable, maintain relationship |
| ACCEPTABLE | 60 - 74 | Monitor closely, address issues |
| POOR | 40 - 59 | Needs improvement plan |
| UNACCEPTABLE | < 40 | Consider replacing |

### Input Data
- Restock history from the last 90 days
- Minimum 3 restocks per supplier to qualify

### Output (written to `supplier_scores`)
```
supplier_id, overall_score, delivery_score, quality_score,
lead_time_score, fulfillment_score, rating, total_restocks
```

### Alerts Generated
- POOR supplier: severity HIGH
- UNACCEPTABLE supplier: severity CRITICAL

---

## Model 4: Demand Forecaster

**What it does:** Predicts how many units of each product will be sold over the next 30 days. Uses this to calculate when you'll run out of stock and how much to reorder.

**Algorithm:** Facebook Prophet (time-series forecasting)

### How It Works

1. **Data Loading** - Pulls up to 2 years of daily sales (quantity sold per day per product). Missing days are filled with 0.

2. **Prophet Training** - Fits a Prophet model that automatically detects:
   - **Weekly seasonality** (e.g., more sales on weekends)
   - **Yearly seasonality** (e.g., holiday spikes)
   - **Trend changepoints** (sudden shifts in demand)

3. **Forecast Generation** - Predicts 30 days ahead with 80% confidence intervals (lower bound and upper bound for each day).

4. **Stock Recommendation** - Based on forecasts:
   - `recommended_stock = total_upper_bound x 1.2` (20% safety buffer)
   - `reorder_quantity = recommended_stock - current_stock`
   - `days_until_stockout = current_stock / avg_daily_demand`

### Urgency Levels

| Urgency | Days Until Stockout | Action |
|---------|-------------------|--------|
| URGENT | <= 7 days | Reorder immediately |
| HIGH | <= 14 days | Place order soon |
| MEDIUM | <= 30 days | Schedule reorder |
| LOW | > 30 days | No action needed |

### Accuracy Metrics
- **MAPE** (Mean Absolute Percentage Error) - Average % error. Lower is better. Under 20% is generally good.
- **RMSE** (Root Mean Squared Error) - Absolute error in units. Lower is better.

### Input Data
- Daily order quantities per product over 2 years
- Minimum 30 data points per product

### Output (written to `demand_forecasts`)
```
product_id, forecast_date, forecast_days,
total_predicted_demand, avg_daily_demand,
current_stock, recommended_stock, reorder_quantity,
days_until_stockout, urgency, mape, rmse
```

### Model Caching
- Trained models are saved as `.pkl` files (one per product)
- Cache expires after 7 days, then retrains automatically

### Alerts Generated
- URGENT stockout: severity CRITICAL
- HIGH stockout: severity HIGH

---

## Model 5: Price Anomaly Detector

**What it does:** Flags products whose current price is unusually different from their expected price based on historical patterns. Catches pricing errors, unexpected market shifts, or data entry mistakes.

**Algorithm:** Isolation Forest (scikit-learn)

### How It Works

1. **Expected Price Calculation** - For each product, computes a rolling average of the last 30 price entries as the "expected" price, plus standard deviation.

2. **Feature Engineering** - Each price record gets 4 features:
   - `price` - The actual price
   - `buying_price` - The supplier cost
   - `z_score` - How many standard deviations away from expected: `(price - mean) / std`
   - `margin_ratio` - The profit margin: `(price - buying_price) / price`

3. **Isolation Forest** - This algorithm works by randomly splitting data. Anomalies are easier to isolate (fewer splits needed), so they get higher anomaly scores. The `contamination=0.1` parameter tells the model to flag approximately 10% of records as anomalous.

4. **Scoring** - Each record gets:
   - `anomaly_score` - Negative values indicate anomalies (more negative = more anomalous)
   - `is_anomaly` - Boolean flag (true if the model labels it as -1)

### Input Data
- Full selling price history (all productprices_prp records)
- Product buying prices

### Output (written to `price_anomalies`)
```
product_id, current_price, expected_price,
anomaly_score, is_anomaly
```

### Model Caching
- Saved as `price_anomaly_model.pkl`, expires after 7 days

### Alerts Generated
- Every anomaly detected: severity HIGH

---

## Model 6: Sales Anomaly Detector

**What it does:** Flags products with unusual sales volumes - either unexpectedly high (potential fraud, bulk buy, viral trend) or unexpectedly low (distribution issue, competitor action).

**Algorithm:** Isolation Forest (scikit-learn)

### How It Works

Same principle as the Price Anomaly Detector, but applied to sales volumes.

**Features per product:**
- `volume` - Total units sold
- `order_count` - Number of distinct orders
- `z_score` - Volume deviation from expected
- `avg_order_size` - volume / order_count
- `orders_z_score` - Order count deviation from expected

### Severity Levels
- **CRITICAL** (anomaly_score <= -0.8): Extreme deviation
- **MEDIUM** (anomaly_score <= -0.5): Moderate deviation

### Input Data
- Aggregated sales per product (total volume + order count)

### Output (written to `sales_anomalies`)
```
product_id, sales_volume, expected_sales,
anomaly_score, is_anomaly
```

### Model Caching
- Saved as `sales_anomaly_model.pkl`, expires after 7 days

### Alerts Generated
- Score <= -0.8: severity HIGH
- Score <= -0.5: severity MEDIUM

---

## Model 7: Price Suggester

**What it does:** Recommends optimal selling prices for each product based on historical price trends, volatility, and market positioning. Tells you whether to raise, lower, or keep the current price.

**Algorithm:** Random Forest Regressor or Gradient Boosting Regressor (auto-selects the better one via cross-validation)

### How It Works

1. **Training Data Construction** - For each product with >= 3 price entries, creates sliding-window samples: uses prices[0:i] to predict prices[i]. This builds a dataset of "given these historical features, what was the next price?"

2. **Feature Engineering** (8 features):
   - `current_price` - Latest known price
   - `mean_price` - Average of historical prices
   - `std_price` - Price volatility
   - `min_price` / `max_price` - Range boundaries
   - `price_trend` - Direction: (last - first) / first
   - `volatility` - Coefficient of variation: std / mean
   - `price_position` - Where current sits in the range: (current - min) / (max - min)

3. **Model Selection** - Trains both Random Forest (max_depth=10) and Gradient Boosting (max_depth=5). Runs 5-fold cross-validation, picks the model with higher R^2 score.

4. **Prediction** - For each product, predicts the optimal price. Safety-capped at +/- 50% of current price.

5. **Confidence Score** - `confidence = 0.4 x data_confidence + 0.6 x model_R2`
   - `data_confidence = min(1.0, num_prices / 10)`

### Output (written to `price_suggestions`)
```
product_id, suggested_price, current_price,
reason, confidence
```

### Example Reasons
- "Current price is optimal" (change < 2%)
- "Consider increasing price by 5.2% based on market trends"
- "Consider decreasing price by 3.1% to improve competitiveness"

### Model Caching
- Saved as `price_suggester_model.pkl`, expires after 7 days

### Alerts Generated
- Only for suggestions with confidence >= 0.7
- Price change > 10%: severity HIGH
- Otherwise: severity MEDIUM
- Category: `suggestion` (not `alert`)

---

## Configuration Reference

All model thresholds are defined in `ai-service/config/settings.py`:

```python
MODEL_THRESHOLDS = {
    "price_suggester": {
        "min_confidence": 0.7,
    },
    "price_anomaly": {
        "anomaly_score_threshold": -0.5,
    },
    "sales_anomaly": {
        "anomaly_score_threshold": -0.5,
        "high_severity_threshold": -0.8,
    },
    "demand_forecast": {
        "urgent_days": 7,
        "high_days": 14,
        "min_data_points": 30,
        "forecast_horizon": 30,
    },
    "abc_xyz_classifier": {
        "lookback_days": 90,
        "min_transactions": 5,
    },
    "product_clusterer": {
        "lookback_days": 90,
        "min_transactions": 5,
        "max_clusters": 10,
    },
    "supplier_scorer": {
        "lookback_days": 90,
        "min_restocks": 3,
        "poor_threshold": 60,
    },
}
```

---

## Execution Timeline

A typical full run takes ~2 minutes and produces:

```
Group 1 (parallel):  PriceAnomaly + SalesAnomaly     ~5s
Group 2 (parallel):  Classification + Clustering + SupplierScoring  ~10s
Group 3:             DemandForecasting                ~90s  (trains Prophet per product)
Group 4:             PriceSuggestion                  ~15s

Total:               ~120 seconds
```

Output example from a run with seed data:
- 73 price anomalies detected
- 20 sales anomalies detected
- 145 products classified
- 482 products clustered into 2 groups
- 20 suppliers scored
- Demand forecasts for eligible products
- 659 price suggestions generated
- ~115 notifications/alerts created
