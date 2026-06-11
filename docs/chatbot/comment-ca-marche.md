# Comment fonctionne le chatbot

Vue d'ensemble du fonctionnement interne du chatbot StockS : ce qui se passe
entre le moment où l'utilisateur tape une question et celui où la réponse
apparaît.

## 1. Architecture en couches

```
┌──────────────┐
│   Frontend   │  Tape une question, reçoit la réponse + métadonnées
└──────┬───────┘
       │ HTTPS + JWT
       ▼
┌─────────────────────────────────────┐
│  API Rust (stocks_api, port 8090)   │  Auth, persistance des sessions,
│  module features/chat/              │  routage vers ai-service
└──────────────┬──────────────────────┘
               │ Réseau Docker interne
               ▼
┌─────────────────────────────────────────────────────────┐
│  ai-service Python (Flask, port 8001)                   │
│  ┌────────────────────────────────────────────────┐    │
│  │  Pipeline du tour :                            │    │
│  │   1. Raccourci déterministe ?                  │    │
│  │   2. Cache de réponses ?                       │    │
│  │   3. Classification d'intent                   │    │
│  │   4. RAG hybride (vector + keyword + rerank)   │    │
│  │   5. Compression historique si > 20 messages   │    │
│  │   6. Boucle outils (3 itérations max)          │    │
│  │   7. Appel LLM (Mistral / Groq / local)        │    │
│  └────────────────────────────────────────────────┘    │
└──────────────┬──────────────────────────────────────────┘
               │
       ┌───────┴────────────┐
       ▼                    ▼
┌──────────────┐    ┌──────────────────┐
│ PostgreSQL   │    │  Provider LLM    │
│  + pgvector  │    │  (selon config)  │
└──────────────┘    └──────────────────┘
```

Le **frontend ne parle qu'à Rust**. Rust orchestre tout en interne, l'utilisateur
ne voit jamais ai-service ni les LLM.

## 2. Cycle de vie d'une question

Voici ce qui se passe quand l'utilisateur envoie *"Combien de produits en
rupture critique ?"*.

### Étape 1 — Rust reçoit la requête

`POST /chat/sessions/{session_id}/messages` avec le JWT en header.

Rust :
1. Valide le JWT et identifie l'utilisateur.
2. Vérifie que la session lui appartient.
3. Persiste le message utilisateur dans `chat_messages`.
4. Auto-génère le titre de la session si c'est le premier message.
5. Charge l'historique récent (30 derniers messages).
6. Forwarde tout à ai-service `POST /chat/turn` avec le JWT de l'utilisateur.

### Étape 2 — ai-service décide quoi faire

Python applique un pipeline en cascade, où chaque étage peut court-circuiter
les suivants si une réponse est déjà trouvable :

#### Étage A — Raccourci déterministe

Avant même de toucher au LLM, on vérifie si la question correspond à un
**raccourci** connu (regex en français). Exemples :

| Question | Outil ciblé directement |
|---|---|
| « Produits en rupture / stock bas » | `get_low_stock()` |
| « Top 5 produits par CA » | `get_top_products(metric="revenue", limit=5)` |
| « Détails du produit 42 » | `get_product_detail(product_id=42)` |
| « Résumé / tableau de bord » | `get_global_kpis()` |
| « Alertes critiques » | `get_alerts(severity="CRITICAL")` |

Si un raccourci correspond : on exécute directement l'outil, puis on passe
**seulement la mise en forme** au LLM. On évite ainsi la phase la plus
fragile (le LLM qui doit décider d'appeler le bon outil avec les bons
arguments), qui est unreliable sur les petits modèles.

#### Étage B — Cache de réponses

Pour les questions **conceptuelles** (« Que signifie ABC-XYZ ? »), la réponse
ne change jamais — donc on la cache. On hash la question normalisée et on
cherche une réponse identique. Si raté, on cherche une réponse pour une
question **sémantiquement proche** via embedding (distance cosine < 0.05).

Cache hit → réponse en **< 50 ms** au lieu de **60-180 s** sur LLM local.

#### Étage C — Classification d'intent

Un classifieur **par règles regex** (pas d'appel LLM) range la question dans
une de quatre catégories :

- `doc` — question conceptuelle (« qu'est-ce que », « comment fonctionne »)
- `data` — question sur les données business (« combien », « top », noms de
  produits/fournisseurs/catégories)
- `action` — déclencher un job IA (« lance », « relance », « crée une alerte »)
- `chitchat` — bonjour, merci, etc.

Le défaut quand rien ne matche : `data` (le cas le plus utile pour un outil
de gestion de stock).

#### Étage D — Récupération de contexte (RAG, si intent = doc)

Pour les questions doc, on récupère les passages pertinents de la
**documentation indexée** dans pgvector :

1. **Recherche vectorielle** : top 20 plus proches en cosine sur l'embedding
   de la question (modèle MiniLM multilingue FR+EN, 384 dimensions).
2. **Recherche par mots-clés** : top 20 par `ts_rank_cd` sur l'index tsvector
   français de Postgres.
3. **Fusion RRF** (Reciprocal Rank Fusion) : combine les deux listes en une
   seule sans avoir à normaliser les scores.
4. **Reranker cross-encoder** (BGE-reranker-v2-m3, 280 MB) : re-score les
   20 candidats et garde les 4 réellement les plus pertinents.

Les 4 chunks finaux sont injectés dans le prompt système.

#### Étage E — Compression d'historique

Si la session dépasse 20 messages, on demande au LLM de produire un résumé
de 200 mots des messages anciens. Le résumé remplace ces messages dans le
contexte envoyé au LLM (mais les originaux restent en base). Permet de
tenir dans un contexte de 8k tokens sur des conversations longues.

#### Étage F — Appel LLM avec outils

On envoie au LLM :
- Le prompt système (persona StockS + RAG context + règles + few-shot)
- L'historique (compressé si long)
- La question utilisateur
- La liste des outils disponibles (10 outils, ou un sous-ensemble selon
  l'intent)

Le LLM produit soit :
- Une réponse texte directe (chitchat, ou doc avec RAG)
- Un appel d'outil sous forme `<tool_call>{"name": "...", "arguments": {...}}</tool_call>`

Si c'est un appel d'outil, ai-service l'exécute (en rappelant l'API Rust
avec le JWT de l'utilisateur), puis renvoie le résultat au LLM qui formule
la réponse finale en français. Max 3 itérations de boucle pour éviter
les runaways.

#### Étage G — Garde-fou actions

Si l'outil demandé est une **action sensible** (`trigger_ai_run`), il n'est
PAS exécuté. À la place, on renvoie un objet `pending_action` au frontend,
qui demande confirmation à l'utilisateur. Sur `POST /chat/sessions/{id}/confirm-action`
avec `decision: "confirm"`, Rust exécute alors l'action.

### Étape 3 — Rust persiste et renvoie

Rust reçoit la réponse de ai-service et :
1. Persiste le message assistant dans `chat_messages`.
2. Crée éventuellement un enregistrement `chat_pending_actions`.
3. Met à jour `updated_at` de la session.
4. Renvoie la réponse au frontend avec toutes les métadonnées (provider,
   intent, citations, latence, etc.).

## 3. Anatomie d'une réponse

Le frontend reçoit :

```jsonc
{
  "assistant_message": { ...le message complet... },
  "pending_action": null,            // ou { tool_name, tool_args } si confirmation requise
  "provider_used": "local",          // mistral | groq | local | cache | none
  "intent": "doc",                   // doc | data | action | chitchat
  "citations": [                     // sources RAG utilisées (pour les questions doc)
    { "source_path": "docs/ai/AI_MODELS.md", "heading": "ABC Classification", "similarity": 0.89 }
  ],
  "cached": false,                   // true si la réponse vient du cache
  "shortcut_used": null,             // ou nom de l'outil si raccourci utilisé
  "usage": {
    "tokens_in": 2204,
    "tokens_out": 672,
    "latency_ms": 68760
  }
}
```

## 4. Pourquoi cette architecture en cascade

L'idée centrale : **le moins on touche au LLM, le mieux c'est**. Chaque
étage qui court-circuite le LLM apporte :

- **Fiabilité** : un raccourci est 100 % déterministe, le LLM ~80 %.
- **Vitesse** : 0 ms vs 5-180 s.
- **Coût** : zéro token consommé (utile sur Mistral/Groq free tier).

Les LLM, surtout les petits modèles locaux (~3B), excellent dans **la mise
en forme et l'explication**, beaucoup moins dans **la décision et la
structuration**. Notre pipeline leur confie ce qu'ils savent bien faire
et garde le reste hors de leur portée.

## 5. Persistance et état

- **`chat_sessions`** — métadonnées de session (titre, provider, user)
- **`chat_messages`** — historique complet, jamais tronqué
- **`chat_pending_actions`** — actions en attente de confirmation
- **`rag_documents` + `rag_chunks`** — corpus indexé
- **`chat_response_cache`** — cache de réponses doc
- **`chat_summaries`** — résumés produits par la compression d'historique
- **`v_chat_usage_daily`** — vue d'agrégation pour quotas/billing

ai-service est **stateless** — toute la mémoire est en base. Redémarrer
ai-service ne perd aucune conversation.

## 6. Sécurité — propagation du JWT

Quand le bot appelle un outil (ex. `get_low_stock`), c'est **avec le JWT de
l'utilisateur connecté**, pas un super-token. Si l'utilisateur n'a pas le
droit de voir certaines données, le bot ne les verra pas non plus. La RBAC
de l'API Rust existante s'applique automatiquement aux outils du chatbot.
