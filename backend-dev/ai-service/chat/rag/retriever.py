"""RAG retrieval: vector, keyword, hybrid + optional reranker.

Hybrid search combines:
  - **Vector** (pgvector cosine on the embedding column) — semantic match
  - **Keyword** (Postgres tsvector with French config) — literal match for product
    names, codes, jargon
Their results are merged with Reciprocal Rank Fusion (RRF), then optionally
reranked by a cross-encoder for the highest-quality top-k.

`retrieve()` keeps the legacy signature (vector-only) for backward compat.
`retrieve_hybrid()` is the new combined entry point used by the agent.
"""

from __future__ import annotations

import os
from dataclasses import dataclass
from typing import List

from utils.logger import get_logger
# RAG corpus lives in the master DB; pin to it so an in-progress per-tenant ML
# run (which switches the shared pool) can't redirect retrieval to a tenant DB.
from database.connection import get_master_db_connection as get_db_connection

from .embedder import get_embedder


logger = get_logger("chat.rag.retriever")

DEFAULT_TOP_K = 5
DEFAULT_FETCH_K = 20  # Cast a wider net before reranking
DEFAULT_CONTEXT_TOKEN_BUDGET = 2000
RRF_K = 60  # Standard RRF constant


@dataclass
class Retrieved:
    chunk_id: int
    doc_id: int
    source_path: str
    heading: str
    content: str
    similarity: float  # Higher = better. After rerank, this is the cross-encoder score.


def _fmt_vec(vec: list[float]) -> str:
    return "[" + ",".join(f"{v:.7f}" for v in vec) + "]"


def _vector_search(query: str, top_k: int) -> List[Retrieved]:
    embedder = get_embedder()
    qvec = embedder.embed_one(query)
    qvec_lit = _fmt_vec(qvec)
    with get_db_connection() as conn:
        with conn.cursor() as cur:
            cur.execute(
                """
                SELECT c.chunk_id, c.doc_id, d.source_path, c.heading, c.content,
                       1 - (c.embedding <=> %s::vector) AS similarity
                FROM rag_chunks c
                JOIN rag_documents d ON c.doc_id = d.doc_id
                ORDER BY c.embedding <=> %s::vector
                LIMIT %s
                """,
                (qvec_lit, qvec_lit, top_k),
            )
            rows = cur.fetchall()
    return [
        Retrieved(
            chunk_id=r[0], doc_id=r[1], source_path=r[2],
            heading=r[3] or "", content=r[4], similarity=float(r[5]),
        )
        for r in rows
    ]


def _keyword_search(query: str, top_k: int) -> List[Retrieved]:
    """tsvector full-text search against the rag_chunks.content_tsv column."""
    with get_db_connection() as conn:
        with conn.cursor() as cur:
            try:
                cur.execute(
                    """
                    SELECT c.chunk_id, c.doc_id, d.source_path, c.heading, c.content,
                           ts_rank_cd(c.content_tsv, q) AS rank
                    FROM rag_chunks c
                    JOIN rag_documents d ON c.doc_id = d.doc_id,
                         plainto_tsquery('french', %s) q
                    WHERE c.content_tsv @@ q
                    ORDER BY rank DESC
                    LIMIT %s
                    """,
                    (query, top_k),
                )
                rows = cur.fetchall()
            except Exception as e:
                # tsvector column may not exist yet (pre-V003); soft-fail.
                logger.debug("Keyword search unavailable (%s); skipping", e)
                return []
    return [
        Retrieved(
            chunk_id=r[0], doc_id=r[1], source_path=r[2],
            heading=r[3] or "", content=r[4], similarity=float(r[5]),
        )
        for r in rows
    ]


def _rrf_merge(rank_lists: list[list[Retrieved]], k: int = RRF_K) -> List[Retrieved]:
    """Reciprocal Rank Fusion: combine multiple rankings into one.

    Each chunk gets sum(1 / (k + rank_in_list)) across lists where it appears.
    More robust than weighted-sum because it doesn't need score normalisation.
    """
    scores: dict[int, float] = {}
    by_id: dict[int, Retrieved] = {}
    for lst in rank_lists:
        for rank, hit in enumerate(lst, start=1):
            scores[hit.chunk_id] = scores.get(hit.chunk_id, 0.0) + 1.0 / (k + rank)
            by_id[hit.chunk_id] = hit
    merged = [by_id[i] for i, _ in sorted(scores.items(), key=lambda x: x[1], reverse=True)]
    # Overwrite similarity with the RRF score for downstream sorting/ranking.
    for h in merged:
        h.similarity = scores[h.chunk_id]
    return merged


def retrieve_hybrid(query: str, *, fetch_k: int = DEFAULT_FETCH_K,
                    top_k: int = DEFAULT_TOP_K,
                    use_reranker: bool = True) -> List[Retrieved]:
    """Hybrid: vector + keyword merged via RRF, optionally cross-encoder reranked."""
    if not query.strip():
        return []
    vec = _vector_search(query, fetch_k)
    kw = _keyword_search(query, fetch_k)
    merged = _rrf_merge([vec, kw]) if (vec or kw) else []
    if not merged:
        return []
    if use_reranker:
        try:
            from .reranker import rerank
            return rerank(query, merged, top_k=top_k)
        except Exception as e:
            logger.warning("rerank failed (%s); falling back to RRF order", e)
    return merged[:top_k]


def retrieve(query: str, *, top_k: int = DEFAULT_TOP_K) -> List[Retrieved]:
    """Backward-compatible vector-only retrieval (used by tests + search_docs tool).

    For agent calls, prefer `retrieve_hybrid`.
    """
    return _vector_search(query, top_k)


def format_context(
    hits: List[Retrieved],
    *,
    token_budget: int = DEFAULT_CONTEXT_TOKEN_BUDGET,
) -> str:
    if not hits:
        return ""
    parts: list[str] = ["Voici des extraits de la documentation pertinents :"]
    used = 0
    for h in hits:
        rough_tokens = max(1, len(h.content) // 4)
        if used + rough_tokens > token_budget:
            break
        loc = f"[{h.source_path}{(' — ' + h.heading) if h.heading else ''}]"
        parts.append(f"{loc}\n{h.content}")
        used += rough_tokens
    parts.append(
        "\nUtilise ces extraits en priorité pour répondre. Cite la source entre crochets quand pertinent."
    )
    return "\n\n".join(parts)
