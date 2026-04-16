"""
Price Anomaly Handler

Runs price anomaly detection for all products and saves results
to the database with notifications for detected anomalies.
"""

from handlers.base_handler import BaseHandler
from models.price_anomaly_detector import PriceAnomalyDetector
from database.connection import get_connection, return_connection
from database.queries import get_price_history
from database.writers import insert_price_anomalies_batch, insert_notifications_batch, insert_price_anomaly
from config.settings import MODEL_THRESHOLDS


class PriceAnomalyHandler(BaseHandler):
    """Handler for detecting price anomalies on schedule"""

    def __init__(self):
        super().__init__()
        self.model = PriceAnomalyDetector()
        self.thresholds = MODEL_THRESHOLDS.get("price_anomaly", {
            "anomaly_score_threshold": -0.5
        })

    def run(self):
        """Run price anomaly detection for all products"""
        self.logger.info("Starting price anomaly handler")
        conn = get_connection()

        try:
            # Get price history
            price_data = get_price_history(conn)
            if not price_data:
                self.logger.warning("No price history found")
                return

            # Run anomaly detection
            predictions = self.model.predict(price_data)
            self.logger.info(f"Analyzed {len(predictions)} price records")

            if not predictions:
                self.logger.info("No predictions generated")
                return

            # Batch insert anomaly results
            try:
                result_ids = insert_price_anomalies_batch(conn, predictions)
            except Exception as e:
                self.logger.error(f"Failed to batch insert anomalies: {e}")
                # Fallback to individual inserts
                result_ids = self._insert_individually(conn, predictions)

            # Create notifications for anomalies
            notifications = []
            anomaly_count = 0

            score_threshold = self.thresholds.get("anomaly_score_threshold", -0.5)

            for i, prediction in enumerate(predictions):
                if prediction["is_anomaly"]:
                    score = prediction["anomaly_score"]
                    # Skip weak anomalies (score close to 0 means normal)
                    if score > score_threshold:
                        continue
                    anomaly_count += 1
                    result_id = result_ids[i] if i < len(result_ids) else None

                    notifications.append({
                        "product_id": prediction["product_id"],
                        "model_type": "price_anomaly",
                        "category": "alert",
                        "notification_type": "price_anomaly_detected",
                        "severity": "HIGH",
                        "message": f"Price anomaly detected: current {prediction['current_price']:.2f}, "
                                   f"expected {prediction['expected_price']:.2f} "
                                   f"(score: {prediction['anomaly_score']:.2f})",
                        "action_recommended": "Investigate unusual price change immediately",
                        "related_result_id": result_id,
                    })

            # Batch insert notifications
            if notifications:
                try:
                    insert_notifications_batch(conn, notifications)
                except Exception as e:
                    self.logger.error(f"Failed to batch insert notifications: {e}")

            self.logger.info(f"Price anomaly handler completed: {anomaly_count} anomalies detected")

        except Exception as e:
            self.logger.error(f"Price anomaly handler error: {e}", exc_info=True)
            raise
        finally:
            return_connection(conn)

    def _insert_individually(self, conn, predictions):
        """Fallback: insert predictions one by one"""
        result_ids = []
        for pred in predictions:
            try:
                result_id = insert_price_anomaly(conn, pred)
                result_ids.append(result_id)
            except Exception as e:
                self.logger.warning(f"Failed to insert anomaly for product {pred['product_id']}: {e}")
                result_ids.append(None)

        return result_ids
