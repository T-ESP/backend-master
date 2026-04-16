"""
Demand Forecasting using Facebook Prophet

Predicts product demand using time series forecasting with automatic
seasonality detection (weekly, yearly) and generates stock recommendations.
"""

from datetime import datetime, timedelta
from pathlib import Path
from typing import Optional, Dict, Any, List
import pickle
import pandas as pd
import numpy as np
from prophet import Prophet
import logging

logger = logging.getLogger(__name__)


class DemandForecaster:
    """
    Demand forecasting engine using Prophet

    Features:
    - Time series forecasting with Prophet
    - Weekly and yearly seasonality detection
    - Confidence intervals (80%)
    - Stock recommendations with urgency levels
    - Model caching (7 days)
    """

    def __init__(self, model_dir: str = "/app/saved_models", cache_days: int = 7):
        self.model_dir = Path(model_dir)
        self.model_dir.mkdir(parents=True, exist_ok=True)
        self.cache_days = cache_days

    def forecast_product(
        self,
        conn,
        product_id: int,
        days_ahead: int = 30,
        force_retrain: bool = False,
        confidence_level: float = 0.8
    ) -> Dict[str, Any]:
        """Generate demand forecast for a product"""
        if days_ahead < 1 or days_ahead > 90:
            raise ValueError("days_ahead must be between 1 and 90")

        # Load product info
        product_info = self._get_product_info(conn, product_id)

        # Get or train model
        model, training_info = self._get_or_train_model(
            conn, product_id, force_retrain, confidence_level
        )

        # Generate forecast
        forecast = self._generate_forecast(model, days_ahead)

        # Build response
        return self._build_forecast_result(
            product_id, product_info, forecast, training_info, days_ahead, confidence_level
        )

    def forecast_all_products(
        self,
        conn,
        days_ahead: int = 30,
        min_data_points: int = 30
    ) -> List[Dict[str, Any]]:
        """Generate forecasts for all eligible products"""
        # Get products with enough data
        query = """
        SELECT DISTINCT lor.product_id_lor as product_id
        FROM line_order_lor lor
        JOIN order_ord o ON lor.order_id_lor = o.id_ord
        WHERE o.order_date_ord >= NOW() - INTERVAL '2 years'
        GROUP BY lor.product_id_lor
        HAVING COUNT(DISTINCT DATE(o.order_date_ord)) >= %s
        ORDER BY lor.product_id_lor;
        """

        with conn.cursor() as cur:
            cur.execute(query, (min_data_points,))
            rows = cur.fetchall()

        results = []
        for row in rows:
            product_id = row[0]
            try:
                forecast = self.forecast_product(conn, product_id, days_ahead)
                results.append({
                    "product_id": product_id,
                    "success": True,
                    "forecast": forecast
                })
            except Exception as e:
                logger.warning(f"Failed to forecast product {product_id}: {e}")
                results.append({
                    "product_id": product_id,
                    "success": False,
                    "error": str(e)
                })

        return results

    def _get_product_info(self, conn, product_id: int) -> Dict[str, Any]:
        """Get basic product information"""
        query = """
        SELECT
            p.id_pro, p.name_pro, p.stock_quantity_pro,
            COALESCE(pp.price_prp, 0) as selling_price
        FROM products_pro p
        LEFT JOIN LATERAL (
            SELECT price_prp FROM productprices_prp
            WHERE product_ref_prp = p.id_pro
            ORDER BY created_at DESC LIMIT 1
        ) pp ON true
        WHERE p.id_pro = %s;
        """

        with conn.cursor() as cur:
            cur.execute(query, (product_id,))
            row = cur.fetchone()

        if not row:
            raise ValueError(f"Product {product_id} not found")

        return {
            "id": row[0],
            "name": row[1],
            "current_stock": row[2] or 0,
            "selling_price": float(row[3] or 0)
        }

    def _load_sales_history(self, conn, product_id: int, min_data_points: int = 30) -> pd.DataFrame:
        """Load historical sales data for training"""
        start_date = datetime.now() - timedelta(days=730)

        query = """
        SELECT DATE(o.order_date_ord) as date, COALESCE(SUM(lor.quantity_lor), 0) as quantity
        FROM line_order_lor lor
        JOIN order_ord o ON lor.order_id_lor = o.id_ord
        WHERE lor.product_id_lor = %s AND o.order_date_ord >= %s
        GROUP BY DATE(o.order_date_ord)
        ORDER BY date;
        """

        with conn.cursor() as cur:
            cur.execute(query, (product_id, start_date))
            rows = cur.fetchall()

        if not rows:
            raise ValueError(f"No sales history found for product {product_id}")

        df = pd.DataFrame([{"ds": row[0], "y": float(row[1])} for row in rows])
        df['ds'] = pd.to_datetime(df['ds'])

        # Fill missing dates with zero sales
        # Normalize to date-only (midnight) to ensure merge works correctly
        date_range = pd.date_range(start=start_date.date(), end=datetime.now().date(), freq='D')
        complete_df = pd.DataFrame({'ds': date_range})
        df = complete_df.merge(df, on='ds', how='left')
        df['y'] = df['y'].fillna(0)

        if len(df) < min_data_points:
            raise ValueError(
                f"Insufficient data for product {product_id}: {len(df)} days, need {min_data_points}"
            )

        return df

    def _get_or_train_model(self, conn, product_id: int, force_retrain: bool, confidence_level: float):
        """Get cached model or train new one"""
        model_path = self.model_dir / f"forecast_product_{product_id}.pkl"

        if not force_retrain and model_path.exists():
            model_age = (datetime.now() - datetime.fromtimestamp(model_path.stat().st_mtime)).days
            if model_age < self.cache_days:
                with open(model_path, 'rb') as f:
                    data = pickle.load(f)
                return data['model'], data['training_info']

        # Train new model
        model, training_info = self._train_model(conn, product_id, confidence_level)

        # Save to cache
        with open(model_path, 'wb') as f:
            pickle.dump({'model': model, 'training_info': training_info, 'trained_at': datetime.now()}, f)

        return model, training_info

    def _train_model(self, conn, product_id: int, confidence_level: float):
        """Train Prophet model"""
        df = self._load_sales_history(conn, product_id)

        # Suppress Prophet logging
        import logging as prophet_logging
        prophet_logging.getLogger('prophet').setLevel(prophet_logging.WARNING)
        prophet_logging.getLogger('cmdstanpy').setLevel(prophet_logging.WARNING)

        model = Prophet(
            daily_seasonality=False,
            weekly_seasonality=True,
            yearly_seasonality=True,
            seasonality_mode='additive',
            changepoint_prior_scale=0.05,
            interval_width=confidence_level
        )

        train_start = datetime.now()
        model.fit(df)
        train_duration = (datetime.now() - train_start).total_seconds()

        # Calculate accuracy metrics
        predictions = model.predict(df)
        mape = self._calculate_mape(df['y'].values, predictions['yhat'].values)
        rmse = self._calculate_rmse(df['y'].values, predictions['yhat'].values)

        training_info = {
            'product_id': product_id,
            'training_date_range': {'start': df['ds'].min().date(), 'end': df['ds'].max().date()},
            'data_points': len(df),
            'train_duration_seconds': round(train_duration, 2),
            'model_version': 'prophet_v1.1.5',
            'accuracy': {'mape': round(mape, 2), 'rmse': round(rmse, 2)},
            'trained_at': datetime.now()
        }

        return model, training_info

    def _generate_forecast(self, model: Prophet, days_ahead: int) -> pd.DataFrame:
        """Generate predictions"""
        future = model.make_future_dataframe(periods=days_ahead, freq='D')
        return model.predict(future).tail(days_ahead)

    @staticmethod
    def _calculate_mape(actual: np.ndarray, predicted: np.ndarray) -> float:
        mask = actual != 0
        if not mask.any():
            return 0.0
        return np.mean(np.abs((actual[mask] - predicted[mask]) / actual[mask])) * 100

    @staticmethod
    def _calculate_rmse(actual: np.ndarray, predicted: np.ndarray) -> float:
        return np.sqrt(np.mean((actual - predicted) ** 2))

    def _build_forecast_result(
        self, product_id, product_info, forecast, training_info, days_ahead, confidence_level
    ) -> Dict[str, Any]:
        """Build complete forecast result"""
        predictions = []
        for _, row in forecast.iterrows():
            pred_date = row['ds'].date() if hasattr(row['ds'], 'date') else row['ds']
            predictions.append({
                'date': pred_date.isoformat(),
                'predicted_demand': round(max(0, row['yhat']), 2),
                'lower_bound': round(max(0, row['yhat_lower']), 2),
                'upper_bound': round(max(0, row['yhat_upper']), 2),
                'confidence': confidence_level
            })

        total_predicted = sum(p['predicted_demand'] for p in predictions)
        avg_daily = total_predicted / len(predictions) if predictions else 0

        # Stock recommendation
        current_stock = product_info['current_stock']
        total_upper = sum(p['upper_bound'] for p in predictions)
        recommended_stock = int(total_upper * 1.2)
        reorder_needed = recommended_stock - current_stock
        days_until_stockout = int(current_stock / avg_daily) if avg_daily > 0 else 999

        if reorder_needed <= 0:
            urgency = "LOW"
        elif days_until_stockout <= 7:
            urgency = "URGENT"
        elif days_until_stockout <= 14:
            urgency = "HIGH"
        elif days_until_stockout <= 30:
            urgency = "MEDIUM"
        else:
            urgency = "LOW"

        return {
            'product_id': product_id,
            'product_name': product_info['name'],
            'forecast_generated_at': datetime.now().isoformat(),
            'forecast_days': days_ahead,
            'model_accuracy': training_info['accuracy'],
            'predictions': predictions,
            'summary': {
                'total_predicted_demand': round(total_predicted, 2),
                'avg_daily_demand': round(avg_daily, 2)
            },
            'stock_recommendation': {
                'current_stock': current_stock,
                'recommended_stock_level': recommended_stock,
                'reorder_quantity': max(0, reorder_needed),
                'days_until_stockout': days_until_stockout,
                'urgency': urgency
            }
        }
