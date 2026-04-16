"""
AI Service Configuration Settings
"""

MODEL_THRESHOLDS = {
    # Original models
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
    # New models
    "demand_forecast": {
        "urgent_days": 7,      # Days until stockout for URGENT alert
        "high_days": 14,       # Days until stockout for HIGH alert
        "min_data_points": 30, # Minimum days of data required
        "forecast_horizon": 30, # Default forecast days
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
        "poor_threshold": 60,  # Score below this triggers alert
    },
}
