"""Response cache for repeated doc questions.

For doc/concept questions (e.g. "que signifie ABC-XYZ ?"), the answer is
deterministic given the corpus. We cache by query embedding so equivalent
phrasings ("c'est quoi ABC-XYZ ?", "explique ABC-XYZ") share an entry.

Cache lookup: nearest neighbor in `chat_response_cache.query_embedding`,
hit if cosine_distance <= CACHE_HIT_THRESHOLD (0.05 by default — very strict).

Only doc-intent turns use the cache. Data/action turns always go fresh
because the underlying KPIs change.
"""

from __future__ import annotations

import hashlib
import os
from typing import Optional

from utils.logger import get_logger
from database.connection import get_db_connection

from ..rag.embedder import get_embedder


CACHE_ENABLED = os.getenv("CHAT_RESPONSE_CACHE", "true").lower() == "true"
CACHE_HIT_THRESHOLD = float(os.getenv("CHAT_CACHE_THRESHOLD", "0.05"))

logger = get_logger("chat.agent.cache")


def _norm_hash(query: str) -> str:
    return hashlib.sha256(query.strip().lower().encode("utf-8")).hexdigest()


def _fmt_vec(vec: list[float]) -> str:
    return "[" + ",".join(f"{v:.7f}" for v in vec) + "]"


def lookup(query: str) -> Optional[tuple[str, str]]:
    """Try to find a cached answer. Returns (response, provider) or None."""
    if not CACHE_ENABLED or not query.strip():
        return None
    try:
        # Exact normalized-hash hit first (fast).
        qhash = _norm_hash(query)
        with get_db_connection() as conn:
            with conn.cursor() as cur:
                cur.execute(
                    "SELECT response, provider FROM chat_response_cache WHERE query_hash = %s",
                    (qhash,),
                )
                row = cur.fetchone()
                if row:
                    cur.execute(
                        "UPDATE chat_response_cache SET hit_count = hit_count + 1, "
                        "last_hit_at = NOW() WHERE query_hash = %s",
                        (qhash,),
                    )
                    conn.commit()
                    return (row[0], row[1] or "cache")

        # Fuzzy semantic hit (slower, only if exact missed).
        qvec = get_embedder().embed_one(query)
        qvec_lit = _fmt_vec(qvec)
        with get_db_connection() as conn:
            with conn.cursor() as cur:
                cur.execute(
                    """
                    SELECT response, provider, query_embedding <=> %s::vector AS dist
                    FROM chat_response_cache
                    WHERE query_embedding IS NOT NULL
                    ORDER BY query_embedding <=> %s::vector
                    LIMIT 1
                    """,
                    (qvec_lit, qvec_lit),
                )
                row = cur.fetchone()
                if row and float(row[2]) <= CACHE_HIT_THRESHOLD:
                    return (row[0], row[1] or "cache")
    except Exception as e:
        logger.debug("cache lookup soft-failed (%s); continuing without cache", e)
    return None


def store(query: str, response: str, provider: str) -> None:
    """Persist a successful response for future reuse."""
    if not CACHE_ENABLED or not query.strip() or not response.strip():
        return
    try:
        qhash = _norm_hash(query)
        qvec_lit = _fmt_vec(get_embedder().embed_one(query))
        with get_db_connection() as conn:
            with conn.cursor() as cur:
                cur.execute(
                    """
                    INSERT INTO chat_response_cache
                        (query_hash, query_embedding, response, provider)
                    VALUES (%s, %s::vector, %s, %s)
                    ON CONFLICT (query_hash) DO UPDATE
                       SET response = EXCLUDED.response,
                           provider = EXCLUDED.provider
                    """,
                    (qhash, qvec_lit, response, provider),
                )
                conn.commit()
    except Exception as e:
        logger.debug("cache store soft-failed (%s); ignoring", e)
