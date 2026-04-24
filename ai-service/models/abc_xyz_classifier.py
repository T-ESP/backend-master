"""
ABC-XYZ Product Classification

ABC Analysis: Classifies products by revenue importance (Pareto principle)
- A: Top 20% products → 80% revenue (high value)
- B: Next 30% products → 15% revenue (moderate value)
- C: Remaining 50% products → 5% revenue (low value)

XYZ Analysis: Classifies products by demand variability
- X: Low variability (CV < 0.5) - predictable demand
- Y: Medium variability (0.5 ≤ CV < 1.0) - moderate fluctuation
- Z: High variability (CV ≥ 1.0) - unpredictable demand

Combined: 9 categories for inventory optimization strategy.
"""

from datetime import datetime, timedelta
from typing import Dict, Any, List
import pandas as pd
import numpy as np
import logging

logger = logging.getLogger(__name__)


class ABCXYZClassifier:
    """Product classification engine using ABC-XYZ methodology"""

    # Strategy recommendations for each ABC-XYZ class
    RECOMMENDATIONS = {
        'AX': {
            'strategy': 'Just-In-Time with Safety Stock',
            'priority': 'CRITICAL',
            'actions': ['Automate reordering', 'Negotiate favorable terms', 'Maintain 5-7 days safety stock']
        },
        'AY': {
            'strategy': 'Regular Review with Moderate Safety Stock',
            'priority': 'CRITICAL',
            'actions': ['Maintain 10-14 days safety stock', 'Weekly demand review', 'Backup suppliers']
        },
        'AZ': {
            'strategy': 'High Safety Stock and Continuous Monitoring',
            'priority': 'CRITICAL',
            'actions': ['Maintain 20-30 days safety stock', 'Daily monitoring', 'Multiple suppliers']
        },
        'BX': {
            'strategy': 'Standard Periodic Review',
            'priority': 'HIGH',
            'actions': ['Bi-weekly review', 'Standard reorder quantities', '7-10 days safety stock']
        },
        'BY': {
            'strategy': 'Periodic Review with Moderate Safety Stock',
            'priority': 'HIGH',
            'actions': ['Weekly review', 'Consider seasonal patterns', '10-15 days safety stock']
        },
        'BZ': {
            'strategy': 'Careful Monitoring with High Safety Stock',
            'priority': 'MEDIUM',
            'actions': ['Weekly monitoring', 'Analyze demand patterns', '15-20 days safety stock']
        },
        'CX': {
            'strategy': 'Bulk Ordering, Low Priority',
            'priority': 'LOW',
            'actions': ['Monthly review', 'Bulk orders', 'Minimal safety stock (3-5 days)']
        },
        'CY': {
            'strategy': 'Simple Replenishment',
            'priority': 'LOW',
            'actions': ['Monthly review', 'Simple reorder point', '5-7 days safety stock']
        },
        'CZ': {
            'strategy': 'Evaluate Discontinuation',
            'priority': 'LOW',
            'actions': ['Evaluate necessity', 'Consider discontinuing', 'No safety stock']
        }
    }

    def classify_all_products(
        self,
        conn,
        days_lookback: int = 90,
        min_transactions: int = 5
    ) -> Dict[str, Any]:
        """Classify all products using ABC-XYZ methodology"""
        df = self._load_product_sales_data(conn, days_lookback, min_transactions)

        if df.empty:
            return {
                "status": "no_data",
                "message": "No products found with sufficient transaction history",
                "products": [],
                "summary": {}
            }

        # Calculate classifications
        df = self._calculate_abc(df)
        df = self._calculate_xyz(df)
        df['combined_class'] = df['abc_class'] + df['xyz_class']
        df['recommendation'] = df['combined_class'].map(
            lambda x: self.RECOMMENDATIONS.get(x, {'strategy': 'Unknown', 'priority': 'MEDIUM', 'actions': []})
        )

        return self._build_result(df, days_lookback)

    def classify_single_product(self, conn, product_id: int, days_lookback: int = 90) -> Dict[str, Any]:
        """Classify a single product"""
        # Need all products to determine relative ABC class
        df = self._load_product_sales_data(conn, days_lookback, min_transactions=1)

        if df.empty or product_id not in df['product_id'].values:
            raise ValueError(f"Product {product_id} not found or has no sales in last {days_lookback} days")

        df = self._calculate_abc(df)
        df = self._calculate_xyz(df)
        df['combined_class'] = df['abc_class'] + df['xyz_class']

        product_row = df[df['product_id'] == product_id].iloc[0]
        combined_class = product_row['combined_class']
        recommendation = self.RECOMMENDATIONS.get(combined_class, {})

        return {
            'product_id': product_id,
            'product_name': product_row['product_name'],
            'abc_class': product_row['abc_class'],
            'xyz_class': product_row['xyz_class'],
            'combined_class': combined_class,
            'total_revenue': round(float(product_row['total_revenue']), 2),
            'revenue_contribution_pct': round(float(product_row['revenue_pct']), 2),
            'total_units_sold': int(product_row['total_units_sold']),
            'coefficient_of_variation': round(float(product_row['cv']), 3),
            'recommendation': recommendation
        }

    def _load_product_sales_data(self, conn, days_lookback: int, min_transactions: int) -> pd.DataFrame:
        """Load sales data for all products"""
        cutoff_date = datetime.now() - timedelta(days=days_lookback)

        query = """
        SELECT
            p.id_pro as product_id,
            p.name_pro as product_name,
            COUNT(DISTINCT o.id_ord) as num_transactions,
            SUM(lor.quantity_lor) as total_units_sold,
            SUM(lor.line_total_lor) as total_revenue,
            ARRAY_AGG(lor.quantity_lor ORDER BY o.order_date_ord) as quantities
        FROM products_pro p
        JOIN line_order_lor lor ON p.id_pro = lor.product_id_lor
        JOIN order_ord o ON lor.order_id_lor = o.id_ord
        WHERE o.order_date_ord >= %s
        GROUP BY p.id_pro, p.name_pro
        HAVING COUNT(DISTINCT o.id_ord) >= %s
        ORDER BY total_revenue DESC;
        """

        with conn.cursor() as cur:
            cur.execute(query, (cutoff_date, min_transactions))
            rows = cur.fetchall()

        if not rows:
            return pd.DataFrame()

        data = []
        for row in rows:
            quantities = row[5] if row[5] else []
            cv = np.std(quantities) / np.mean(quantities) if quantities and np.mean(quantities) > 0 else 999
            data.append({
                'product_id': row[0],
                'product_name': row[1],
                'num_transactions': row[2],
                'total_units_sold': row[3],
                'total_revenue': float(row[4]),
                'cv': cv
            })

        return pd.DataFrame(data)

    def _calculate_abc(self, df: pd.DataFrame) -> pd.DataFrame:
        """Calculate ABC classification (Pareto principle)"""
        df = df.sort_values('total_revenue', ascending=False).copy()
        df['cumulative_revenue'] = df['total_revenue'].cumsum()
        total_revenue = df['total_revenue'].sum()
        df['revenue_pct'] = (df['total_revenue'] / total_revenue) * 100
        df['cumulative_revenue_pct'] = (df['cumulative_revenue'] / total_revenue) * 100

        def assign_abc(row):
            if row['cumulative_revenue_pct'] <= 80:
                return 'A'
            elif row['cumulative_revenue_pct'] <= 95:
                return 'B'
            return 'C'

        df['abc_class'] = df.apply(assign_abc, axis=1)
        return df

    def _calculate_xyz(self, df: pd.DataFrame) -> pd.DataFrame:
        """Calculate XYZ classification (variability)"""
        def assign_xyz(cv):
            if cv < 0.5:
                return 'X'
            elif cv < 1.0:
                return 'Y'
            return 'Z'

        df['xyz_class'] = df['cv'].apply(assign_xyz)
        return df

    def _build_result(self, df: pd.DataFrame, days_lookback: int) -> Dict[str, Any]:
        """Build classification result"""
        total_revenue = df['total_revenue'].sum()

        # Distributions
        abc_dist = df['abc_class'].value_counts().to_dict()
        xyz_dist = df['xyz_class'].value_counts().to_dict()
        abc_revenue = df.groupby('abc_class')['total_revenue'].sum().to_dict()

        # Matrix
        matrix = df.groupby(['abc_class', 'xyz_class']).size().to_dict()
        matrix = {f"{k[0]}{k[1]}": v for k, v in matrix.items()}

        products = []
        for _, row in df.iterrows():
            products.append({
                'product_id': int(row['product_id']),
                'product_name': row['product_name'],
                'abc_class': row['abc_class'],
                'xyz_class': row['xyz_class'],
                'combined_class': row['combined_class'],
                'total_revenue': round(row['total_revenue'], 2),
                'revenue_contribution_pct': round(row['revenue_pct'], 2),
                'total_units_sold': int(row['total_units_sold']),
                'coefficient_of_variation': round(row['cv'], 3),
                'recommendation': row['recommendation']
            })

        return {
            'classification_date': datetime.now().isoformat(),
            'analysis_period_days': days_lookback,
            'summary': {
                'total_products_classified': len(df),
                'total_revenue_analyzed': round(total_revenue, 2),
                'abc_distribution': {'A': abc_dist.get('A', 0), 'B': abc_dist.get('B', 0), 'C': abc_dist.get('C', 0)},
                'abc_revenue_pct': {
                    'A': round((abc_revenue.get('A', 0) / total_revenue) * 100, 2) if total_revenue > 0 else 0,
                    'B': round((abc_revenue.get('B', 0) / total_revenue) * 100, 2) if total_revenue > 0 else 0,
                    'C': round((abc_revenue.get('C', 0) / total_revenue) * 100, 2) if total_revenue > 0 else 0
                },
                'xyz_distribution': {'X': xyz_dist.get('X', 0), 'Y': xyz_dist.get('Y', 0), 'Z': xyz_dist.get('Z', 0)},
                'abc_xyz_matrix': matrix
            },
            'products': products
        }
