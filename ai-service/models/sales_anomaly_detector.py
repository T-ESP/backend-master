"""
Sales Anomaly Detection

Detects unusual sales patterns using Isolation Forest algorithm.
Features:
- Per-product expected sales based on historical data
- Time-based features (day of week, trends)
- Model caching for efficiency
- Detailed anomaly scoring
"""

import pickle
import logging
from pathlib import Path
from datetime import datetime
from typing import Dict, Any, List, Optional

import numpy as np
from sklearn.ensemble import IsolationForest

logger = logging.getLogger(__name__)


class SalesAnomalyDetector:
    """
    Detects sales anomalies using Isolation Forest.

    Features:
    - Per-product expected sales calculation
    - Multiple feature engineering (volume, frequency, deviation)
    - Model persistence for faster subsequent runs
    - Configurable sensitivity via contamination parameter
    """

    def __init__(
        self,
        model_dir: str = "/app/saved_models",
        cache_days: int = 7,
        contamination: float = 0.05
    ):
        """
        Initialize the sales anomaly detector.

        Args:
            model_dir: Directory to store cached models
            cache_days: Days before model needs retraining
            contamination: Expected proportion of anomalies (0.01 to 0.5)
        """
        self.model_dir = Path(model_dir)
        self.model_dir.mkdir(parents=True, exist_ok=True)
        self.model_path = self.model_dir / "sales_anomaly_model.pkl"
        self.cache_days = cache_days
        self.contamination = contamination
        self.model: Optional[IsolationForest] = None
        self.product_stats: Dict[int, Dict[str, float]] = {}

    def predict(self, sales_data: List[Dict[str, Any]]) -> List[Dict[str, Any]]:
        """
        Detect sales anomalies in the given sales data.

        Args:
            sales_data: List of sales records with product_id, sales_volume, order_count

        Returns:
            List of anomaly predictions with scores and expected sales
        """
        if len(sales_data) < 2:
            logger.warning("Not enough sales data for anomaly detection")
            return []

        # Calculate per-product statistics
        self._calculate_product_stats(sales_data)

        # Prepare features
        features_list = []
        for record in sales_data:
            pid = record["product_id"]
            volume = int(record["sales_volume"])
            order_count = int(record.get("order_count", 0) or 0)

            # Get per-product statistics
            stats = self.product_stats.get(pid, {})
            expected_volume = stats.get("mean_volume", volume)
            volume_std = stats.get("std_volume", 1)

            # Feature: volume z-score
            z_score = (volume - expected_volume) / volume_std if volume_std > 0 else 0

            # Feature: average order size
            avg_order_size = volume / order_count if order_count > 0 else volume

            # Feature: order frequency deviation
            expected_orders = stats.get("mean_orders", order_count)
            orders_std = stats.get("std_orders", 1)
            orders_z_score = (order_count - expected_orders) / orders_std if orders_std > 0 else 0

            features_list.append([volume, order_count, z_score, avg_order_size, orders_z_score])

        X = np.array(features_list)

        # Load or train model
        self.model = self._get_or_train_model(X)

        # Predict anomalies
        scores = self.model.decision_function(X)
        predictions = self.model.predict(X)

        # Build results
        results = []
        for i, record in enumerate(sales_data):
            pid = record["product_id"]
            volume = int(record["sales_volume"])
            stats = self.product_stats.get(pid, {})

            results.append({
                "product_id": pid,
                "sales_volume": volume,
                "expected_sales": round(stats.get("mean_volume", float(volume)), 2),
                "sales_std": round(stats.get("std_volume", 0), 2),
                "order_count": int(record.get("order_count", 0) or 0),
                "anomaly_score": round(float(scores[i]), 4),
                "is_anomaly": bool(predictions[i] == -1),
            })

        anomaly_count = sum(1 for r in results if r["is_anomaly"])
        logger.info(f"Detected {anomaly_count} sales anomalies out of {len(results)} records")

        return results

    def _calculate_product_stats(self, sales_data: List[Dict[str, Any]]) -> None:
        """Calculate per-product sales statistics."""
        product_volumes: Dict[int, List[int]] = {}
        product_orders: Dict[int, List[int]] = {}

        for record in sales_data:
            pid = record["product_id"]
            volume = int(record["sales_volume"])
            orders = int(record.get("order_count", 0) or 0)

            if pid not in product_volumes:
                product_volumes[pid] = []
                product_orders[pid] = []

            product_volumes[pid].append(volume)
            product_orders[pid].append(orders)

        self.product_stats = {}
        for pid in product_volumes:
            volumes = product_volumes[pid]
            orders = product_orders[pid]

            self.product_stats[pid] = {
                "mean_volume": float(np.mean(volumes)),
                "std_volume": float(np.std(volumes)) if len(volumes) > 1 else 1.0,
                "min_volume": float(np.min(volumes)),
                "max_volume": float(np.max(volumes)),
                "mean_orders": float(np.mean(orders)),
                "std_orders": float(np.std(orders)) if len(orders) > 1 else 1.0,
                "count": len(volumes)
            }

    def _get_or_train_model(self, X: np.ndarray) -> IsolationForest:
        """Get cached model or train new one."""
        if self.model_path.exists():
            model_age = (datetime.now() - datetime.fromtimestamp(self.model_path.stat().st_mtime)).days
            if model_age < self.cache_days:
                try:
                    with open(self.model_path, 'rb') as f:
                        cached = pickle.load(f)
                    logger.info(f"Loaded cached sales anomaly model (age: {model_age} days)")
                    return cached['model']
                except Exception as e:
                    logger.warning(f"Failed to load cached model: {e}")

        # Train new model
        model = IsolationForest(
            contamination=self.contamination,
            random_state=42,
            n_estimators=100,
            max_samples='auto'
        )
        model.fit(X)
        logger.info(f"Trained sales anomaly detector on {len(X)} samples")

        # Cache the model
        try:
            with open(self.model_path, 'wb') as f:
                pickle.dump({
                    'model': model,
                    'trained_at': datetime.now(),
                    'samples': len(X)
                }, f)
        except Exception as e:
            logger.warning(f"Failed to cache model: {e}")

        return model

    def clear_cache(self) -> None:
        """Clear the cached model."""
        if self.model_path.exists():
            self.model_path.unlink()
            logger.info("Cleared sales anomaly model cache")
