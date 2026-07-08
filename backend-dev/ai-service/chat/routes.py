"""Flask routes exposed by ai-service for the chatbot.

These endpoints are **internal** — only the Rust API should call them. The
Rust side handles JWT validation and forwards the user's token in the
`Authorization` header on each call.

Public surface:
- POST /chat/turn        — one chat turn (non-streaming)
- POST /rag/reindex      — reindex the corpus
- GET  /rag/stats        — corpus stats
- GET  /llm/health       — provider availability
"""

from __future__ import annotations

import os
from typing import Any

import json
import time

from flask import Blueprint, Response, jsonify, request, stream_with_context

from utils.logger import get_logger

from .agent import run_turn
from .llm import provider_health
from .rag import get_index_stats, index_corpus
from .tools import ToolContext
from .types import Message, ToolCall


bp = Blueprint("chat", __name__)
logger = get_logger("chat.routes")


def _msg_from_payload(p: dict[str, Any]) -> Message:
    role = p.get("role") or "user"
    tool_calls = None
    raw_tcs = p.get("tool_calls")
    if isinstance(raw_tcs, list):
        tool_calls = [
            ToolCall(
                id=tc.get("id", f"hist_{i}"),
                name=tc.get("name") or tc.get("function", {}).get("name", ""),
                arguments=tc.get("arguments") or tc.get("function", {}).get("arguments", {}) or {},
            )
            for i, tc in enumerate(raw_tcs)
        ]
    return Message(
        role=role,
        content=p.get("content", ""),
        tool_calls=tool_calls,
        tool_call_id=p.get("tool_call_id"),
        name=p.get("name"),
    )


@bp.route("/chat/turn", methods=["POST"])
def chat_turn() -> Response:
    """One chat turn.

    Request body:
        {
          "user_message": str,
          "history": [ {role, content, tool_calls?, name?, tool_call_id?}, ... ],
          "user_jwt": str,
          "user_id": int,
          "session_id": str,
          "provider": "auto" | "groq" | "mistral" | null,
          "proactive_summary": str | null
        }

    Response: {content, intent, provider_used, tool_calls[], pending_action?, usage}
    """
    body = request.get_json(silent=True) or {}

    user_message = (body.get("user_message") or "").strip()
    if not user_message:
        return jsonify({"error": "user_message is required"}), 400

    user_jwt = body.get("user_jwt") or ""
    session_id = str(body.get("session_id") or "")
    commerce_id = str(body.get("commerce_id") or "")
    slug = str(body.get("slug") or "")
    provider_pref = body.get("provider")
    proactive_summary = body.get("proactive_summary")

    history_payload = body.get("history") or []
    history = [_msg_from_payload(p) for p in history_payload]

    ctx = ToolContext(user_jwt=user_jwt, session_id=session_id,
                      commerce_id=commerce_id, slug=slug)

    try:
        result = run_turn(
            user_message=user_message,
            history=history,
            ctx=ctx,
            provider_pref=provider_pref,
            proactive_summary=proactive_summary,
        )
    except Exception as e:
        logger.exception("chat_turn failed")
        return jsonify({"error": f"internal: {e}"}), 500

    return jsonify({
        "content": result.content,
        "intent": result.intent,
        "provider_used": result.provider_used,
        "tool_calls": result.tool_calls,
        "pending_action": result.pending_action,
        "citations": [
            {"source_path": c.source_path, "heading": c.heading, "similarity": c.similarity}
            for c in result.citations
        ],
        "cached": result.cached,
        "shortcut_used": result.shortcut_used,
        "numbers_verified": result.numbers_verified,
        "suggestions": result.suggestions,
        "usage": {
            "tokens_in": result.usage.tokens_in,
            "tokens_out": result.usage.tokens_out,
            "latency_ms": result.usage.latency_ms,
        },
    })


@bp.route("/chat/turn/stream", methods=["POST"])
def chat_turn_stream() -> Response:
    """SSE-style stream of a chat turn.

    Emits one event per pipeline stage so the frontend can render progress:
        - intent: which intent was classified
        - shortcut: if a deterministic shortcut matched
        - tool_call: each tool that fired
        - delta: chunks of the final answer (currently single chunk; real
          token-streaming requires per-provider hooks — left for follow-up)
        - done: final usage + citations + pending_action

    Body shape: same as /chat/turn.
    """
    body = request.get_json(silent=True) or {}
    user_message = (body.get("user_message") or "").strip()
    if not user_message:
        return jsonify({"error": "user_message is required"}), 400

    user_jwt = body.get("user_jwt") or ""
    session_id = str(body.get("session_id") or "")
    commerce_id = str(body.get("commerce_id") or "")
    slug = str(body.get("slug") or "")
    provider_pref = body.get("provider")
    proactive_summary = body.get("proactive_summary")
    history = [_msg_from_payload(p) for p in (body.get("history") or [])]
    ctx = ToolContext(user_jwt=user_jwt, session_id=session_id,
                      commerce_id=commerce_id, slug=slug)

    def _sse(event: str, data: dict) -> str:
        return f"event: {event}\ndata: {json.dumps(data, ensure_ascii=False)}\n\n"

    @stream_with_context
    def gen():
        # Heartbeat so frontends know we're alive while the LLM thinks.
        yield _sse("ping", {"ts": int(time.time())})
        try:
            result = run_turn(
                user_message=user_message,
                history=history,
                ctx=ctx,
                provider_pref=provider_pref,
                proactive_summary=proactive_summary,
            )
        except Exception as e:
            logger.exception("stream turn failed")
            yield _sse("error", {"message": str(e)})
            return

        # Surface intermediate decisions as events for nicer UX.
        yield _sse("intent", {"intent": result.intent})
        if result.shortcut_used:
            yield _sse("shortcut", {"tool": result.shortcut_used})
        for tc in result.tool_calls:
            yield _sse("tool_call", tc)
        if result.cached:
            yield _sse("cached", {"provider": result.provider_used})

        # Single-chunk "delta" — real token-by-token streaming is provider-
        # specific (Groq / Mistral SDK). The frontend protocol is identical
        # either way, so we can upgrade later transparently.
        if result.content:
            yield _sse("delta", {"content": result.content})

        if result.pending_action:
            yield _sse("pending_action", result.pending_action)

        yield _sse("done", {
            "provider_used": result.provider_used,
            "intent": result.intent,
            "numbers_verified": result.numbers_verified,
            "shortcut_used": result.shortcut_used,
            "suggestions": result.suggestions,
            "citations": [
                {"source_path": c.source_path, "heading": c.heading,
                 "similarity": c.similarity}
                for c in result.citations
            ],
            "usage": {
                "tokens_in": result.usage.tokens_in,
                "tokens_out": result.usage.tokens_out,
                "latency_ms": result.usage.latency_ms,
            },
        })

    return Response(gen(), mimetype="text/event-stream",
                    headers={"Cache-Control": "no-cache", "X-Accel-Buffering": "no"})


@bp.route("/chat/execute-tool", methods=["POST"])
def chat_execute_tool() -> Response:
    """Exécute un outil par son nom — utilisé par Rust pour lancer une action
    d'écriture APRÈS confirmation de l'utilisateur.

    Body : {tool_name, tool_args, user_jwt, user_id, session_id}
    """
    from .tools import execute_tool as _exec

    body = request.get_json(silent=True) or {}
    tool_name = body.get("tool_name") or ""
    tool_args = body.get("tool_args") or {}
    if not tool_name:
        return jsonify({"ok": False, "error": "tool_name requis"}), 400
    ctx = ToolContext(
        user_jwt=body.get("user_jwt") or "",
        session_id=str(body.get("session_id") or ""),
        commerce_id=str(body.get("commerce_id") or ""),
        slug=str(body.get("slug") or ""),
    )
    try:
        result = _exec(tool_name, tool_args, ctx)
        return jsonify({"ok": result.ok, "data": result.data, "error": result.error})
    except Exception as e:
        logger.exception("execute-tool failed")
        return jsonify({"ok": False, "error": f"internal: {e}"}), 500


@bp.route("/chat/briefing", methods=["POST"])
def chat_briefing() -> Response:
    """Briefing proactif de l'état du magasin (alertes, réappros, stock bas).

    Déterministe, sans LLM. Appelé par Rust à l'ouverture d'une session.
    Body : {user_jwt, user_id, session_id}
    """
    from .agent.briefing import build_briefing

    body = request.get_json(silent=True) or {}
    ctx = ToolContext(
        user_jwt=body.get("user_jwt") or "",
        session_id=str(body.get("session_id") or ""),
        commerce_id=str(body.get("commerce_id") or ""),
        slug=str(body.get("slug") or ""),
    )
    try:
        return jsonify(build_briefing(ctx))
    except Exception as e:
        logger.exception("briefing failed")
        return jsonify({"error": f"internal: {e}"}), 500


@bp.route("/rag/reindex", methods=["POST"])
def rag_reindex() -> Response:
    body = request.get_json(silent=True) or {}
    force = bool(body.get("force", False))
    metrics = index_corpus(force=force)
    return jsonify(metrics)


@bp.route("/rag/stats", methods=["GET"])
def rag_stats() -> Response:
    return jsonify(get_index_stats())


@bp.route("/llm/health", methods=["GET"])
def llm_health() -> Response:
    return jsonify(provider_health())
