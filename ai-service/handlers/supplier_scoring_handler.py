"""
Supplier Scoring Handler

Runs supplier performance scoring and saves results to the database
with notifications for poor-performing suppliers.
"""

from handlers.base_handler import BaseHandler
from models.supplier_scorer import SupplierScorer
from database.connection import get_connection, return_connection
from database.writers import insert_supplier_score, insert_notification


class SupplierScoringHandler(BaseHandler):
    """Handler for running supplier scoring on schedule"""

    def __init__(self):
        super().__init__()
        self.model = SupplierScorer()

    def run(self):
        """Run supplier scoring for all suppliers"""
        self.logger.info("Starting supplier scoring handler")
        conn = get_connection()

        try:
            # Score all suppliers
            result = self.model.score_all_suppliers(
                conn,
                days_lookback=90,
                min_restocks=3
            )

            if result.get("status") == "no_data":
                self.logger.warning("No suppliers found for scoring")
                return

            suppliers = result.get("suppliers", [])
            self.logger.info(f"Scored {len(suppliers)} suppliers")

            inserted = 0
            for supplier in suppliers:
                supplier_id = supplier["supplier_id"]
                scores = supplier.get("scores", {})
                metrics = supplier.get("metrics", {})

                # Save score to database
                try:
                    score_id = insert_supplier_score(conn, {
                        "supplier_id": supplier_id,
                        "overall_score": supplier["overall_score"],
                        "delivery_score": scores.get("delivery_performance", 0),
                        "quality_score": scores.get("quality_score", 0),
                        "lead_time_score": scores.get("lead_time_score", 0),
                        "fulfillment_score": scores.get("fulfillment_rate", 0),
                        "rating": supplier["rating"],
                        "total_restocks": metrics.get("delivery", {}).get("total_deliveries", 0)
                    })
                    inserted += 1
                except Exception as e:
                    self.logger.warning(f"Failed to insert supplier score for supplier {supplier_id}: {e}")
                    try:
                        conn.rollback()
                    except Exception:
                        pass
                    continue

                # Create notifications for poor performers
                if supplier["rating"] in ["POOR", "UNACCEPTABLE"]:
                    severity = "CRITICAL" if supplier["rating"] == "UNACCEPTABLE" else "HIGH"
                    recommendation = supplier.get("recommendation", {})

                    try:
                        insert_notification(conn, {
                            "product_id": None,  # Supplier-level notification
                            "model_type": "supplier_scorer",
                            "category": "alert",
                            "notification_type": "supplier_performance",
                            "severity": severity,
                            "message": f"Supplier '{supplier['supplier_name']}' rated {supplier['rating']} "
                                       f"(score: {supplier['overall_score']:.1f}/100)",
                            "action_recommended": "; ".join(recommendation.get("actions", [])),
                            "related_result_id": score_id
                        })
                    except Exception as e:
                        self.logger.warning(f"Failed to insert notification for supplier {supplier_id}: {e}")
                        try:
                            conn.rollback()
                        except Exception:
                            pass

            self.logger.info(f"Inserted {inserted}/{len(suppliers)} supplier scores")

            # Log summary
            summary = result.get("summary", {})
            rating_dist = summary.get("rating_distribution", {})
            self.logger.info(
                f"Scoring complete: EXCELLENT={rating_dist.get('EXCELLENT', 0)}, "
                f"GOOD={rating_dist.get('GOOD', 0)}, ACCEPTABLE={rating_dist.get('ACCEPTABLE', 0)}, "
                f"POOR={rating_dist.get('POOR', 0)}, UNACCEPTABLE={rating_dist.get('UNACCEPTABLE', 0)}"
            )

        except Exception as e:
            self.logger.error(f"Supplier scoring handler error: {e}", exc_info=True)
            raise
        finally:
            return_connection(conn)
