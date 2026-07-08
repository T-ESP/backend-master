# Chatbot StockS — Démarrage rapide

Assistant conversationnel français qui s'intègre à l'API StockS existante.
Parle à une couche LLM Groq (primaire) + Mistral (fallback), récupère du
contexte dans la doc projet via RAG pgvector, et appelle les endpoints API
existants comme outils.

## Où se trouve quoi

- Spec design : [../superpowers/specs/2026-05-07-stock-rag-chatbot-design.md](../superpowers/specs/2026-05-07-stock-rag-chatbot-design.md)
- Python (RAG, providers, outils, agent) : [`ai-service/chat/`](../../ai-service/chat/)
- Module Rust (sessions, messages, admin) : [`stocks_api/src/features/chat/`](../../stocks_api/src/features/chat/)
- Migrations : [`V002__chat_and_rag.sql`](../../stocks_api/migrations/V002__chat_and_rag.sql), [`V003__chat_improvements.sql`](../../stocks_api/migrations/V003__chat_improvements.sql)
- docker-compose : utilise `pgvector/pgvector:pg16`, monte les docs

## Lancer

1. Copier le template d'env :
   ```sh
   cp .env.chatbot.example .env
   # Requis : GROQ_API_KEY (gratuit sur https://console.groq.com).
   # Optionnel : MISTRAL_API_KEY pour l'auto-fallback en cas de 429 Groq.
   ```

2. Monter la stack :
   ```sh
   docker compose up -d --build
   ```

3. Au premier démarrage, ai-service :
   - télécharge l'embedder multilingue MiniLM (~120 MB, dans `embed_cache`)
   - télécharge le reranker BGE-reranker-v2-m3 (~280 MB)
   - indexe tous les `*.md` du `/app/corpus` (monté depuis `docs/` etc.)

   Tout ça tourne en thread de fond ; le service répond immédiatement aux
   endpoints de santé.

4. Se connecter en tant qu'admin de la seed :
   ```sh
   curl -s -X POST http://localhost:8090/auth/login \
     -H 'content-type: application/json' \
     -d '{"email":"admin@example.com","password":"adminpass"}' | jq -r .data.token
   ```

   Sauvegarder le token comme `$T`.

5. Vérifications rapides :
   ```sh
   # Disponibilité des providers
   curl -s http://localhost:8090/admin/chat/providers -H "authorization: Bearer $T" | jq

   # Stats du RAG (chemin interne ai-service, utile pour debug)
   curl -s http://localhost:8001/rag/stats | jq

   # Forcer une réindexation
   curl -s -X POST http://localhost:8090/admin/rag/reindex -H "authorization: Bearer $T" | jq
   ```

6. Lancer une conversation :
   ```sh
   SID=$(curl -s -X POST http://localhost:8090/chat/sessions \
     -H "authorization: Bearer $T" -H 'content-type: application/json' \
     -d '{"title":"Test"}' | jq -r .data.session_id)

   curl -s -X POST "http://localhost:8090/chat/sessions/$SID/messages" \
     -H "authorization: Bearer $T" -H 'content-type: application/json' \
     -d '{"content":"Combien de produits sont en rupture critique ?"}' | jq
   ```

7. Ou plus simple : lancer le script de smoke test complet :
   ```sh
   bash scripts/chat_smoke_test.sh
   ```

## Résumé des endpoints (côté Rust)

| Méthode | Chemin | Description |
|---|---|---|
| `POST`   | `/chat/sessions` | Créer une session |
| `GET`    | `/chat/sessions` | Lister les sessions |
| `GET`    | `/chat/sessions/:id` | Session + messages |
| `DELETE` | `/chat/sessions/:id` | Supprimer |
| `POST`   | `/chat/sessions/:id/messages` | Envoyer un message |
| `POST`   | `/chat/sessions/:id/confirm-action` | Confirmer/annuler une action write |
| `GET`    | `/chat/sessions/:id/export?format=markdown` | Exporter la conversation |
| `GET`    | `/admin/chat/providers` | Santé des providers LLM |
| `POST`   | `/admin/rag/reindex` | Forcer la réindexation du corpus |

Swagger UI : `http://localhost:8090/swagger-ui`

## Changer de provider par requête

```sh
# Forcer Groq pour un tour spécifique :
curl -s -X POST "http://localhost:8090/chat/sessions/$SID/messages" \
  -H "authorization: Bearer $T" -H 'content-type: application/json' \
  -d '{"content":"Top 3 catégories ?", "provider":"groq"}'
```

`provider` accepte `auto | groq | mistral`.

## Outils disponibles pour le bot

**Lecture seule (aucune confirmation requise) :**

- `get_global_kpis(period_days)` → wrappe `/ai/insights`
- `get_top_products(metric, limit)` → wrappe `/kpis/top-flop`
- `get_product_detail(product_id)` → wrappe `/products/:id` + sous-routes KPI
- `get_alerts(severity, limit)` → wrappe `/alerts`
- `get_low_stock()` → wrappe `/stocks/low-stock`
- `get_supplier_score(supplier_id)` → wrappe `/suppliers/:id/profile`
- `get_forecast(product_id)` → wrappe `/ai/forecasts/:id`
- `get_classification(product_id)` → wrappe `/ai/classifications/:id`
- `compare_products(product_id_a, product_id_b)` → comparaison côte-à-côte
- `get_sales_anomalies(limit)` → wrappe `/ai/sales-anomalies`
- `get_price_anomalies(limit)` → wrappe `/ai/price-anomalies`
- `get_urgent_restocks()` → wrappe `/ai/urgent-restocks`
- `get_price_suggestions(limit)` → wrappe `/ai/price-suggestions`
- `search_docs(query, top_k)` → recherche RAG dans la doc projet

**Écriture (nécessitent confirmation) :**

- `trigger_ai_run()` → re-lance tous les jobs IA batch

## Mémoire / RAM

VPS 8 GB — plafond très confortable depuis le retrait du LLM local :

| État | Total |
|---|---|
| Idle | ~1.5 GB |
| Chat actif (Groq / Mistral) | ~1.9 GB |
| Cron ML + chat actif | ~2.6 GB |

Les LLM tournent côté Groq/Mistral, donc plus rien de lourd à charger côté
ai-service — seuls l'embedder MiniLM et le reranker BGE restent en RAM.
