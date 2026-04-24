import os
import threading
from flask import Flask, jsonify
from scheduler import start_scheduler, run_all_jobs, get_metrics
from database.connection import check_database_health, close_pool
from utils.logger import get_logger

logger = get_logger("main")

app = Flask(__name__)

# Track if a run is in progress
_run_lock = threading.Lock()
_is_running = False


@app.route("/ai/health", methods=["GET"])
def health():
    db_health = check_database_health()
    return jsonify({
        "service": "ai-service",
        "status": "healthy" if db_health["status"] == "healthy" else "degraded",
        "database": db_health
    })


@app.route("/ai/status", methods=["GET"])
def status():
    metrics = get_metrics()
    return jsonify({
        "last_run": metrics if metrics else None,
        "is_running": _is_running
    })


@app.route("/ai/run", methods=["POST"])
def trigger_run():
    global _is_running

    if _is_running:
        return jsonify({"error": "A run is already in progress"}), 409

    def _run():
        global _is_running
        with _run_lock:
            _is_running = True
            try:
                run_all_jobs()
            finally:
                _is_running = False

    thread = threading.Thread(target=_run, daemon=True)
    thread.start()

    return jsonify({"message": "AI jobs triggered successfully", "status": "started"}), 202


if __name__ == "__main__":
    print("AI Service starting...")

    # Start the background scheduler (runs on startup + cron)
    scheduler = start_scheduler()

    # Start the Flask HTTP server
    port = int(os.getenv("AI_SERVICE_PORT", "8001"))
    logger.info(f"Starting HTTP server on port {port}")

    try:
        app.run(host="0.0.0.0", port=port, debug=False)
    finally:
        scheduler.shutdown()
        close_pool()
