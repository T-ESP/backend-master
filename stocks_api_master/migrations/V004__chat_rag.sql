-- V004: Shared RAG corpus for the AI chatbot (master DB only).
--
-- The RAG corpus is internal product/method documentation — identical for every
-- tenant — so it lives once in the master database. The Python ai-service reads
-- these tables for retrieval. Per-tenant conversation tables (chat_sessions,
-- chat_messages, ...) live in each tenant DB via sql/tenant_schema.sql instead.

CREATE EXTENSION IF NOT EXISTS vector;
CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE IF NOT EXISTS rag_documents (
    doc_id       SERIAL PRIMARY KEY,
    source_path  TEXT UNIQUE NOT NULL,
    title        TEXT,
    content_hash TEXT NOT NULL,
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS rag_chunks (
    chunk_id    SERIAL PRIMARY KEY,
    doc_id      INTEGER NOT NULL REFERENCES rag_documents(doc_id) ON DELETE CASCADE,
    chunk_index INTEGER NOT NULL,
    heading     TEXT,
    content     TEXT NOT NULL,
    embedding   vector(384) NOT NULL,
    token_count INTEGER NOT NULL DEFAULT 0,
    -- Hybrid search: full-text vector (French + English) alongside the embedding.
    content_tsv tsvector GENERATED ALWAYS AS (
        setweight(to_tsvector('french', coalesce(heading, '')), 'A') ||
        setweight(to_tsvector('french', content), 'B')
    ) STORED
);

CREATE INDEX IF NOT EXISTS rag_chunks_doc_idx ON rag_chunks(doc_id);
CREATE INDEX IF NOT EXISTS rag_chunks_embedding_hnsw
    ON rag_chunks USING hnsw (embedding vector_cosine_ops);
CREATE INDEX IF NOT EXISTS rag_chunks_tsv_idx
    ON rag_chunks USING GIN (content_tsv);
