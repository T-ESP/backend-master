"""
Price Suggestion Handler

Generates optimal price suggestions for products and saves results
to the database with notifications for high-confidence suggestions.
"""

from handlers.base_handler import BaseHandler
from models.price_suggester import PriceSuggester
from database.connection import get_connection, return_connection
from database.queries import get_products_with_prices, get_price_history
from database.writers import insert_price_suggestions_batch, insert_notifications_batch, insert_price_suggestion
from config.settings import MODEL_THRESHOLDS


class PriceSuggestionHandler(BaseHandler):
    """Handler for generating price suggestions on schedule"""

    def __init__(self):
        super().__init__()
        self.model = PriceSuggester()
        self.thresholds = MODEL_THRESHOLDS.get("price_suggester", {
            "min_confidence": 0.7
        })

    def run(self):
        """Generate price suggestions for all products"""
        self.logger.info("Starting price suggestion handler")
        conn = get_connection()

        try:
            # Get products and price history
            products = get_products_with_prices(conn)
            if not products:
                self.logger.warning("No products found")
                return

            price_history = get_price_history(conn)

            # Generate suggestions
            predictions = self.model.predict(products, price_history)
            self.logger.info(f"Generated {len(predictions)} price suggestions")

            if not predictions:
                self.logger.info("No suggestions generated")
                return

            # Batch insert suggestion results
            try:
                result_ids = insert_price_suggestions_batch(conn, predictions)
            except Exception as e:
                self.logger.error(f"Failed to batch insert suggestions: {e}")
                # Fallback to individual inserts
                result_ids = self._insert_individually(conn, predictions)

            # Create notifications for high-confidence suggestions
            notifications = []
            high_confidence_count = 0
            min_confidence = self.thresholds.get("min_confidence", 0.7)

            for i, prediction in enumerate(predictions):
                if prediction["confidence"] >= min_confidence:
                    high_confidence_count += 1
                    result_id = result_ids[i] if i < len(result_ids) else None

                    # Determine severity based on price change magnitude
                    price_change = abs(prediction.get("price_change_pct", 0))
                    severity = "HIGH" if price_change > 10 else "MEDIUM"

                    notifications.append({
                        "product_id": prediction["product_id"],
                        "model_type": "price_suggester",
                        "category": "suggestion",
                        "notification_type": "price_suggestion",
                        "severity": severity,
                        "message": f"Suggested price {prediction['suggested_price']:.2f} "
                                   f"(current: {prediction['current_price']:.2f}, "
                                   f"confidence: {prediction['confidence']:.0%})",
                        "action_recommended": prediction.get("reason", "Review suggested price and update if appropriate"),
                        "related_result_id": result_id,
                    })

            # Batch insert notifications
            if notifications:
                try:
                    insert_notifications_batch(conn, notifications)
                except Exception as e:
                    self.logger.error(f"Failed to batch insert notifications: {e}")

            self.logger.info(f"Price suggestion handler completed: {high_confidence_count} high-confidence suggestions")

        except Exception as e:
            self.logger.error(f"Price suggestion handler error: {e}", exc_info=True)
            raise
        finally:
            return_connection(conn)

    def _insert_individually(self, conn, predictions):
        """Fallback: insert predictions one by one"""
        result_ids = []
        for pred in predictions:
            try:
                result_id = insert_price_suggestion(conn, pred)
                result_ids.append(result_id)
            except Exception as e:
                self.logger.warning(f"Failed to insert suggestion for product {pred['product_id']}: {e}")
                result_ids.append(None)

        return result_ids
