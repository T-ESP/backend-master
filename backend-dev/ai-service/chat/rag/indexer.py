"""Corpus indexer.

Walks one or more directories, picks up every .md file, hashes it, and only
re-embeds files whose content changed. Persists to pgvector via psycopg2.
"""

from __future__ import annotations

import hashlib
import os
from pathlib import Path
from typing import Iterable, List

from utils.logger import get_logger
from database.connection import get_db_connection

from .chunker import Chunk, chunk_markdown
from .embedder import get_embedder, EMBED_DIM


CORPUS_PATHS_ENV = os.getenv("RAG_CORPUS_PATHS", "/app/corpus")

logger = get_logger("chat.rag.indexer")


def _list_corpus_files() -> list[Path]:
    """Resolve corpus paths from env. Each entry can be a file or a dir.

    We pick up *.md files recursively from directories.
    """
    raw = [p.strip() for p in CORPUS_PATHS_ENV.split(":") if p.strip()]
    files: list[Path] = []
    for entry in raw:
        p = Path(entry)
        if not p.exists():
            logger.warning("Corpus path %s does not exist, skipping", p)
            continue
        if p.is_file():
            files.append(p)
        else:
            files.extend(sorted(p.rglob("*.md")))
    return files


def _hash_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for block in iter(lambda: f.read(1 << 20), b""):
            h.update(block)
    return h.hexdigest()


def _format_vector(vec: list[float]) -> str:
    """psycopg2 doesn't know about pgvector — feed it the literal `[a,b,c]` form."""
    return "[" + ",".join(f"{v:.7f}" for v in vec) + "]"


def index_corpus(force: bool = False) -> dict:
    """Index every markdown file under the configured corpus paths.

    `force=True` re-embeds even unchanged files.

    Returns a metrics dict (number of files seen, embedded, chunks written).
    """
    files = _list_corpus_files()
    if not files:
        logger.warning("No corpus files found — RAG will be empty.")
        return {"files_seen": 0, "files_embedded": 0, "chunks_written": 0, "skipped": 0}

    embedder = get_embedder()

    files_embedded = 0
    chunks_written = 0
    skipped = 0

    with get_db_connection() as conn:
        with conn.cursor() as cur:
            for path in files:
                file_hash = _hash_file(path)
                source_path = str(path)

                cur.execute(
                    "SELECT doc_id, content_hash FROM rag_documents WHERE source_path = %s",
                    (source_path,),
                )
                row = cur.fetchone()

                if row and row[1] == file_hash and not force:
                    skipped += 1
                    continue

                # Read + chunk
                try:
                    text = path.read_text(encoding="utf-8")
                except UnicodeDecodeError:
                    logger.warning("Skipping non-utf8 file %s", path)
                    continue

                chunks = chunk_markdown(text)
                if not chunks:
                    skipped += 1
                    continue

                # Embed all chunks in one batch (faster).
                vectors = embedder.embed([c.content for c in chunks])

                title = _derive_title(text, path)

                if row:
                    doc_id = row[0]
                    # Wipe old chunks before re-embedding.
                    cur.execute("DELETE FROM rag_chunks WHERE doc_id = %s", (doc_id,))
                    cur.execute(
                        "UPDATE rag_documents SET title = %s, content_hash = %s, "
                        "updated_at = NOW() WHERE doc_id = %s",
                        (title, file_hash, doc_id),
                    )
                else:
                    cur.execute(
                        "INSERT INTO rag_documents (source_path, title, content_hash) "
                        "VALUES (%s, %s, %s) RETURNING doc_id",
                        (source_path, title, file_hash),
                    )
                    doc_id = cur.fetchone()[0]

                for chunk, vec in zip(chunks, vectors):
                    if len(vec) != EMBED_DIM:
                        logger.error("Embedding dim mismatch (%d vs expected %d)",
                                     len(vec), EMBED_DIM)
                        continue
                    cur.execute(
                        "INSERT INTO rag_chunks "
                        "(doc_id, chunk_index, heading, content, embedding, token_count) "
                        "VALUES (%s, %s, %s, %s, %s::vector, %s)",
                        (doc_id, chunk.chunk_index, chunk.heading, chunk.content,
                         _format_vector(vec), chunk.token_count),
                    )
                    chunks_written += 1

                files_embedded += 1
                logger.info("Indexed %s (%d chunks)", path, len(chunks))

        conn.commit()

    return {
        "files_seen": len(files),
        "files_embedded": files_embedded,
        "chunks_written": chunks_written,
        "skipped": skipped,
    }


def _derive_title(text: str, path: Path) -> str:
    """Use the first H1 as title; fall back to the file name."""
    for line in text.splitlines():
        s = line.strip()
        if s.startswith("# "):
            return s[2:].strip()
    return path.stem.replace("_", " ")


def get_index_stats() -> dict:
    """Return current corpus stats — for /rag/stats."""
    with get_db_connection() as conn:
        with conn.cursor() as cur:
            cur.execute("SELECT COUNT(*) FROM rag_documents")
            n_docs = cur.fetchone()[0]
            cur.execute("SELECT COUNT(*) FROM rag_chunks")
            n_chunks = cur.fetchone()[0]
            cur.execute(
                "SELECT MAX(updated_at) FROM rag_documents"
            )
            last_indexed = cur.fetchone()[0]
    return {
        "documents": n_docs,
        "chunks": n_chunks,
        "last_indexed": last_indexed.isoformat() if last_indexed else None,
        "embed_model": os.getenv("EMBED_MODEL"),
        "embed_dim": EMBED_DIM,
    }
