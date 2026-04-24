"""
Price Anomaly Detection

Detects unusual price changes using Isolation Forest algorithm.
Features:
- Per-product expected prices based on historical rolling average
- Model caching for efficiency
- Configurable contamination rate
- Detailed anomaly scoring
"""

import pickle
import logging
from pathlib import Path
from datetime import datetime, timedelta
from typing import Dict, Any, List, Optional

import numpy as np
from sklearn.ensemble import IsolationForest

logger = logging.getLogger(__name__)


class PriceAnomalyDetector:
    """
    Detects price anomalies using Isolation Forest.

    Features:
    - Per-product expected price calculation (rolling average)
    - Model persistence for faster subsequent runs
    - Configurable sensitivity via contamination parameter
    """

    def __init__(
        self,
        model_dir: str = "/app/saved_models",
        cache_days: int = 7,
        contamination: float = 0.05,
        rolling_window: int = 30
    ):
        """
        Initialize the price anomaly detector.

        Args:
            model_dir: Directory to store cached models
            cache_days: Days before model needs retraining
            contamination: Expected proportion of anomalies (0.01 to 0.5)
            rolling_window: Days for rolling average calculation
        """
        self.model_dir = Path(model_dir)
        self.model_dir.mkdir(parents=True, exist_ok=True)
        self.model_path = self.model_dir / "price_anomaly_model.pkl"
        self.cache_days = cache_days
        self.contamination = contamination
        self.rolling_window = rolling_window
        self.model: Optional[IsolationForest] = None
        self.product_stats: Dict[int, Dict[str, float]] = {}

    def predict(self, price_history: List[Dict[str, Any]]) -> List[Dict[str, Any]]:
        """
        Detect price anomalies in the given price history.

        Args:
            price_history: List of price records with product_id, price, buying_price

        Returns:
            List of anomaly predictions with scores and expected prices
        """
        if len(price_history) < 2:
            logger.warning("Not enough price history for anomaly detection")
            return []

        # Calculate per-product statistics for expected prices
        self._calculate_product_stats(price_history)

        # Prepare features
        features_list = []
        for record in price_history:
            pid = record["product_id"]
            price = float(record["price"])
            buying_price = float(record.get("buying_price", 0) or 0)

            # Get per-product expected price
            stats = self.product_stats.get(pid, {})
            expected_price = stats.get("mean", price)
            price_std = stats.get("std", 1)

            # Feature: price deviation from expected (z-score)
            z_score = (price - expected_price) / price_std if price_std > 0 else 0

            # Feature: margin ratio
            margin_ratio = (price - buying_price) / price if price > 0 else 0

            features_list.append([price, buying_price, z_score, margin_ratio])

        X = np.array(features_list)

        # Load or train model
        self.model = self._get_or_train_model(X)

        # Predict anomalies
        scores = self.model.decision_function(X)
        predictions = self.model.predict(X)

        # Build results
        results = []
        for i, record in enumerate(price_history):
            pid = record["product_id"]
            price = float(record["price"])
            stats = self.product_stats.get(pid, {})

            results.append({
                "product_id": pid,
                "current_price": price,
                "expected_price": round(stats.get("mean", price), 2),
                "price_std": round(stats.get("std", 0), 2),
                "anomaly_score": round(float(scores[i]), 4),
                "is_anomaly": bool(predictions[i] == -1),
            })

        anomaly_count = sum(1 for r in results if r["is_anomaly"])
        logger.info(f"Detected {anomaly_count} price anomalies out of {len(results)} records")

        return results

    def _calculate_product_stats(self, price_history: List[Dict[str, Any]]) -> None:
        """Calculate per-product price statistics for expected price estimation."""
        product_prices: Dict[int, List[float]] = {}

        for record in price_history:
            pid = record["product_id"]
            price = float(record["price"])

            if pid not in product_prices:
                product_prices[pid] = []
            product_prices[pid].append(price)

        self.product_stats = {}
        for pid, prices in product_prices.items():
            # Use rolling window if enough data
            recent_prices = prices[-self.rolling_window:] if len(prices) > self.rolling_window else prices

            self.product_stats[pid] = {
                "mean": float(np.mean(recent_prices)),
                "std": float(np.std(recent_prices)) if len(recent_prices) > 1 else 1.0,
                "min": float(np.min(recent_prices)),
                "max": float(np.max(recent_prices)),
                "count": len(prices)
            }

    def _get_or_train_model(self, X: np.ndarray) -> IsolationForest:
        """Get cached model or train new one."""
        if self.model_path.exists():
            model_age = (datetime.now() - datetime.fromtimestamp(self.model_path.stat().st_mtime)).days
            if model_age < self.cache_days:
                try:
                    with open(self.model_path, 'rb') as f:
                        cached = pickle.load(f)
                    logger.info(f"Loaded cached price anomaly model (age: {model_age} days)")
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
        logger.info(f"Trained price anomaly detector on {len(X)} samples")

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
            logger.info("Cleared price anomaly model cache")
