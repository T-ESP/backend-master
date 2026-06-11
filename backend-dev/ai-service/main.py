import os
import threading
from flask import Flask, jsonify
from scheduler import start_scheduler, run_all_tenants, get_metrics
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
                # Free local LLM RAM during heavy ML cron run.
                try:
                    from chat.llm.factory import _instance as _llm_instance  # type: ignore
                    _llm_instance("local").unload()
                except Exception as e:
                    logger.warning("Could not unload local LLM before cron: %s", e)

                run_all_tenants()
            finally:
                _is_running = False

    thread = threading.Thread(target=_run, daemon=True)
    thread.start()

    return jsonify({"message": "AI jobs triggered successfully", "status": "started"}), 202


# ---------------------------------------------------------------------------
# Chatbot blueprint
# ---------------------------------------------------------------------------
try:
    from chat.routes import bp as chat_bp
    app.register_blueprint(chat_bp)
    logger.info("Chat blueprint registered")
except Exception as e:
    logger.error("Failed to register chat blueprint: %s", e)


def _index_rag_at_startup() -> None:
    """Run the RAG indexer on startup if configured.

    Runs in a background thread so the HTTP server starts immediately.
    """
    if os.getenv("RAG_INDEX_ON_STARTUP", "true").lower() != "true":
        logger.info("RAG_INDEX_ON_STARTUP=false — skipping index at startup")
        return

    def _job():
        try:
            from chat.rag import index_corpus
            logger.info("Running RAG indexing at startup...")
            metrics = index_corpus(force=False)
            logger.info("RAG index complete: %s", metrics)
        except Exception as e:
            logger.exception("RAG startup indexing failed: %s", e)

    threading.Thread(target=_job, daemon=True).start()


if __name__ == "__main__":
    print("AI Service starting...")

    # Start the background scheduler (runs on startup + cron)
    scheduler = start_scheduler()

    # Kick off RAG indexing in the background
    _index_rag_at_startup()

    # Start the Flask HTTP server
    port = int(os.getenv("AI_SERVICE_PORT", "8001"))
    logger.info(f"Starting HTTP server on port {port}")

    try:
        app.run(host="0.0.0.0", port=port, debug=False, threaded=True)
    finally:
        scheduler.shutdown()
        close_pool()
