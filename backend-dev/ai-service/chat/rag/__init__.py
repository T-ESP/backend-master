"""RAG (Retrieval-Augmented Generation) layer.

- Embedder: sentence-transformers multilingual MiniLM (FR + EN)
- Vector store: pgvector in the existing Postgres
- Indexer: scans corpus paths, hashes files, only re-embeds changes
- Retriever: top-k cosine similarity for a user query
"""

from .embedder import Embedder, get_embedder
from .indexer import index_corpus, get_index_stats
from .retriever import retrieve, retrieve_hybrid, format_context, Retrieved

__all__ = [
    "Embedder",
    "get_embedder",
    "index_corpus",
    "get_index_stats",
    "retrieve",
    "retrieve_hybrid",
    "format_context",
    "Retrieved",
]
