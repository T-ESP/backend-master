"""
Demand Forecast Handler

Runs demand forecasting for all eligible products and saves results
to the database with notifications for urgent restock needs.
"""

from handlers.base_handler import BaseHandler
from models.demand_forecaster import DemandForecaster
from database.connection import get_connection, return_connection
from database.writers import insert_demand_forecast, insert_notification
from config.settings import MODEL_THRESHOLDS


class DemandForecastHandler(BaseHandler):
    """Handler for running demand forecasts on schedule"""

    def __init__(self):
        super().__init__()
        self.model = DemandForecaster()
        self.thresholds = MODEL_THRESHOLDS.get("demand_forecast", {
            "urgent_days": 7,
            "high_days": 14
        })

    def run(self):
        """Run demand forecasting for all eligible products"""
        self.logger.info("Starting demand forecast handler")
        conn = get_connection()

        try:
            # Forecast all products with sufficient data
            results = self.model.forecast_all_products(
                conn,
                days_ahead=30,
                min_data_points=30
            )

            successful = 0
            failed = 0

            for result in results:
                product_id = result["product_id"]

                if not result["success"]:
                    failed += 1
                    self.logger.warning(f"Forecast failed for product {product_id}: {result.get('error')}")
                    continue

                successful += 1
                forecast = result["forecast"]

                # Save forecast to database (convert numpy types to Python types)
                forecast_id = insert_demand_forecast(conn, {
                    "product_id": int(product_id),
                    "forecast_date": forecast["forecast_generated_at"],
                    "forecast_days": int(forecast["forecast_days"]),
                    "total_predicted_demand": float(forecast["summary"]["total_predicted_demand"]),
                    "avg_daily_demand": float(forecast["summary"]["avg_daily_demand"]),
                    "current_stock": int(forecast["stock_recommendation"]["current_stock"]),
                    "recommended_stock": int(forecast["stock_recommendation"]["recommended_stock_level"]),
                    "reorder_quantity": int(forecast["stock_recommendation"]["reorder_quantity"]),
                    "days_until_stockout": int(forecast["stock_recommendation"]["days_until_stockout"]),
                    "urgency": forecast["stock_recommendation"]["urgency"],
                    "mape": float(forecast["model_accuracy"]["mape"]),
                    "rmse": float(forecast["model_accuracy"]["rmse"])
                })

                # Create notification for urgent/high priority items
                urgency = forecast["stock_recommendation"]["urgency"]
                days_left = forecast["stock_recommendation"]["days_until_stockout"]

                if urgency in ["URGENT", "HIGH"]:
                    severity = "CRITICAL" if urgency == "URGENT" else "HIGH"
                    insert_notification(conn, {
                        "product_id": product_id,
                        "model_type": "demand_forecast",
                        "category": "alert",
                        "notification_type": "low_stock_warning",
                        "severity": severity,
                        "message": f"Stock will run out in ~{days_left} days. "
                                   f"Recommend ordering {forecast['stock_recommendation']['reorder_quantity']} units.",
                        "action_recommended": f"Order {forecast['stock_recommendation']['reorder_quantity']} units immediately",
                        "related_result_id": forecast_id
                    })

            self.logger.info(f"Demand forecast completed: {successful} successful, {failed} failed")

        except Exception as e:
            self.logger.error(f"Demand forecast handler error: {e}", exc_info=True)
            raise
        finally:
            return_connection(conn)
