# Chatbot — Améliorations 2026-05-09

Récapitulatif court et orienté utilisateur des nouveautés livrées le 2026-05-09.
Spec technique complète : [../superpowers/specs/2026-05-09-chatbot-improvements.md](../superpowers/specs/2026-05-09-chatbot-improvements.md).

## Ce qui change pour vous

### 🎯 Réponses plus pertinentes
- **Modèle local par défaut passé de Qwen2.5-1.5B à Qwen2.5-3B** : meilleur français, meilleur tool-calling.
- **Hybrid search** : la recherche dans la doc combine désormais sémantique (vectoriel) et littéral (tsvector). Quand vous tapez un nom de produit ou un code exact, le bot le trouve.
- **Cross-encoder reranker** (BGE-reranker-v2-m3) : les extraits de doc présentés au bot sont maintenant filtrés par un modèle qui lit vraiment les paires (question, extrait), pas juste une similarité aveugle.

### ⚡ Réponses plus rapides
- **Raccourcis déterministes** pour les questions courantes : "Quels produits sont en stock bas ?", "Top 5 par chiffre d'affaires", "Alertes critiques", etc. Le bot saute l'étape de réflexion et appelle directement l'outil concerné. Gain : ~2× plus rapide et 100% fiable sur ces phrasings.
- **Cache de réponses** pour les questions de doc/concept : la deuxième fois qu'on demande "Que signifie ABC-XYZ ?", la réponse arrive en < 100 ms (au lieu de ~60 s).
- **Compression d'historique** : au-delà de 20 messages, le bot résume les anciens turns. Vous pouvez avoir des conversations de 100+ messages sans ralentissement.

### 🔧 Tool-calling fiable même sur petit modèle
- **Grammaire GBNF** : llama.cpp est désormais contraint au niveau du décodeur à n'émettre que du JSON valide quand il appelle un outil. Fini les "je vais chercher pour vous" suivi de rien — soit du texte libre, soit un appel d'outil correct.
- **Few-shot examples** dans le prompt système : 5 exemples concrets que le modèle imite.

### 🛠️ Plus d'outils
5 nouveaux outils que le bot peut appeler :
- `compare_products(a, b)` — comparaison côte-à-côte de deux produits
- `get_sales_anomalies()` — anomalies de ventes récentes
- `get_price_anomalies()` — anomalies de prix
- `get_urgent_restocks()` — produits à réapprovisionner en urgence
- `get_price_suggestions()` — suggestions d'optimisation de prix

### 📤 Nouveaux endpoints
| Endpoint | Description |
|---|---|
| `POST /chat/sessions/:id/messages/stream` | Stream SSE — émet des événements `intent`, `shortcut`, `tool_call`, `cached`, `delta`, `done`. UX bien plus vivante. |
| `GET /chat/sessions/:id/export?format=markdown\|json` | Export d'une conversation pour archivage / partage. |

### 📊 Nouvelles données dans les réponses
La réponse de `POST /chat/sessions/:id/messages` contient maintenant :
- `citations` : liste des extraits de doc utilisés (avec `source_path`, `heading`, `similarity`). Le frontend peut afficher des liens cliquables.
- `cached: true|false` : si la réponse vient du cache.
- `shortcut_used: "<tool>"|null` : si un raccourci a été utilisé.

### 📈 Vue admin
Nouvelle vue Postgres `v_chat_usage_daily` pour les dashboards admin : tokens / latence / nb de messages, par utilisateur / par jour / par provider.

## Migration

Une nouvelle migration `V003__chat_improvements.sql` :
- Ajoute la colonne `content_tsv` (tsvector français) sur `rag_chunks` + index GIN
- Crée les tables `chat_response_cache` et `chat_summaries`
- Crée la vue `v_chat_usage_daily`

Idempotent (utilise `IF NOT EXISTS` partout). Lancée automatiquement par `docker compose up`.

## Variables d'env nouvelles

| Variable | Défaut | Effet |
|---|---|---|
| `LOCAL_LLM_USE_GRAMMAR` | `true` | Active la grammaire GBNF pour les tool-calls |
| `RAG_USE_RERANKER` | `true` | Active le reranker BGE |
| `RAG_RERANKER_MODEL` | `BAAI/bge-reranker-v2-m3` | Modèle de rerank alternatif |
| `CHAT_COMPRESS_THRESHOLD` | `20` | Compresser à partir de N messages |
| `CHAT_COMPRESS_KEEP_RECENT` | `10` | Garder les N derniers intacts |
| `CHAT_RESPONSE_CACHE` | `true` | Activer le cache de réponses |
| `CHAT_CACHE_THRESHOLD` | `0.05` | Distance cosine maximum pour un hit cache fuzzy |
| `CHAT_RAG_TOP_K` | `4` | Chunks finals après rerank |
| `CHAT_RAG_FETCH_K` | `20` | Chunks récupérés avant rerank |

## RAM — nouveau budget

| État | v1 | v2 |
|---|---|---|
| Idle | 1.6 GB | 1.6 GB |
| Chat actif (LLM local) | 3.4 GB | **4.1 GB** (3B + reranker) |
| Cron + chat actif | 5.2 GB ⚠ | **5.9 GB** ⚠⚠ |

**Si vous voulez rester light** :
- Garder le 1.5B : laisser `LOCAL_LLM_MODEL_URL` pointer vers le 1.5B GGUF (URL alternative dans `docker-compose.yml`).
- Désactiver le reranker : `RAG_USE_RERANKER=false` → -280 MB.

## Tests

- 32 tests unitaires (10 nouveaux ajoutés). Tous passent.
- Smoke test mis à jour pour exercer les nouveaux champs : `bash scripts/chat_smoke_test.sh`.

## Vérification

```sh
# 1. Rebuild des images
docker compose build

# 2. Migration applique automatiquement V001+V002+V003
docker compose up -d

# 3. Tests
docker run --rm backend-ai-service:latest python -m pytest tests/test_chat.py -v

# 4. Conversation
bash scripts/chat_smoke_test.sh
```

## Ce qui reste pour plus tard

- **Streaming token-par-token** dans le local LLM (pour l'instant `delta` est mono-chunk — protocole déjà compatible).
- **Dashboard admin** (les données sont prêtes via `v_chat_usage_daily`).
- **NL-to-SQL en read-only** (réservé à une PR avec revue sécu).
- **HyDE** (Hypothetical Document Embeddings) — bénéfice marginal sur petit corpus.
