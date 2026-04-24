"""
ABC-XYZ Classification Handler

Runs product classification and saves results to the database
with notifications for products that changed classification.
"""

from handlers.base_handler import BaseHandler
from models.abc_xyz_classifier import ABCXYZClassifier
from database.connection import get_connection, return_connection
from database.writers import insert_classification, insert_notification


class ClassificationHandler(BaseHandler):
    """Handler for running ABC-XYZ classification on schedule"""

    def __init__(self):
        super().__init__()
        self.model = ABCXYZClassifier()

    def run(self):
        """Run ABC-XYZ classification for all products"""
        self.logger.info("Starting ABC-XYZ classification handler")
        conn = get_connection()

        try:
            # Classify all products
            result = self.model.classify_all_products(
                conn,
                days_lookback=90,
                min_transactions=5
            )

            if result.get("status") == "no_data":
                self.logger.warning("No products found for classification")
                return

            products = result.get("products", [])
            self.logger.info(f"Classified {len(products)} products")

            # Get previous classifications for comparison
            previous = self._get_previous_classifications(conn)

            for product in products:
                product_id = product["product_id"]

                # Save classification
                classification_id = insert_classification(conn, {
                    "product_id": product_id,
                    "abc_class": product["abc_class"],
                    "xyz_class": product["xyz_class"],
                    "combined_class": product["combined_class"],
                    "total_revenue": product["total_revenue"],
                    "revenue_contribution_pct": product["revenue_contribution_pct"],
                    "total_units_sold": product["total_units_sold"],
                    "coefficient_of_variation": product["coefficient_of_variation"],
                    "strategy": product["recommendation"]["strategy"],
                    "priority": product["recommendation"]["priority"]
                })

                # Check for class changes (important for inventory strategy)
                prev_class = previous.get(product_id)
                if prev_class and prev_class != product["combined_class"]:
                    # Class changed - notify
                    if product["abc_class"] == "A" or prev_class.startswith("A"):
                        # High-value product class change
                        insert_notification(conn, {
                            "product_id": product_id,
                            "model_type": "abc_xyz_classifier",
                            "category": "alert",
                            "notification_type": "classification_change",
                            "severity": "MEDIUM",
                            "message": f"Product classification changed from {prev_class} to {product['combined_class']}",
                            "action_recommended": f"Review inventory strategy: {product['recommendation']['strategy']}",
                            "related_result_id": classification_id
                        })

            summary = result.get("summary", {})
            self.logger.info(
                f"Classification complete: A={summary.get('abc_distribution', {}).get('A', 0)}, "
                f"B={summary.get('abc_distribution', {}).get('B', 0)}, "
                f"C={summary.get('abc_distribution', {}).get('C', 0)}"
            )

        except Exception as e:
            self.logger.error(f"Classification handler error: {e}", exc_info=True)
            raise
        finally:
            return_connection(conn)

    def _get_previous_classifications(self, conn) -> dict:
        """Get the most recent classification for each product"""
        query = """
        SELECT DISTINCT ON (product_id)
            product_id, combined_class
        FROM product_classifications
        WHERE created_at < CURRENT_DATE
        ORDER BY product_id, created_at DESC;
        """

        try:
            with conn.cursor() as cur:
                cur.execute(query)
                rows = cur.fetchall()
            return {row[0]: row[1] for row in rows}
        except Exception:
            # Table might not exist yet
            return {}
