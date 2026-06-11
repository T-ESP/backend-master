"""Embedding model wrapper.

Loads `paraphrase-multilingual-MiniLM-L12-v2` once and reuses it.
Output is L2-normalised so cosine similarity == inner product.
"""

from __future__ import annotations

import os
import threading
from typing import List, Sequence

from utils.logger import get_logger


MODEL_NAME = os.getenv(
    "EMBED_MODEL",
    "sentence-transformers/paraphrase-multilingual-MiniLM-L12-v2",
)
EMBED_DIM = 384  # MiniLM-L12 — kept in sync with the SQL schema

logger = get_logger("chat.rag.embedder")
_LOCK = threading.Lock()


class Embedder:
    """Lazy singleton wrapper around SentenceTransformer."""

    _instance: "Embedder | None" = None

    def __init__(self) -> None:
        self._model = None

    def _load(self) -> None:
        if self._model is not None:
            return
        with _LOCK:
            if self._model is not None:
                return
            from sentence_transformers import SentenceTransformer  # type: ignore
            logger.info("Loading embedding model %s", MODEL_NAME)
            self._model = SentenceTransformer(MODEL_NAME)
            logger.info("Embedding model ready (dim=%d)", EMBED_DIM)

    @property
    def dim(self) -> int:
        return EMBED_DIM

    def embed(self, texts: Sequence[str]) -> List[List[float]]:
        """Embed a batch of strings. Returns one vector per input."""
        self._load()
        assert self._model is not None
        if not texts:
            return []
        vecs = self._model.encode(
            list(texts),
            batch_size=32,
            normalize_embeddings=True,
            show_progress_bar=False,
        )
        # SentenceTransformer returns numpy arrays; convert to plain lists for psycopg.
        return [v.tolist() for v in vecs]

    def embed_one(self, text: str) -> List[float]:
        return self.embed([text])[0]


_embedder: Embedder | None = None


def get_embedder() -> Embedder:
    global _embedder
    if _embedder is None:
        _embedder = Embedder()
    return _embedder
