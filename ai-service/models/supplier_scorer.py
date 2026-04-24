"""
Supplier Performance Scoring

Evaluates suppliers across multiple dimensions:
- Delivery Performance (40%): On-time delivery rate
- Quality Score (25%): Defect/cancellation rate
- Lead Time (20%): Consistency and speed
- Fulfillment Rate (15%): Order completion rate

Generates overall score (0-100) with ratings and recommendations.
"""

from datetime import datetime, timedelta
from typing import Dict, Any, List, Optional
import numpy as np
import logging

logger = logging.getLogger(__name__)


class SupplierScorer:
    """Supplier performance scoring engine"""

    WEIGHTS = {
        'delivery_performance': 0.40,
        'quality_score': 0.25,
        'lead_time_score': 0.20,
        'fulfillment_rate': 0.15
    }

    def score_all_suppliers(
        self,
        conn,
        days_lookback: int = 90,
        min_restocks: int = 3
    ) -> Dict[str, Any]:
        """Score all suppliers based on performance metrics"""
        suppliers = self._load_supplier_data(conn, days_lookback, min_restocks)

        if not suppliers:
            return {
                "status": "no_data",
                "message": "No suppliers found with sufficient restock history",
                "suppliers": [],
                "summary": {}
            }

        scored = [self._calculate_score(s) for s in suppliers]
        scored.sort(key=lambda x: x['overall_score'], reverse=True)

        return {
            "scoring_date": datetime.now().isoformat(),
            "analysis_period_days": days_lookback,
            "suppliers": scored,
            "summary": self._build_summary(scored)
        }

    def score_single_supplier(self, conn, supplier_id: int, days_lookback: int = 90) -> Dict[str, Any]:
        """Score a single supplier"""
        data = self._load_single_supplier(conn, supplier_id, days_lookback)

        if not data:
            raise ValueError(f"Supplier {supplier_id} not found or has no restocks in last {days_lookback} days")

        score = self._calculate_score(data)
        return {
            "supplier_id": supplier_id,
            "supplier_name": data['name'],
            "scoring_date": datetime.now().isoformat(),
            "analysis_period_days": days_lookback,
            **score
        }

    def _load_supplier_data(self, conn, days_lookback: int, min_restocks: int) -> List[Dict[str, Any]]:
        """Load restock data for all suppliers"""
        cutoff_date = datetime.now() - timedelta(days=days_lookback)

        query = """
        SELECT
            s.id_sup as supplier_id,
            s.name_sup as supplier_name,
            COUNT(DISTINCT r.id_res) as total_restocks,
            SUM(CASE WHEN r.status_res = 'received' THEN 1 ELSE 0 END) as received_count,
            SUM(CASE WHEN r.status_res = 'cancelled' THEN 1 ELSE 0 END) as cancelled_count,
            ARRAY_AGG(
                CASE WHEN r.status_res = 'received'
                THEN EXTRACT(EPOCH FROM (r.restock_date_res - r.created_at)) / 86400
                ELSE NULL END
            ) as lead_times
        FROM supplier_sup s
        JOIN restock_res r ON s.id_sup = r.supplier_id_res
        WHERE r.created_at >= %s
        GROUP BY s.id_sup, s.name_sup
        HAVING COUNT(DISTINCT r.id_res) >= %s
        ORDER BY s.id_sup;
        """

        with conn.cursor() as cur:
            cur.execute(query, (cutoff_date, min_restocks))
            rows = cur.fetchall()

        suppliers = []
        for row in rows:
            lead_times = [float(lt) for lt in (row[5] or []) if lt is not None]
            suppliers.append({
                'id': row[0],
                'name': row[1],
                'total_restocks': int(row[2]),
                'received_count': int(row[3] or 0),
                'cancelled_count': int(row[4] or 0),
                'lead_times': lead_times
            })

        return suppliers

    def _load_single_supplier(self, conn, supplier_id: int, days_lookback: int) -> Optional[Dict[str, Any]]:
        """Load restock data for a single supplier"""
        cutoff_date = datetime.now() - timedelta(days=days_lookback)

        query = """
        SELECT
            s.id_sup, s.name_sup,
            COUNT(DISTINCT r.id_res) as total_restocks,
            SUM(CASE WHEN r.status_res = 'received' THEN 1 ELSE 0 END) as received_count,
            SUM(CASE WHEN r.status_res = 'cancelled' THEN 1 ELSE 0 END) as cancelled_count,
            ARRAY_AGG(
                CASE WHEN r.status_res = 'received'
                THEN EXTRACT(EPOCH FROM (r.restock_date_res - r.created_at)) / 86400
                ELSE NULL END
            ) as lead_times
        FROM supplier_sup s
        JOIN restock_res r ON s.id_sup = r.supplier_id_res
        WHERE s.id_sup = %s AND r.created_at >= %s
        GROUP BY s.id_sup, s.name_sup;
        """

        with conn.cursor() as cur:
            cur.execute(query, (supplier_id, cutoff_date))
            row = cur.fetchone()

        if not row:
            return None

        lead_times = [float(lt) for lt in (row[5] or []) if lt is not None]
        return {
            'id': row[0],
            'name': row[1],
            'total_restocks': int(row[2]),
            'received_count': int(row[3] or 0),
            'cancelled_count': int(row[4] or 0),
            'lead_times': lead_times
        }

    def _calculate_score(self, data: Dict[str, Any]) -> Dict[str, Any]:
        """Calculate comprehensive supplier score"""
        delivery_score, delivery_metrics = self._calc_delivery(data)
        quality_score, quality_metrics = self._calc_quality(data)
        lead_time_score, lead_time_metrics = self._calc_lead_time(data)
        fulfillment_score, fulfillment_metrics = self._calc_fulfillment(data)

        overall = (
            delivery_score * self.WEIGHTS['delivery_performance'] +
            quality_score * self.WEIGHTS['quality_score'] +
            lead_time_score * self.WEIGHTS['lead_time_score'] +
            fulfillment_score * self.WEIGHTS['fulfillment_rate']
        )

        rating, recommendation = self._generate_recommendation(
            overall, delivery_score, quality_score, lead_time_score, fulfillment_score
        )

        return {
            'supplier_id': data['id'],
            'supplier_name': data['name'],
            'overall_score': round(overall, 2),
            'rating': rating,
            'scores': {
                'delivery_performance': round(delivery_score, 2),
                'quality_score': round(quality_score, 2),
                'lead_time_score': round(lead_time_score, 2),
                'fulfillment_rate': round(fulfillment_score, 2)
            },
            'metrics': {
                'delivery': delivery_metrics,
                'quality': quality_metrics,
                'lead_time': lead_time_metrics,
                'fulfillment': fulfillment_metrics
            },
            'recommendation': recommendation
        }

    def _calc_delivery(self, data: Dict[str, Any]) -> tuple:
        """Calculate delivery performance score"""
        total = data['total_restocks']
        received = data['received_count']

        if total == 0:
            return 0.0, {'on_time_count': 0, 'total_deliveries': 0, 'on_time_pct': 0}

        on_time_pct = (received / total) * 100
        return min(100, on_time_pct), {
            'on_time_count': received,
            'total_deliveries': total,
            'on_time_pct': round(on_time_pct, 2)
        }

    def _calc_quality(self, data: Dict[str, Any]) -> tuple:
        """Calculate quality score (based on cancellations)"""
        total = data['total_restocks']
        cancelled = data['cancelled_count']

        if total == 0:
            return 100.0, {'defect_count': 0, 'total_orders': 0, 'quality_pct': 100}

        quality_pct = 100 - (cancelled / total) * 100
        return max(0, quality_pct), {
            'defect_count': cancelled,
            'total_orders': total,
            'quality_pct': round(quality_pct, 2)
        }

    def _calc_lead_time(self, data: Dict[str, Any]) -> tuple:
        """Calculate lead time consistency score"""
        lead_times = [float(lt) for lt in data['lead_times']]

        if not lead_times or len(lead_times) < 2:
            return 50.0, {'avg_lead_time_days': 0, 'std_dev_days': 0, 'consistency_score': 50}

        avg = float(np.mean(lead_times))
        std = float(np.std(lead_times))

        # Lead time component (lower is better)
        if avg <= 3:
            lead_component = 100
        elif avg >= 14:
            lead_component = 50
        else:
            lead_component = 100 - ((avg - 3) / 11) * 50

        # Consistency component
        cv = std / avg if avg > 0 else 0
        if cv <= 0.2:
            consistency = 100
        elif cv >= 0.5:
            consistency = 50
        else:
            consistency = 100 - ((cv - 0.2) / 0.3) * 50

        score = lead_component * 0.6 + consistency * 0.4

        return score, {
            'avg_lead_time_days': round(avg, 2),
            'std_dev_days': round(std, 2),
            'consistency_score': round(consistency, 2)
        }

    def _calc_fulfillment(self, data: Dict[str, Any]) -> tuple:
        """Calculate fulfillment rate"""
        total = data['total_restocks']
        received = data['received_count']

        if total == 0:
            return 100.0, {'fulfilled_count': 0, 'total_orders': 0, 'fulfillment_pct': 100}

        pct = (received / total) * 100
        return pct, {
            'fulfilled_count': received,
            'total_orders': total,
            'fulfillment_pct': round(pct, 2)
        }

    def _generate_recommendation(self, overall, delivery, quality, lead_time, fulfillment) -> tuple:
        """Generate rating and recommendations"""
        if overall >= 90:
            rating = 'EXCELLENT'
            status = 'preferred_supplier'
        elif overall >= 75:
            rating = 'GOOD'
            status = 'reliable_supplier'
        elif overall >= 60:
            rating = 'ACCEPTABLE'
            status = 'monitor_closely'
        elif overall >= 40:
            rating = 'POOR'
            status = 'needs_improvement'
        else:
            rating = 'UNACCEPTABLE'
            status = 'consider_replacing'

        actions = []
        if delivery < 80:
            actions.append('Improve on-time delivery')
        if quality < 80:
            actions.append('Reduce cancellations')
        if lead_time < 70:
            actions.append('Improve lead time consistency')
        if fulfillment < 90:
            actions.append('Increase fulfillment rate')

        if overall >= 90:
            actions = ['Maintain performance', 'Negotiate better terms', 'Consider strategic partnership']
        elif overall < 60:
            actions.append('Develop backup suppliers')

        priority_map = {
            'EXCELLENT': 'MAINTAIN', 'GOOD': 'MONITOR', 'ACCEPTABLE': 'REVIEW',
            'POOR': 'ACTION_REQUIRED', 'UNACCEPTABLE': 'URGENT_ACTION'
        }

        return rating, {
            'status': status,
            'priority': priority_map.get(rating, 'MONITOR'),
            'summary': f'Supplier performance: {overall:.1f}/100 - {rating}',
            'actions': actions
        }

    def _build_summary(self, scored: List[Dict[str, Any]]) -> Dict[str, Any]:
        """Build summary statistics"""
        rating_dist = {}
        for s in scored:
            rating_dist[s['rating']] = rating_dist.get(s['rating'], 0) + 1

        avg_overall = np.mean([s['overall_score'] for s in scored])
        avg_delivery = np.mean([s['scores']['delivery_performance'] for s in scored])
        avg_quality = np.mean([s['scores']['quality_score'] for s in scored])

        return {
            'total_suppliers_scored': len(scored),
            'rating_distribution': rating_dist,
            'average_scores': {
                'overall': round(avg_overall, 2),
                'delivery_performance': round(avg_delivery, 2),
                'quality': round(avg_quality, 2)
            },
            'top_performers': [
                {'supplier_id': s['supplier_id'], 'supplier_name': s['supplier_name'], 'overall_score': s['overall_score'], 'rating': s['rating']}
                for s in scored[:3]
            ],
            'needs_attention': [
                {'supplier_id': s['supplier_id'], 'supplier_name': s['supplier_name'], 'overall_score': s['overall_score'], 'rating': s['rating']}
                for s in scored[-3:]
            ]
        }
