"""
AI Service Scheduler

Runs all AI handlers on a configurable cron schedule.
Features:
- Parallel execution for independent handlers
- Sequential execution for dependent handlers
- Comprehensive logging and metrics
- Graceful error handling

Default: Daily at 2 AM
"""

import os
import time
import psycopg2
from urllib.parse import urlparse, urlunparse
from datetime import datetime
from concurrent.futures import ThreadPoolExecutor, as_completed
from typing import List, Dict, Any

from apscheduler.schedulers.background import BackgroundScheduler
from apscheduler.triggers.cron import CronTrigger
from dotenv import load_dotenv

from utils.logger import get_logger
from database.connection import check_database_health, close_pool, get_db_connection

# Original handlers (can run in parallel)
from handlers.price_suggestion_handler import PriceSuggestionHandler
from handlers.price_anomaly_handler import PriceAnomalyHandler
from handlers.sales_anomaly_handler import SalesAnomalyHandler

# New handlers
from handlers.demand_forecast_handler import DemandForecastHandler
from handlers.classification_handler import ClassificationHandler
from handlers.clustering_handler import ClusteringHandler
from handlers.supplier_scoring_handler import SupplierScoringHandler

load_dotenv()

logger = get_logger("scheduler")

CRON_SCHEDULE = os.getenv("CRON_SCHEDULE", "0 2 * * *")
PARALLEL_EXECUTION = os.getenv("PARALLEL_EXECUTION", "true").lower() == "true"
MAX_WORKERS = int(os.getenv("MAX_WORKERS", "4"))
RUN_ON_STARTUP = os.getenv("RUN_ON_STARTUP", "true").lower() == "true"

# Handler groups - handlers in the same group can run in parallel
# Groups are executed sequentially
HANDLER_GROUPS = [
    # Group 1: Independent anomaly detection (can run in parallel)
    {
        "name": "Anomaly Detection",
        "parallel": True,
        "handlers": [
            PriceAnomalyHandler,
            SalesAnomalyHandler,
        ]
    },
    # Group 2: Analysis handlers (can run in parallel)
    {
        "name": "Product Analysis",
        "parallel": True,
        "handlers": [
            ClassificationHandler,
            ClusteringHandler,
            SupplierScoringHandler,
        ]
    },
    # Group 3: Forecasting (depends on fresh data)
    {
        "name": "Forecasting",
        "parallel": False,
        "handlers": [
            DemandForecastHandler,
        ]
    },
    # Group 4: Price suggestions (uses insights from other models)
    {
        "name": "Price Optimization",
        "parallel": False,
        "handlers": [
            PriceSuggestionHandler,
        ]
    },
]

# Metrics storage
_last_run_metrics: Dict[str, Any] = {}


def get_metrics() -> Dict[str, Any]:
    """Get metrics from the last run"""
    return _last_run_metrics.copy()


def _run_handler(handler_class) -> Dict[str, Any]:
    """Run a single handler and return metrics"""
    handler_name = handler_class.__name__
    start_time = time.time()

    try:
        handler = handler_class()
        handler.run()
        duration = time.time() - start_time

        return {
            "handler": handler_name,
            "status": "success",
            "duration_seconds": round(duration, 2),
            "error": None
        }
    except Exception as e:
        duration = time.time() - start_time
        logger.error(f"{handler_name} failed: {e}", exc_info=True)

        return {
            "handler": handler_name,
            "status": "failed",
            "duration_seconds": round(duration, 2),
            "error": str(e)
        }


def _run_group_parallel(group: Dict[str, Any]) -> List[Dict[str, Any]]:
    """Run handlers in a group in parallel"""
    results = []

    with ThreadPoolExecutor(max_workers=MAX_WORKERS) as executor:
        futures = {
            executor.submit(_run_handler, handler_class): handler_class.__name__
            for handler_class in group["handlers"]
        }

        for future in as_completed(futures):
            handler_name = futures[future]
            try:
                result = future.result()
                results.append(result)
            except Exception as e:
                logger.error(f"Unexpected error in {handler_name}: {e}")
                results.append({
                    "handler": handler_name,
                    "status": "failed",
                    "duration_seconds": 0,
                    "error": str(e)
                })

    return results


def _run_group_sequential(group: Dict[str, Any]) -> List[Dict[str, Any]]:
    """Run handlers in a group sequentially"""
    results = []

    for handler_class in group["handlers"]:
        result = _run_handler(handler_class)
        results.append(result)

    return results


def run_all_jobs() -> Dict[str, Any]:
    """Run all handlers with parallel execution support"""
    global _last_run_metrics

    run_start = datetime.now()
    logger.info("=" * 60)
    logger.info("Starting scheduled AI jobs...")
    logger.info(f"Parallel execution: {PARALLEL_EXECUTION}, Max workers: {MAX_WORKERS}")
    logger.info("=" * 60)

    all_results = []
    group_metrics = []

    for group in HANDLER_GROUPS:
        group_name = group["name"]
        group_start = time.time()

        logger.info(f"\n--- {group_name} ---")

        if PARALLEL_EXECUTION and group["parallel"] and len(group["handlers"]) > 1:
            logger.info(f"Running {len(group['handlers'])} handlers in parallel...")
            results = _run_group_parallel(group)
        else:
            logger.info(f"Running {len(group['handlers'])} handlers sequentially...")
            results = _run_group_sequential(group)

        group_duration = time.time() - group_start
        all_results.extend(results)

        # Log group results
        successful = sum(1 for r in results if r["status"] == "success")
        failed = sum(1 for r in results if r["status"] == "failed")

        group_metrics.append({
            "group": group_name,
            "successful": successful,
            "failed": failed,
            "duration_seconds": round(group_duration, 2)
        })

        logger.info(f"{group_name} completed: {successful} successful, {failed} failed ({group_duration:.2f}s)")

    # Calculate totals
    run_duration = (datetime.now() - run_start).total_seconds()
    total_successful = sum(1 for r in all_results if r["status"] == "success")
    total_failed = sum(1 for r in all_results if r["status"] == "failed")

    logger.info("\n" + "=" * 60)
    logger.info(f"All jobs completed: {total_successful} successful, {total_failed} failed")
    logger.info(f"Total duration: {run_duration:.2f} seconds")
    logger.info("=" * 60)

    # Store metrics
    _last_run_metrics = {
        "run_started": run_start.isoformat(),
        "run_completed": datetime.now().isoformat(),
        "duration_seconds": round(run_duration, 2),
        "total_handlers": len(all_results),
        "successful": total_successful,
        "failed": total_failed,
        "groups": group_metrics,
        "handlers": all_results
    }

    return _last_run_metrics


# ---------------------------------------------------------------------------
# Multi-tenant orchestration
# ---------------------------------------------------------------------------
# The ML handlers read/write via the shared connection pool. To run them for
# every tenant we point that pool at each ``tenant_<slug>`` database in turn
# (and namespace cached models per tenant via AI_MODEL_DIR) so results land in
# the right tenant's tables. RAG retrieval stays pinned to the master DB.

def _master_dsn() -> str:
    return os.environ["DATABASE_URL"]


def list_active_tenants():
    """Return [(slug, db_name), ...] for active commerces from the master DB."""
    conn = psycopg2.connect(_master_dsn())
    try:
        with conn.cursor() as cur:
            cur.execute(
                "SELECT slug, db_name FROM commerces WHERE status = 'active' ORDER BY slug"
            )
            return cur.fetchall()
    finally:
        conn.close()


def _tenant_dsn(db_name: str) -> str:
    parsed = urlparse(_master_dsn())
    return urlunparse(parsed._replace(path=f"/{db_name}"))


def run_all_tenants():
    """Run every ML handler for every active tenant, against that tenant's DB."""
    from database.connection import reconfigure_pool

    base_model_dir = os.getenv("AI_MODEL_BASE_DIR", "/app/saved_models")
    try:
        tenants = list_active_tenants()
    except Exception as e:
        logger.error(f"Could not list tenants: {e}", exc_info=True)
        return {"tenants": {}, "count": 0, "error": str(e)}

    if not tenants:
        logger.warning("No active tenants found; nothing to run.")
        return {"tenants": {}, "count": 0}

    results: Dict[str, Any] = {}
    for slug, db_name in tenants:
        logger.info("=" * 60)
        logger.info(f"ML run for tenant '{slug}' ({db_name})")
        os.environ["AI_MODEL_DIR"] = os.path.join(base_model_dir, slug)
        try:
            reconfigure_pool(_tenant_dsn(db_name))
            results[slug] = run_all_jobs()
        except Exception as e:
            logger.error(f"ML run failed for tenant {slug}: {e}", exc_info=True)
            results[slug] = {"error": str(e)}

    # Restore the shared pool to master and clear the per-tenant model dir.
    os.environ.pop("AI_MODEL_DIR", None)
    try:
        reconfigure_pool(_master_dsn())
    except Exception as e:
        logger.warning(f"Could not restore master pool: {e}")

    logger.info(f"Per-tenant ML run complete for {len(tenants)} tenant(s)")
    return {"tenants": results, "count": len(tenants)}


def start_scheduler():
    """Start the APScheduler with configured cron schedule.
    Returns the scheduler instance (BackgroundScheduler) so the HTTP server can run alongside it.
    """
    # Check database health first
    health = check_database_health()
    if health["status"] != "healthy":
        logger.error(f"Database health check failed: {health}")
        raise RuntimeError("Database is not healthy")

    logger.info(f"Database health: {health['status']} ({health.get('version', 'unknown')})")

    scheduler = BackgroundScheduler()

    parts = CRON_SCHEDULE.split()
    trigger = CronTrigger(
        minute=parts[0],
        hour=parts[1],
        day=parts[2],
        month=parts[3],
        day_of_week=parts[4],
    )

    scheduler.add_job(run_all_tenants, trigger)

    # Count total handlers
    total_handlers = sum(len(g["handlers"]) for g in HANDLER_GROUPS)

    logger.info("=" * 60)
    logger.info("AI Service Scheduler Started")
    logger.info(f"Cron schedule: {CRON_SCHEDULE}")
    logger.info(f"Run on startup: {RUN_ON_STARTUP}")
    logger.info(f"Parallel execution: {PARALLEL_EXECUTION}")
    logger.info(f"Max workers: {MAX_WORKERS}")
    logger.info(f"Handler groups: {len(HANDLER_GROUPS)}")
    logger.info(f"Total handlers: {total_handlers}")

    for group in HANDLER_GROUPS:
        mode = "parallel" if group["parallel"] else "sequential"
        logger.info(f"  [{mode}] {group['name']}:")
        for handler_class in group["handlers"]:
            logger.info(f"    - {handler_class.__name__}")

    logger.info("=" * 60)

    scheduler.start()

    # Run all jobs for every tenant immediately on startup if configured
    if RUN_ON_STARTUP:
        logger.info("Running all jobs for all tenants on startup...")
        run_all_tenants()

    return scheduler


def _wait_for_seed_data(max_wait=300, poll_interval=5, min_products=100, min_orders=1000):
    """Wait for seed data to be fully available before running AI models.
    Uses a stability check: counts must meet minimum thresholds AND be unchanged
    between two consecutive polls, meaning the seed has finished inserting.
    Also checks that no heavy locks (TRUNCATE) are active on key tables.
    """
    logger.info("Waiting for seed data to be ready...")
    waited = 0
    prev_product_count = 0
    prev_order_count = 0

    while waited < max_wait:
        try:
            with get_db_connection() as conn:
                with conn.cursor() as cur:
                    # Check for active locks that would indicate seed is truncating
                    cur.execute("""
                        SELECT COUNT(*) FROM pg_locks l
                        JOIN pg_class c ON l.relation = c.oid
                        WHERE c.relname IN ('products_pro', 'order_ord', 'line_order_lor')
                        AND l.mode = 'AccessExclusiveLock'
                    """)
                    active_locks = cur.fetchone()[0]
                    if active_locks > 0:
                        logger.info(f"Seed TRUNCATE still in progress ({active_locks} exclusive locks), waiting {poll_interval}s...")
                        time.sleep(poll_interval)
                        waited += poll_interval
                        continue

                    cur.execute("SELECT COUNT(*) FROM products_pro")
                    product_count = cur.fetchone()[0]
                    cur.execute("SELECT COUNT(*) FROM order_ord")
                    order_count = cur.fetchone()[0]

                    if (product_count >= min_products and order_count >= min_orders
                            and product_count == prev_product_count
                            and order_count == prev_order_count):
                        logger.info(f"Seed data ready and stable: {product_count} products, {order_count} orders")
                        return
                    elif product_count > 0 or order_count > 0:
                        logger.info(f"Seed in progress (products={product_count}, orders={order_count}, "
                                    f"need >={min_products}/{min_orders}), waiting for stability...")
                    else:
                        logger.info(f"No seed data yet (products=0, orders=0), waiting {poll_interval}s...")

                    prev_product_count = product_count
                    prev_order_count = order_count
        except Exception as e:
            logger.warning(f"Error checking seed data: {e}, retrying in {poll_interval}s...")

        time.sleep(poll_interval)
        waited += poll_interval

    logger.warning(f"Seed data not ready after {max_wait}s, proceeding anyway...")
