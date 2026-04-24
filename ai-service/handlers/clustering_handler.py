"""
Product Clustering Handler

Runs ML-based product clustering and saves results to the database.
"""

from handlers.base_handler import BaseHandler
from models.product_clusterer import ProductClusterer
from database.connection import get_connection, return_connection
from database.writers import insert_cluster_result, insert_notification


class ClusteringHandler(BaseHandler):
    """Handler for running product clustering on schedule"""

    def __init__(self):
        super().__init__()
        self.model = ProductClusterer()

    def run(self):
        """Run K-Means clustering for all products"""
        self.logger.info("Starting product clustering handler")
        conn = get_connection()

        try:
            # Cluster all products
            result = self.model.cluster_all_products(
                conn,
                days=90,
                min_transactions=5,
                n_clusters=None  # Auto-detect
            )

            if result.get("error"):
                self.logger.warning(f"Clustering failed: {result.get('error')}")
                return

            products = result.get("products", [])
            clusters = result.get("clusters", [])

            self.logger.info(
                f"Clustered {len(products)} products into {result.get('n_clusters')} clusters "
                f"(silhouette: {result.get('silhouette_score', 0):.3f})"
            )

            # Save each product's cluster assignment
            inserted = 0
            for product in products:
                try:
                    insert_cluster_result(conn, {
                        "product_id": product["product_id"],
                        "cluster_id": product["cluster_id"],
                        "cluster_name": product["cluster_name"],
                        "revenue_score": product["features"]["revenue_score"],
                        "variability_score": product["features"]["variability_score"],
                        "trend_score": product["features"]["trend_score"],
                        "seasonality_score": product["features"]["seasonality_score"],
                        "frequency_score": product["features"]["frequency_score"],
                        "margin_score": product["features"]["margin_score"],
                        "distance_to_centroid": product["distance_to_centroid"],
                        "n_clusters": result.get("n_clusters"),
                        "silhouette_score": result.get("silhouette_score")
                    })
                    inserted += 1
                except Exception as e:
                    self.logger.warning(f"Failed to insert cluster result for product {product.get('product_id')}: {e}")
                    try:
                        conn.rollback()
                    except Exception:
                        pass

            self.logger.info(f"Inserted {inserted}/{len(products)} cluster results")

            # Log cluster summary
            for cluster in clusters:
                self.logger.info(
                    f"  Cluster {cluster['cluster_id']} ({cluster['name']}): "
                    f"{cluster['size']} products - {cluster['strategy']}"
                )

        except Exception as e:
            self.logger.error(f"Clustering handler error: {e}", exc_info=True)
            raise
        finally:
            return_connection(conn)
