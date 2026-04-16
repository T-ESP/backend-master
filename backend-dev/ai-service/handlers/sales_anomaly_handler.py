"""
Sales Anomaly Handler

Runs sales anomaly detection for all products and saves results
to the database with notifications for detected anomalies.
"""

from handlers.base_handler import BaseHandler
from models.sales_anomaly_detector import SalesAnomalyDetector
from database.connection import get_connection, return_connection
from database.queries import get_sales_data
from database.writers import insert_sales_anomalies_batch, insert_notifications_batch, insert_sales_anomaly
from config.settings import MODEL_THRESHOLDS


class SalesAnomalyHandler(BaseHandler):
    """Handler for detecting sales anomalies on schedule"""

    def __init__(self):
        super().__init__()
        self.model = SalesAnomalyDetector()
        self.thresholds = MODEL_THRESHOLDS.get("sales_anomaly", {
            "anomaly_score_threshold": -0.5,
            "high_severity_threshold": -0.8
        })

    def run(self):
        """Run sales anomaly detection for all products"""
        self.logger.info("Starting sales anomaly handler")
        conn = get_connection()

        try:
            # Get sales data
            sales_data = get_sales_data(conn)
            if not sales_data:
                self.logger.warning("No sales data found")
                return

            # Run anomaly detection
            predictions = self.model.predict(sales_data)
            self.logger.info(f"Analyzed {len(predictions)} sales records")

            if not predictions:
                self.logger.info("No predictions generated")
                return

            # Batch insert anomaly results
            try:
                result_ids = insert_sales_anomalies_batch(conn, predictions)
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
                    severity = (
                        "HIGH" if score <= self.thresholds.get("high_severity_threshold", -0.8)
                        else "MEDIUM"
                    )
                    result_id = result_ids[i] if i < len(result_ids) else None

                    notifications.append({
                        "product_id": prediction["product_id"],
                        "model_type": "sales_anomaly",
                        "category": "alert",
                        "notification_type": "sales_anomaly_detected",
                        "severity": severity,
                        "message": f"Sales anomaly detected: volume {prediction['sales_volume']}, "
                                   f"expected {prediction['expected_sales']:.1f} "
                                   f"(score: {score:.2f})",
                        "action_recommended": "Review sales patterns for this product",
                        "related_result_id": result_id,
                    })

            # Batch insert notifications
            if notifications:
                try:
                    insert_notifications_batch(conn, notifications)
                except Exception as e:
                    self.logger.error(f"Failed to batch insert notifications: {e}")

            self.logger.info(f"Sales anomaly handler completed: {anomaly_count} anomalies detected")

        except Exception as e:
            self.logger.error(f"Sales anomaly handler error: {e}", exc_info=True)
            raise
        finally:
            return_connection(conn)

    def _insert_individually(self, conn, predictions):
        """Fallback: insert predictions one by one"""
        result_ids = []
        for pred in predictions:
            try:
                result_id = insert_sales_anomaly(conn, pred)
                result_ids.append(result_id)
            except Exception as e:
                self.logger.warning(f"Failed to insert anomaly for product {pred['product_id']}: {e}")
                result_ids.append(None)

        return result_ids
