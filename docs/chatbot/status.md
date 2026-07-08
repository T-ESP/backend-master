# Chatbot — État de l'implémentation

**Branche :** `chatbot`
**Date dernière mise à jour :** 2026-07-08

## Ce qui est fait et vérifié

### Code livré

- [Spec design](../superpowers/specs/2026-05-07-stock-rag-chatbot-design.md) — design complet approuvé avant le code.
- [Spec améliorations 2026-05-09](../superpowers/specs/2026-05-09-chatbot-improvements.md) — second tour d'améliorations.
- [V002__chat_and_rag.sql](../../stocks_api/migrations/V002__chat_and_rag.sql) — extension pgvector + 5 tables.
- [V003__chat_improvements.sql](../../stocks_api/migrations/V003__chat_improvements.sql) — hybrid search, cache, summaries.
- [docker-compose.yml](../../docker-compose.yml) — DB en `pgvector/pgvector:pg16`, mounts docs.
- [`ai-service/chat/`](../../ai-service/chat/) — module Python complet :
  - `types.py` — Message / ToolCall / ToolSpec / ChatResponse agnostiques.
  - `llm/` — interface `base`, `groq_provider` (primaire, Llama-3.3-70B), `mistral_provider` (fallback), `factory` avec auto-fallback.
  - `rag/` — embedder MiniLM, chunker markdown, indexer incrémental, retriever vector + keyword (hybrid) + reranker BGE.
  - `tools/` — 14 outils dont 13 read-only et 1 write (`trigger_ai_run`).
  - `agent/` — shortcuts déterministes, cache de réponses, compression historique, intent router, orchestrator avec boucle outils.
  - `routes.py` — blueprint Flask : `/chat/turn`, `/rag/reindex`, `/rag/stats`, `/llm/health`.
- [`stocks_api/src/features/chat/`](../../stocks_api/src/features/chat/) — module Rust complet :
  - `dto.rs` — sessions, messages, citations, pending actions, admin DTOs.
  - `services.rs` — requêtes sqlx async pour toutes les tables chat.
  - `ai_client.rs` — wrapper reqwest vers ai-service.
  - `handlers.rs` — 9 handlers avec auth + ownership checks.
  - `router.rs` — toutes les routes incluant export et confirm-action.
- Cargo.toml — `uuid`, `reqwest` ajoutés.
- OpenAPI — tous les handlers + DTOs enregistrés.
- 32 tests Python unitaires — tous passent.
- Script smoke test — [`scripts/chat_smoke_test.sh`](../../scripts/chat_smoke_test.sh).
- [`.env.chatbot.example`](../../.env.chatbot.example).

### État du build

- `backend-web` (Rust) — build clean ✓
- `backend-ai-service` (Python) — image allégée après retrait de llama-cpp-python
- DB en place, migrations V001 + V002 + V003 appliquées ✓
- Extension pgvector + 5 tables chat/RAG + 2 tables d'amélioration + 1 vue ✓
- Login + JWT fonctionnent ✓

## Changement 2026-07-08 — retrait du provider local

Le provider local (Qwen2.5 via llama.cpp) a été retiré (voir factory / compose).
Raisons :

- Qualité tool-calling insuffisante sur 1.5B–3B pour un usage sérieux, même
  avec GBNF grammar et few-shot.
- Latence CPU (5–20 s/tour) inacceptable vs Groq (< 1 s).
- Poids : ~2 GB de modèle GGUF + `llama-cpp-python` gonflaient l'image d'env.
  ~7 GB et alourdissaient les builds.

Défaut désormais : Groq (Llama-3.3-70B) en primaire, Mistral en fallback si
`MISTRAL_API_KEY` est défini.

## Limitations connues

### Streaming SSE — non implémenté

L'endpoint `POST /chat/sessions/:id/messages/stream` est dans la roadmap.
Pour l'instant on n'a que le non-streaming. Le frontend doit montrer un
spinner pendant la génération (cf. [frontend.md](frontend.md) section 7).

### Garde admin sur `/admin/*`

Actuellement toute personne avec un JWT valide peut hitter
`/admin/chat/providers` et `/admin/rag/reindex`. Pas critique pour un dev,
à durcir en prod via une vérification de rôle dans l'auth middleware.

## Notes opérationnelles

### Bug refinery contourné

refinery 0.8's `embed_migrations!()` corrompt les métadonnées de V002 dans
le binaire Windows-built (cf. commit `da8d0b0`). On a abandonné le runner
refinery au profit de notre propre applicateur dans
[`migrate.rs`](../../stocks_api/src/bin/migrate.rs) qui :

- bundle V001 + V002 + V003 via `include_str!()`
- applique chacun dans une transaction
- enregistre dans `refinery_schema_history` pour compatibilité tooling

Idempotent : les reruns skippent les migrations déjà appliquées.

### Switch image Postgres

Le `db` est passé de `postgres:16-alpine` à `pgvector/pgvector:pg16`.
Mêmes données on-disk, mais Alpine (musl) → Debian (glibc) peut changer
le comportement de collation sur les index texte. Si une base existante
était conservée à travers le switch, lancer `REINDEX DATABASE stocks` une
fois. Pour un dev fraîchement seedé, RAS.

## Pistes pour la suite (non implémentées)

Voir [improvements-2026-05-09.md](improvements-2026-05-09.md) pour le tour
d'améliorations le plus récent. Pistes encore ouvertes :

- Endpoint SSE streaming
- NL→SQL en read-only sur une vue restreinte (couvre les cas tordus que
  les outils prédéfinis manquent)
- Garde admin sur `/admin/*`
- UI frontend (séparée — voir [frontend.md](frontend.md) pour le guide
  d'intégration)
- Quotas par utilisateur basés sur `v_chat_usage_daily`
- Reranker plus précis (BGE-reranker-v2-gemma, 4 GB) pour les VPS gros
