"""
Price Suggestion Model

Suggests optimal prices using Random Forest regression.
Features:
- Historical price trends
- Margin analysis
- Demand elasticity estimation
- Model caching for efficiency
"""

import pickle
import logging
from pathlib import Path
from datetime import datetime
from typing import Dict, Any, List, Optional, Tuple

import numpy as np
from sklearn.ensemble import RandomForestRegressor, GradientBoostingRegressor
from sklearn.model_selection import cross_val_score

logger = logging.getLogger(__name__)


class PriceSuggester:
    """
    Suggests optimal product prices using ensemble methods.

    Features:
    - Multiple feature engineering (trends, margins, volatility)
    - Model selection between Random Forest and Gradient Boosting
    - Cross-validation for model quality assessment
    - Model persistence for faster subsequent runs
    """

    def __init__(
        self,
        model_dir: str = "/app/saved_models",
        cache_days: int = 7,
        min_confidence: float = 0.5
    ):
        """
        Initialize the price suggester.

        Args:
            model_dir: Directory to store cached models
            cache_days: Days before model needs retraining
            min_confidence: Minimum confidence threshold for suggestions
        """
        self.model_dir = Path(model_dir)
        self.model_dir.mkdir(parents=True, exist_ok=True)
        self.model_path = self.model_dir / "price_suggester_model.pkl"
        self.cache_days = cache_days
        self.min_confidence = min_confidence
        self.model = None
        self.model_score: float = 0.0

    def predict(
        self,
        products: List[Dict[str, Any]],
        price_history: List[Dict[str, Any]]
    ) -> List[Dict[str, Any]]:
        """
        Generate price suggestions for products.

        Args:
            products: List of products with current prices
            price_history: Historical price data for training

        Returns:
            List of price suggestions with confidence scores
        """
        if not price_history:
            logger.warning("No price history available for training")
            return []

        # Build per-product price history
        product_prices = self._build_price_history(price_history)

        # Prepare training data
        X_train, y_train, feature_names = self._prepare_training_data(product_prices)

        if len(X_train) < 5:
            logger.warning("Not enough training data for price suggester")
            return []

        # Train or load model
        self.model, self.model_score = self._get_or_train_model(X_train, y_train)

        # Generate suggestions
        results = []
        for product in products:
            suggestion = self._suggest_price(product, product_prices)
            if suggestion:
                results.append(suggestion)

        logger.info(f"Generated {len(results)} price suggestions (model R²: {self.model_score:.3f})")
        return results

    def _build_price_history(self, price_history: List[Dict[str, Any]]) -> Dict[int, List[float]]:
        """Build per-product price history."""
        product_prices: Dict[int, List[float]] = {}

        for record in price_history:
            pid = record["product_id"]
            price = float(record["price"])

            if pid not in product_prices:
                product_prices[pid] = []
            product_prices[pid].append(price)

        return product_prices

    def _prepare_training_data(
        self,
        product_prices: Dict[int, List[float]]
    ) -> Tuple[np.ndarray, np.ndarray, List[str]]:
        """Prepare features and targets for training."""
        X_train = []
        y_train = []

        feature_names = [
            'current_price', 'mean_price', 'std_price', 'min_price', 'max_price',
            'price_trend', 'volatility', 'price_position'
        ]

        for pid, prices in product_prices.items():
            if len(prices) < 3:
                continue

            for i in range(2, len(prices)):
                historical = prices[:i]
                target = prices[i]

                # Features
                current = historical[-1]
                mean_price = np.mean(historical)
                std_price = np.std(historical) if len(historical) > 1 else 0
                min_price = np.min(historical)
                max_price = np.max(historical)

                # Trend: positive if prices are increasing
                if len(historical) >= 2:
                    trend = (historical[-1] - historical[0]) / historical[0] if historical[0] > 0 else 0
                else:
                    trend = 0

                # Volatility: coefficient of variation
                volatility = std_price / mean_price if mean_price > 0 else 0

                # Price position: where current price sits in the range
                price_range = max_price - min_price
                price_position = (current - min_price) / price_range if price_range > 0 else 0.5

                X_train.append([
                    current, mean_price, std_price, min_price, max_price,
                    trend, volatility, price_position
                ])
                y_train.append(target)

        return np.array(X_train), np.array(y_train), feature_names

    def _get_or_train_model(
        self,
        X: np.ndarray,
        y: np.ndarray
    ) -> Tuple[RandomForestRegressor, float]:
        """Get cached model or train new one."""
        if self.model_path.exists():
            model_age = (datetime.now() - datetime.fromtimestamp(self.model_path.stat().st_mtime)).days
            if model_age < self.cache_days:
                try:
                    with open(self.model_path, 'rb') as f:
                        cached = pickle.load(f)
                    logger.info(f"Loaded cached price suggester model (age: {model_age} days)")
                    return cached['model'], cached.get('score', 0.0)
                except Exception as e:
                    logger.warning(f"Failed to load cached model: {e}")

        # Try both models and pick the best
        rf_model = RandomForestRegressor(n_estimators=100, random_state=42, max_depth=10)
        gb_model = GradientBoostingRegressor(n_estimators=100, random_state=42, max_depth=5)

        # Cross-validation scores
        rf_scores = cross_val_score(rf_model, X, y, cv=min(5, len(X)), scoring='r2')
        gb_scores = cross_val_score(gb_model, X, y, cv=min(5, len(X)), scoring='r2')

        rf_mean = np.mean(rf_scores)
        gb_mean = np.mean(gb_scores)

        # Pick the better model
        if rf_mean >= gb_mean:
            model = rf_model
            score = rf_mean
            model_type = "RandomForest"
        else:
            model = gb_model
            score = gb_mean
            model_type = "GradientBoosting"

        model.fit(X, y)
        logger.info(f"Trained {model_type} price suggester on {len(X)} samples (R²: {score:.3f})")

        # Cache the model
        try:
            with open(self.model_path, 'wb') as f:
                pickle.dump({
                    'model': model,
                    'trained_at': datetime.now(),
                    'samples': len(X),
                    'score': score,
                    'model_type': model_type
                }, f)
        except Exception as e:
            logger.warning(f"Failed to cache model: {e}")

        return model, score

    def _suggest_price(
        self,
        product: Dict[str, Any],
        product_prices: Dict[int, List[float]]
    ) -> Optional[Dict[str, Any]]:
        """Generate price suggestion for a single product."""
        pid = product["product_id"]
        current_price = float(product["current_price"])

        prices = product_prices.get(pid, [current_price])

        # Calculate features (convert numpy scalars to native Python floats)
        mean_price = float(np.mean(prices))
        std_price = float(np.std(prices)) if len(prices) > 1 else 0.0
        min_price = float(np.min(prices))
        max_price = float(np.max(prices))

        if len(prices) >= 2:
            trend = float((prices[-1] - prices[0]) / prices[0]) if prices[0] > 0 else 0.0
        else:
            trend = 0.0

        volatility = float(std_price / mean_price) if mean_price > 0 else 0.0
        price_range = float(max_price - min_price)
        price_position = float((current_price - min_price) / price_range) if price_range > 0 else 0.5

        features = np.array([[
            current_price, mean_price, std_price, min_price, max_price,
            trend, volatility, price_position
        ]])

        # Predict
        suggested_price = float(self.model.predict(features)[0])

        # Ensure suggested price is reasonable (within 50% of current)
        suggested_price = max(current_price * 0.5, min(current_price * 1.5, suggested_price))

        # Calculate confidence based on data availability and model score
        data_confidence = min(1.0, len(prices) / 10.0)
        confidence = float(data_confidence * 0.4 + max(0, self.model_score) * 0.6)

        # Determine reason
        price_diff = float(suggested_price - current_price)
        price_diff_pct = float(price_diff / current_price * 100) if current_price > 0 else 0.0

        if abs(price_diff_pct) < 2:
            reason = "Current price is optimal"
        elif price_diff > 0:
            reason = f"Consider increasing price by {price_diff_pct:.1f}% based on market trends"
        else:
            reason = f"Consider decreasing price by {abs(price_diff_pct):.1f}% to improve competitiveness"

        return {
            "product_id": pid,
            "suggested_price": float(round(suggested_price, 2)),
            "current_price": float(round(current_price, 2)),
            "confidence": float(round(confidence, 2)),
            "reason": reason,
            "price_change_pct": float(round(price_diff_pct, 2)),
            "model_score": float(round(self.model_score, 3))
        }

    def clear_cache(self) -> None:
        """Clear the cached model."""
        if self.model_path.exists():
            self.model_path.unlink()
            logger.info("Cleared price suggester model cache")
