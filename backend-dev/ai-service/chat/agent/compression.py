"""History compression: summarize old turns once a session gets long.

Each chat turn re-sends the conversation history. Past ~20 turns, this
balloons context tokens (slow + costly + can blow context window on small
models with 4k ctx).

Strategy: when history exceeds COMPRESS_THRESHOLD messages, ask the LLM to
write a 200-word recap of the older half, store it in `chat_summaries`, and
prepend the summary as a single system message in front of the *recent* tail.
The full original history is kept in the DB — compression only affects what
we ship to the LLM each turn.
"""

from __future__ import annotations

import os
from typing import List, Optional

from utils.logger import get_logger
from database.connection import get_db_connection

from ..llm.factory import chat_with_fallback
from ..types import Message


COMPRESS_THRESHOLD = int(os.getenv("CHAT_COMPRESS_THRESHOLD", "20"))
KEEP_RECENT = int(os.getenv("CHAT_COMPRESS_KEEP_RECENT", "10"))

logger = get_logger("chat.agent.compression")


def get_existing_summary(session_id: str) -> Optional[tuple[str, int]]:
    """Return (summary_text, up_to_message_id) for the session, if any."""
    if not session_id:
        return None
    try:
        with get_db_connection() as conn:
            with conn.cursor() as cur:
                cur.execute(
                    "SELECT summary, up_to_message_id FROM chat_summaries "
                    "WHERE session_id = %s::uuid "
                    "ORDER BY up_to_message_id DESC LIMIT 1",
                    (session_id,),
                )
                row = cur.fetchone()
        if not row:
            return None
        return (row[0], int(row[1]))
    except Exception as e:
        logger.debug("get_existing_summary failed (%s); skipping summary", e)
        return None


def maybe_compress(
    session_id: str,
    history: List[Message],
    *,
    provider_pref: Optional[str] = None,
) -> tuple[List[Message], Optional[str]]:
    """Return (compressed_history, summary_text_or_None).

    `compressed_history` is what should be passed to the LLM:
      - if no compression needed: original history unchanged, summary=None
      - else: [System(summary)] + last KEEP_RECENT messages

    Side effect: persists the new summary to chat_summaries.
    """
    if len(history) <= COMPRESS_THRESHOLD:
        return history, None

    older = history[:-KEEP_RECENT]
    recent = history[-KEEP_RECENT:]

    # Build a compact rendering of older for the summarizer.
    rendered = "\n".join(
        f"{m.role.upper()}: {m.content[:400]}" for m in older
    )

    summarizer_msgs = [
        Message(role="system", content=(
            "Tu es un résumeur. Rédige un résumé en français de la conversation suivante, "
            "en 200 mots maximum. Garde uniquement les faits, décisions et chiffres "
            "importants. Pas de phrases introductives."
        )),
        Message(role="user", content=rendered),
    ]

    try:
        resp = chat_with_fallback(provider_pref or "auto", summarizer_msgs,
                                  tools=None, max_tokens=400, temperature=0.2)
        summary = resp.content.strip()
    except Exception as e:
        logger.warning("Summarizer call failed (%s); skipping compression", e)
        return history, None

    # Persist for future turns.
    try:
        with get_db_connection() as conn:
            with conn.cursor() as cur:
                cur.execute(
                    "INSERT INTO chat_summaries "
                    "(session_id, up_to_message_id, summary, tokens_in, tokens_out) "
                    "VALUES (%s::uuid, %s, %s, %s, %s)",
                    (session_id, len(older), summary,
                     resp.usage.tokens_in, resp.usage.tokens_out),
                )
                conn.commit()
    except Exception as e:
        logger.warning("chat_summaries insert failed (%s); summary used in-memory only", e)

    summary_msg = Message(
        role="system",
        content=f"Résumé de la conversation précédente (de {len(older)} messages) :\n{summary}",
    )
    return [summary_msg] + recent, summary
