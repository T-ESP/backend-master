"""
Database Writers

Functions for inserting AI model results into the database.
Supports both individual and batch inserts for efficiency.
Includes deadlock retry logic for concurrent access safety.
"""

import time
import logging
from typing import List, Dict, Any
from functools import wraps

import psycopg2.errors

logger = logging.getLogger(__name__)

MAX_DEADLOCK_RETRIES = 3
DEADLOCK_RETRY_DELAY = 2  # seconds


def retry_on_deadlock(func):
    """Decorator that retries a DB write on deadlock (up to MAX_DEADLOCK_RETRIES times)."""
    @wraps(func)
    def wrapper(*args, **kwargs):
        for attempt in range(1, MAX_DEADLOCK_RETRIES + 1):
            try:
                return func(*args, **kwargs)
            except psycopg2.errors.DeadlockDetected as e:
                # Rollback the failed transaction so the connection is usable again
                conn = args[0] if args else kwargs.get("conn")
                if conn:
                    try:
                        conn.rollback()
                    except Exception:
                        pass
                if attempt < MAX_DEADLOCK_RETRIES:
                    logger.warning(f"Deadlock in {func.__name__} (attempt {attempt}/{MAX_DEADLOCK_RETRIES}), "
                                   f"retrying in {DEADLOCK_RETRY_DELAY}s...")
                    time.sleep(DEADLOCK_RETRY_DELAY * attempt)
                else:
                    logger.error(f"Deadlock in {func.__name__} after {MAX_DEADLOCK_RETRIES} attempts, giving up")
                    raise
    return wrapper


# =============================================================================
# Individual Insert Functions
# =============================================================================

@retry_on_deadlock
def insert_price_suggestion(conn, data: Dict[str, Any]) -> int:
    """Insert a single price suggestion"""
    query = """
        INSERT INTO price_suggestions (product_id, suggested_price, current_price, reason, confidence)
        VALUES (%(product_id)s, %(suggested_price)s, %(current_price)s, %(reason)s, %(confidence)s)
        RETURNING id
    """
    with conn.cursor() as cur:
        cur.execute(query, data)
        conn.commit()
        return cur.fetchone()[0]


@retry_on_deadlock
def insert_price_anomaly(conn, data: Dict[str, Any]) -> int:
    """Insert a single price anomaly"""
    query = """
        INSERT INTO price_anomalies (product_id, current_price, expected_price, anomaly_score, is_anomaly)
        VALUES (%(product_id)s, %(current_price)s, %(expected_price)s, %(anomaly_score)s, %(is_anomaly)s)
        RETURNING id
    """
    with conn.cursor() as cur:
        cur.execute(query, data)
        conn.commit()
        return cur.fetchone()[0]


@retry_on_deadlock
def insert_sales_anomaly(conn, data: Dict[str, Any]) -> int:
    """Insert a single sales anomaly"""
    query = """
        INSERT INTO sales_anomalies (product_id, sales_volume, expected_sales, anomaly_score, is_anomaly)
        VALUES (%(product_id)s, %(sales_volume)s, %(expected_sales)s, %(anomaly_score)s, %(is_anomaly)s)
        RETURNING id
    """
    with conn.cursor() as cur:
        cur.execute(query, data)
        conn.commit()
        return cur.fetchone()[0]


@retry_on_deadlock
def insert_notification(conn, data: Dict[str, Any]) -> int:
    """Insert a single notification"""
    query = """
        INSERT INTO notifications
            (product_id, model_type, category, notification_type, severity, message, action_recommended, related_result_id)
        VALUES
            (%(product_id)s, %(model_type)s, %(category)s, %(notification_type)s, %(severity)s,
             %(message)s, %(action_recommended)s, %(related_result_id)s)
        RETURNING id
    """
    with conn.cursor() as cur:
        cur.execute(query, data)
        conn.commit()
        return cur.fetchone()[0]


@retry_on_deadlock
def insert_demand_forecast(conn, data: Dict[str, Any]) -> int:
    """Insert demand forecast result"""
    query = """
        INSERT INTO demand_forecasts
            (product_id, forecast_date, forecast_days, total_predicted_demand, avg_daily_demand,
             current_stock, recommended_stock, reorder_quantity, days_until_stockout, urgency, mape, rmse)
        VALUES
            (%(product_id)s, %(forecast_date)s, %(forecast_days)s, %(total_predicted_demand)s,
             %(avg_daily_demand)s, %(current_stock)s, %(recommended_stock)s, %(reorder_quantity)s,
             %(days_until_stockout)s, %(urgency)s, %(mape)s, %(rmse)s)
        RETURNING id
    """
    with conn.cursor() as cur:
        cur.execute(query, data)
        conn.commit()
        return cur.fetchone()[0]


@retry_on_deadlock
def insert_classification(conn, data: Dict[str, Any]) -> int:
    """Insert ABC-XYZ classification result"""
    query = """
        INSERT INTO product_classifications
            (product_id, abc_class, xyz_class, combined_class, total_revenue,
             revenue_contribution_pct, total_units_sold, coefficient_of_variation, strategy, priority)
        VALUES
            (%(product_id)s, %(abc_class)s, %(xyz_class)s, %(combined_class)s, %(total_revenue)s,
             %(revenue_contribution_pct)s, %(total_units_sold)s, %(coefficient_of_variation)s,
             %(strategy)s, %(priority)s)
        RETURNING id
    """
    with conn.cursor() as cur:
        cur.execute(query, data)
        conn.commit()
        return cur.fetchone()[0]


@retry_on_deadlock
def insert_cluster_result(conn, data: Dict[str, Any]) -> int:
    """Insert product clustering result"""
    query = """
        INSERT INTO product_clusters
            (product_id, cluster_id, cluster_name, revenue_score, variability_score,
             trend_score, seasonality_score, frequency_score, margin_score,
             distance_to_centroid, n_clusters, silhouette_score)
        VALUES
            (%(product_id)s, %(cluster_id)s, %(cluster_name)s, %(revenue_score)s,
             %(variability_score)s, %(trend_score)s, %(seasonality_score)s, %(frequency_score)s,
             %(margin_score)s, %(distance_to_centroid)s, %(n_clusters)s, %(silhouette_score)s)
        RETURNING id
    """
    with conn.cursor() as cur:
        cur.execute(query, data)
        conn.commit()
        return cur.fetchone()[0]


@retry_on_deadlock
def insert_supplier_score(conn, data: Dict[str, Any]) -> int:
    """Insert supplier score result"""
    query = """
        INSERT INTO supplier_scores
            (supplier_id, overall_score, delivery_score, quality_score,
             lead_time_score, fulfillment_score, rating, total_restocks)
        VALUES
            (%(supplier_id)s, %(overall_score)s, %(delivery_score)s, %(quality_score)s,
             %(lead_time_score)s, %(fulfillment_score)s, %(rating)s, %(total_restocks)s)
        RETURNING id
    """
    with conn.cursor() as cur:
        cur.execute(query, data)
        conn.commit()
        return cur.fetchone()[0]


# =============================================================================
# Batch Insert Functions (More Efficient for Large Datasets)
# =============================================================================

@retry_on_deadlock
def insert_price_anomalies_batch(conn, predictions: List[Dict[str, Any]]) -> List[int]:
    """Insert multiple price anomalies in a single transaction"""
    if not predictions:
        return []

    query = """
        INSERT INTO price_anomalies (product_id, current_price, expected_price, anomaly_score, is_anomaly)
        VALUES (%(product_id)s, %(current_price)s, %(expected_price)s, %(anomaly_score)s, %(is_anomaly)s)
        RETURNING id
    """
    ids = []
    with conn.cursor() as cur:
        for pred in predictions:
            cur.execute(query, pred)
            ids.append(cur.fetchone()[0])
        conn.commit()

    logger.info(f"Batch inserted {len(ids)} price anomalies")
    return ids


@retry_on_deadlock
def insert_sales_anomalies_batch(conn, predictions: List[Dict[str, Any]]) -> List[int]:
    """Insert multiple sales anomalies in a single transaction"""
    if not predictions:
        return []

    query = """
        INSERT INTO sales_anomalies (product_id, sales_volume, expected_sales, anomaly_score, is_anomaly)
        VALUES (%(product_id)s, %(sales_volume)s, %(expected_sales)s, %(anomaly_score)s, %(is_anomaly)s)
        RETURNING id
    """
    ids = []
    with conn.cursor() as cur:
        for pred in predictions:
            cur.execute(query, pred)
            ids.append(cur.fetchone()[0])
        conn.commit()

    logger.info(f"Batch inserted {len(ids)} sales anomalies")
    return ids


@retry_on_deadlock
def insert_price_suggestions_batch(conn, suggestions: List[Dict[str, Any]]) -> List[int]:
    """Insert multiple price suggestions in a single transaction"""
    if not suggestions:
        return []

    query = """
        INSERT INTO price_suggestions (product_id, suggested_price, current_price, reason, confidence)
        VALUES (%(product_id)s, %(suggested_price)s, %(current_price)s, %(reason)s, %(confidence)s)
        RETURNING id
    """
    ids = []
    with conn.cursor() as cur:
        for sugg in suggestions:
            cur.execute(query, sugg)
            ids.append(cur.fetchone()[0])
        conn.commit()

    logger.info(f"Batch inserted {len(ids)} price suggestions")
    return ids


@retry_on_deadlock
def insert_notifications_batch(conn, notifications: List[Dict[str, Any]]) -> List[int]:
    """Insert multiple notifications in a single transaction"""
    if not notifications:
        return []

    query = """
        INSERT INTO notifications
            (product_id, model_type, category, notification_type, severity, message, action_recommended, related_result_id)
        VALUES
            (%(product_id)s, %(model_type)s, %(category)s, %(notification_type)s, %(severity)s,
             %(message)s, %(action_recommended)s, %(related_result_id)s)
        RETURNING id
    """
    ids = []
    with conn.cursor() as cur:
        for notif in notifications:
            cur.execute(query, notif)
            ids.append(cur.fetchone()[0])
        conn.commit()

    logger.info(f"Batch inserted {len(ids)} notifications")
    return ids
