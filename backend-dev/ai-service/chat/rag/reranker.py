"""Cross-encoder reranker.

Vector retrieval gives us "semantically close" but not always "actually relevant".
A cross-encoder reads (query, chunk) pairs and scores each one, much more
accurately than the bi-encoder cosine similarity used at retrieval time.

Default model: BGE-reranker-v2-m3 — multilingual (FR + EN), ~280 MB, fast on CPU.
We rerank top-20 retrievals down to top-K (default 4) actually-best chunks.

Disabled if RAG_USE_RERANKER=false (env var) or if the model fails to load.
"""

from __future__ import annotations

import os
import threading
from typing import List, Sequence

from utils.logger import get_logger

from .retriever import Retrieved


MODEL_NAME = os.getenv("RAG_RERANKER_MODEL", "BAAI/bge-reranker-v2-m3")

logger = get_logger("chat.rag.reranker")
_LOCK = threading.Lock()
_MODEL = None
_DISABLED = False


def _enabled() -> bool:
    return os.getenv("RAG_USE_RERANKER", "true").lower() == "true"


def _load() -> "CrossEncoder | None":
    """Lazy-load the cross-encoder. Returns None if disabled or unavailable."""
    global _MODEL, _DISABLED
    if _DISABLED or not _enabled():
        return None
    if _MODEL is not None:
        return _MODEL
    with _LOCK:
        if _MODEL is not None:
            return _MODEL
        try:
            from sentence_transformers import CrossEncoder  # type: ignore
            logger.info("Loading reranker model %s", MODEL_NAME)
            _MODEL = CrossEncoder(MODEL_NAME, max_length=512)
            logger.info("Reranker ready.")
        except Exception as e:
            logger.warning("Reranker unavailable (%s); falling back to vector ordering", e)
            _DISABLED = True
            return None
    return _MODEL


def rerank(query: str, hits: Sequence[Retrieved], top_k: int = 4) -> List[Retrieved]:
    """Re-order `hits` by query-conditioned relevance, keep top_k.

    If the reranker is unavailable, returns the first `top_k` of the input
    unchanged so callers don't need to special-case it.
    """
    if not hits:
        return []
    model = _load()
    if model is None:
        return list(hits[:top_k])

    pairs = [(query, h.content) for h in hits]
    try:
        scores = model.predict(pairs, show_progress_bar=False)
    except Exception as e:
        logger.warning("Reranker scoring failed (%s); falling back", e)
        return list(hits[:top_k])

    scored = list(zip(hits, [float(s) for s in scores]))
    scored.sort(key=lambda x: x[1], reverse=True)
    # Re-attach the cross-encoder score as the "similarity" so the prompt
    # context formatter prioritises by reranker score, not original cosine.
    out: list[Retrieved] = []
    for h, s in scored[:top_k]:
        h.similarity = s
        out.append(h)
    return out
