"""
ML-based Product Clustering using K-Means

Discovers natural product segments based on multiple features:
- Revenue score
- Demand variability
- Trend (growing/declining)
- Seasonality
- Order frequency
- Profit margin

Auto-detects optimal number of clusters using silhouette score.
"""

from datetime import datetime, timedelta
from typing import Optional, Dict, Any, List
import pandas as pd
import numpy as np
from sklearn.cluster import KMeans
from sklearn.preprocessing import StandardScaler
from sklearn.metrics import silhouette_score
import logging

logger = logging.getLogger(__name__)


class ProductClusterer:
    """ML-based product clustering using K-Means"""

    FEATURE_COLUMNS = [
        'revenue_score', 'variability_score', 'trend_score',
        'seasonality_score', 'frequency_score', 'margin_score'
    ]

    def __init__(self):
        self.scaler = StandardScaler()
        self.model = None

    def cluster_all_products(
        self,
        conn,
        days: int = 90,
        min_transactions: int = 5,
        n_clusters: Optional[int] = None
    ) -> Dict[str, Any]:
        """
        Cluster all products using K-Means.

        Args:
            conn: Database connection
            days: Analysis period in days
            min_transactions: Minimum transactions required
            n_clusters: Number of clusters (auto-detect if None)

        Returns:
            Clustering results with products and cluster info
        """
        df = self._load_product_features(conn, days, min_transactions)

        if df.empty or len(df) < 5:
            return {
                "error": "Not enough products with sufficient data",
                "products_found": len(df) if not df.empty else 0,
                "min_required": 5
            }

        # Normalize features
        features = df[self.FEATURE_COLUMNS].values
        features_scaled = self.scaler.fit_transform(features)

        # Determine optimal clusters
        if n_clusters is None:
            n_clusters = self._find_optimal_clusters(features_scaled, max_clusters=min(10, len(df) - 1))

        # Fit K-Means
        self.model = KMeans(n_clusters=n_clusters, random_state=42, n_init=10)
        df['cluster_id'] = self.model.fit_predict(features_scaled)

        # Silhouette score
        overall_silhouette = silhouette_score(features_scaled, df['cluster_id']) if len(df) > n_clusters else 0.0

        # Calculate distance to centroid
        centroids = self.model.cluster_centers_
        distances = []
        for idx in range(len(df)):
            cluster_id = df.iloc[idx]['cluster_id']
            point = features_scaled[idx]
            distances.append(float(np.linalg.norm(point - centroids[cluster_id])))
        df['distance_to_centroid'] = distances

        # Name clusters
        cluster_info = self._analyze_clusters(df, n_clusters)
        df['cluster_name'] = df['cluster_id'].map(lambda x: cluster_info[x]['name'])

        return self._build_result(df, cluster_info, n_clusters, overall_silhouette, centroids, days)

    def _load_product_features(self, conn, days: int, min_transactions: int) -> pd.DataFrame:
        """Load and compute product features"""
        start_date = datetime.now() - timedelta(days=days)

        query = """
        WITH daily_sales AS (
            SELECT
                p.id_pro as product_id,
                p.name_pro as product_name,
                p.buying_price_pro,
                pp.price_prp as selling_price,
                DATE(o.order_date_ord) as sale_date,
                SUM(lor.quantity_lor) as daily_quantity,
                SUM(lor.line_total_lor) as daily_revenue
            FROM products_pro p
            JOIN line_order_lor lor ON p.id_pro = lor.product_id_lor
            JOIN order_ord o ON lor.order_id_lor = o.id_ord
            LEFT JOIN productprices_prp pp ON p.id_pro = pp.product_ref_prp
            WHERE o.order_date_ord >= %s
            GROUP BY p.id_pro, p.name_pro, p.buying_price_pro, pp.price_prp, DATE(o.order_date_ord)
        ),
        product_stats AS (
            SELECT
                product_id, product_name, buying_price_pro, selling_price,
                COUNT(DISTINCT sale_date) as days_with_sales,
                COUNT(*) as num_orders,
                SUM(daily_quantity) as total_units,
                SUM(daily_revenue) as total_revenue,
                AVG(daily_quantity) as avg_daily_demand,
                STDDEV(daily_quantity) as std_daily_demand,
                MIN(sale_date) as first_sale,
                MAX(sale_date) as last_sale
            FROM daily_sales
            GROUP BY product_id, product_name, buying_price_pro, selling_price
            HAVING COUNT(*) >= %s
        )
        SELECT *, CASE WHEN avg_daily_demand > 0 THEN std_daily_demand / avg_daily_demand ELSE 0 END as cv
        FROM product_stats ORDER BY total_revenue DESC
        """

        with conn.cursor() as cur:
            cur.execute(query, (start_date, min_transactions))
            columns = [desc[0] for desc in cur.description]
            rows = cur.fetchall()

        if not rows:
            return pd.DataFrame()

        df = pd.DataFrame(rows, columns=columns)
        return self._calculate_feature_scores(df, days)

    def _calculate_feature_scores(self, df: pd.DataFrame, days: int) -> pd.DataFrame:
        """Calculate normalized feature scores (0-1)"""
        # Revenue score
        max_revenue = df['total_revenue'].max()
        df['revenue_score'] = df['total_revenue'] / max_revenue if max_revenue > 0 else 0

        # Variability score
        df['cv'] = df['cv'].fillna(0)
        max_cv = df['cv'].max()
        df['variability_score'] = (df['cv'] / max_cv if max_cv > 0 else 0).clip(0, 1)

        # Trend score (based on recency)
        df['first_sale'] = pd.to_datetime(df['first_sale'])
        df['last_sale'] = pd.to_datetime(df['last_sale'])
        df['days_active'] = (df['last_sale'] - df['first_sale']).dt.days + 1
        df['activity_ratio'] = df['days_with_sales'] / df['days_active'].clip(lower=1)
        days_since_last = (pd.Timestamp.now() - df['last_sale']).dt.days
        df['recency_score'] = 1 - (days_since_last / days).clip(0, 1)
        df['trend_score'] = (df['activity_ratio'] * 0.5 + df['recency_score'] * 0.5).clip(0, 1)

        # Seasonality score
        df['seasonality_score'] = (df['variability_score'] * df['revenue_score']).clip(0, 1)

        # Frequency score
        max_orders = df['num_orders'].max()
        df['frequency_score'] = df['num_orders'] / max_orders if max_orders > 0 else 0

        # Margin score
        df['margin'] = (df['selling_price'] - df['buying_price_pro']) / df['selling_price'].replace(0, 1)
        df['margin'] = df['margin'].fillna(0.3)
        df['margin_score'] = df['margin'].clip(0, 1)

        return df

    def _find_optimal_clusters(self, features: np.ndarray, max_clusters: int = 10) -> int:
        """Find optimal clusters using silhouette score"""
        if len(features) < 3:
            return 2

        max_clusters = min(max_clusters, len(features) - 1)
        if max_clusters < 2:
            return 2

        best_score, best_k = -1, 3

        for k in range(2, max_clusters + 1):
            kmeans = KMeans(n_clusters=k, random_state=42, n_init=10)
            labels = kmeans.fit_predict(features)
            if len(set(labels)) >= 2:
                score = silhouette_score(features, labels)
                if score > best_score:
                    best_score, best_k = score, k

        return best_k

    def _analyze_clusters(self, df: pd.DataFrame, n_clusters: int) -> Dict[int, Dict[str, Any]]:
        """Analyze clusters and assign names"""
        cluster_info = {}

        for cluster_id in range(n_clusters):
            cluster_df = df[df['cluster_id'] == cluster_id]

            if cluster_df.empty:
                cluster_info[cluster_id] = {'name': f'Cluster {cluster_id}', 'size': 0, 'characteristics': [], 'strategy': 'Empty cluster'}
                continue

            # Averages
            avg_revenue = cluster_df['revenue_score'].mean()
            avg_variability = cluster_df['variability_score'].mean()
            avg_trend = cluster_df['trend_score'].mean()

            # Determine characteristics
            characteristics = []
            if avg_revenue > 0.7:
                revenue_level = "high"
                characteristics.append("High revenue")
            elif avg_revenue > 0.3:
                revenue_level = "medium"
                characteristics.append("Medium revenue")
            else:
                revenue_level = "low"
                characteristics.append("Low revenue")

            if avg_variability > 0.6:
                variability_level = "variable"
                characteristics.append("Highly variable demand")
            elif avg_variability < 0.3:
                variability_level = "stable"
                characteristics.append("Stable demand")
            else:
                variability_level = "moderate"
                characteristics.append("Moderate variability")

            if avg_trend > 0.7:
                characteristics.append("Growing/Active")
            elif avg_trend < 0.3:
                characteristics.append("Declining/Inactive")

            # Assign name and strategy
            if revenue_level == "high" and variability_level == "stable":
                name, strategy = "Cash Cows", "Protect supply chain, automate reordering"
            elif revenue_level == "high" and variability_level == "variable":
                name, strategy = "High-Value Volatile", "High safety stock, daily monitoring"
            elif revenue_level == "high":
                name, strategy = "Rising Stars", "Invest in inventory, expand capacity"
            elif revenue_level == "medium" and variability_level == "stable":
                name, strategy = "Steady Performers", "Standard management, bi-weekly review"
            elif revenue_level == "medium":
                name, strategy = "Moderate Risk", "Weekly monitoring, moderate safety stock"
            elif revenue_level == "low" and variability_level == "stable":
                name, strategy = "Reliable Long Tail", "Bulk ordering, low priority"
            elif revenue_level == "low":
                name, strategy = "Unpredictable Low-Value", "Evaluate necessity, consider discontinuation"
            else:
                name, strategy = f"Segment {cluster_id + 1}", "Review for custom strategy"

            cluster_info[cluster_id] = {
                'name': name,
                'size': len(cluster_df),
                'characteristics': characteristics,
                'strategy': strategy
            }

        return cluster_info

    def _build_result(self, df, cluster_info, n_clusters, silhouette, centroids, days) -> Dict[str, Any]:
        """Build clustering result"""
        products = []
        for _, row in df.iterrows():
            cluster_id = int(row['cluster_id'])
            products.append({
                "product_id": int(row['product_id']),
                "product_name": row['product_name'],
                "cluster_id": cluster_id,
                "cluster_name": cluster_info[cluster_id]['name'],
                "features": {col: round(float(row[col]), 3) for col in self.FEATURE_COLUMNS},
                "distance_to_centroid": round(row['distance_to_centroid'], 4)
            })

        products.sort(key=lambda x: (x['cluster_id'], x['distance_to_centroid']))

        clusters = [
            {
                "cluster_id": cid,
                "name": info['name'],
                "size": info['size'],
                "characteristics": info['characteristics'],
                "strategy": info['strategy']
            }
            for cid, info in cluster_info.items()
        ]

        return {
            "clustering_date": datetime.now().isoformat(),
            "analysis_period_days": days,
            "n_clusters": n_clusters,
            "total_products": len(products),
            "silhouette_score": round(silhouette, 3),
            "clusters": clusters,
            "products": products
        }
