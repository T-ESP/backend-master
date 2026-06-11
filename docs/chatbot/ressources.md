# Ressources nécessaires

Tout ce qu'il faut savoir pour dimensionner un serveur capable de faire
tourner le chatbot StockS — RAM, CPU, disque, réseau.

## 1. Cible de référence : VPS 4 cœurs / 8 GB

Toute l'architecture est dimensionnée pour ce profil typique de VPS bon
marché (environ 10–20 €/mois chez OVH, Hetzner, Scaleway, etc.).

**Plafond de consommation visé : 5 GB de RAM** (laisse 3 GB pour l'OS et
les pointes).

## 2. Budget RAM détaillé par scénario

### Scénario A — Idle (rien ne tourne)

| Composant | RAM |
|---|---|
| OS + Docker overhead | ~600 MB |
| PostgreSQL 16 + pgvector | ~500 MB |
| API Rust (stocks_api) | ~80 MB |
| ai-service Flask | ~250 MB |
| Embedder MiniLM (toujours chargé) | ~150 MB |
| pgadmin (optionnel) | ~200 MB |
| **Total** | **~1.8 GB** |

### Scénario B — Chat actif avec API distante (Mistral/Groq)

Identique à A, plus une centaine de Mo pour les requêtes HTTP en vol.
**Total : ~1.9 GB.**

### Scénario C — Chat actif avec LLM local (Qwen2.5-3B)

| Composant | RAM |
|---|---|
| Idle (A) | 1.8 GB |
| Qwen2.5-3B Q4_K_M chargé | ~2.5 GB |
| Reranker BGE-reranker-v2-m3 | ~280 MB |
| Buffers d'inférence | ~150 MB |
| **Total** | **~4.7 GB** |

### Scénario D — Pire cas : cron ML + chat local simultané

| Composant | RAM |
|---|---|
| C (chat local actif) | 4.7 GB |
| Prophet / sklearn / clustering job | ~1.5 GB |
| **Total** | **~6.2 GB** ⚠ |

**Mitigation déjà en place** : quand `POST /ai/run` est appelé, le scheduler
**décharge le LLM local** avant de lancer les jobs lourds. Recharger prend
~10 s. Avec cette mitigation, le pire cas redescend à ~4.5 GB.

### Scénario E — Sans LLM local (uniquement API)

Si on met `LLM_PROVIDER=mistral` ou `=groq`, on ne charge jamais le modèle
local. La RAM plafonne à ~2 GB même en pleine charge.

## 3. Budget CPU

Le profil 4 cœurs suffit largement pour un usage interne (équipe de 5-20
utilisateurs concurrents).

| Activité | Cœurs occupés |
|---|---|
| Idle | < 5 % d'un cœur |
| Une requête chat avec Mistral/Groq | 1 cœur pendant 1-2 s |
| Une requête chat avec LLM local | 3 cœurs pendant 5-20 s |
| Indexation RAG (au démarrage) | 1 cœur pendant ~30 s |
| Cron ML (batch jobs Prophet/sklearn) | 4 cœurs pendant 5-15 min |

**Concurrence** :

- Les requêtes Mistral/Groq sont I/O bound (réseau) — on peut en faire
  beaucoup en parallèle.
- Les requêtes locales sont **sérialisées** via un mutex (`_LLAMA_LOCK`).
  Une seule inférence à la fois. Pour la concurrence, augmenter le nombre
  de workers Flask + utiliser Mistral en parallèle.

## 4. Budget disque

| Élément | Taille | Notes |
|---|---|---|
| Image Docker `backend-web` (Rust) | ~180 MB | Binaires statiques |
| Image Docker `backend-ai-service` | ~9 GB | Python + torch + sentence-transformers + llama.cpp + prophet |
| Volume `db_data` (Postgres) | ~200 MB | Avec seed data |
| Volume `ai_models` (modèles ML cron) | ~50 MB | Modèles Prophet / sklearn entraînés |
| Volume `llm_models` (LLM local) | ~2 GB | Qwen2.5-3B GGUF |
| Volume `embed_cache` (HuggingFace) | ~700 MB | MiniLM + BGE-reranker + tokenizers |
| **Total minimum disque libre** | **~13 GB** | Pour démarrer proprement |

**Conseil** : prévoir 20 GB minimum sur le VPS pour avoir de la marge
(builds Docker temporaires, rotation de logs, backups).

## 5. Budget réseau

### Premier démarrage

Le premier `docker compose up` télécharge :

- Images Docker depuis Docker Hub (~2-3 GB de couches)
- Modèle LLM local depuis HuggingFace (~2 GB)
- Embedder + reranker depuis HuggingFace (~400 MB)

**Total premier boot : ~5 GB de bande passante.** Sur une connexion
correcte, comptez 10-30 minutes.

### Runtime

- Avec LLM local : **zéro trafic sortant** (à part les requêtes JWT/users
  depuis le frontend, négligeable).
- Avec Mistral : ~5 KB par tour de chat (compressé).
- Avec Groq : ~5 KB par tour.
- Mises à jour Docker : à votre rythme, pas automatique.

## 6. Disque dur — SSD vs HDD

**SSD obligatoire** pour :

- PostgreSQL (les recherches pgvector sont I/O sensibles)
- Le volume `llm_models` (le LLM mmap-read le fichier GGUF — un HDD
  divise les tokens/s par 5)

SSD NVMe préféré si possible, mais SATA SSD est suffisant.

## 7. Comment dégrader gracieusement si le VPS est plus petit

### VPS 4 GB de RAM

Trop juste pour le LLM local. Options :

1. **Désactiver le local** : `LLM_PROVIDER=mistral` (ou `groq`), configurer
   les clés API.
2. **Garder le local mais alléger** : passer à Qwen2.5-1.5B (~1.8 GB).
   Désactiver le reranker (`RAG_USE_RERANKER=false`) — gain de 280 MB.
3. **Désactiver pgadmin** : retirer la section `pgadmin` du compose. Gain
   ~200 MB.
4. **Désactiver les batch jobs cron au démarrage** : `RUN_ON_STARTUP=false`
   dans le compose. Évite le pic de RAM au démarrage.

Total avec ces 4 mesures : tient sur ~2.5 GB.

### VPS 16+ GB de RAM

Tu peux te permettre :

- Modèle local **plus gros** : Qwen2.5-7B (~5 GB) ou Llama-3.1-8B (~5.5 GB)
- Reranker plus précis : `BAAI/bge-reranker-v2-gemma` (~4 GB) au lieu de
  v2-m3.
- Cache de contexte plus grand : `LOCAL_LLM_CTX=16384` ou `=32768` (pour
  les modèles qui le supportent comme Qwen2.5).
- Plusieurs workers Flask en parallèle : pour servir plusieurs requêtes
  locales simultanément.

## 8. Latence attendue selon la configuration

Pour une question moyenne (RAG + génération de ~500 tokens) :

| Configuration | Latence par tour |
|---|---|
| Cache hit (réponse déjà vue) | **< 50 ms** |
| Raccourci + Mistral (formatage) | ~1-2 s |
| Raccourci + Groq (formatage) | ~500 ms |
| Raccourci + LLM local | ~5-15 s |
| RAG complet + Mistral | ~2-4 s |
| RAG complet + Groq | ~1-2 s |
| RAG complet + LLM local (Qwen 3B, 4 cœurs CPU) | ~30-90 s |
| Premier appel local (chargement du modèle) | +10 s |
| Premier appel local (téléchargement du modèle) | +2-5 min |

## 9. Surveiller la consommation en production

Commandes utiles :

```sh
# RAM par container
docker stats --no-stream

# Logs ai-service (voir si OOM, si fallback s'active)
docker logs -f backend-ai-service-1

# État Postgres
docker exec backend-db-1 psql -U user -d stocks -c "\dt+ "

# Taille des volumes
docker system df -v
```

Indicateurs d'alerte :

- ai-service consomme > 5 GB → modèle trop gros ou fuite
- Postgres > 1 GB → vérifier `chat_messages` (purge si on dépasse 100 k
  lignes)
- Volume `llm_models` > 5 GB → vérifier qu'on n'a pas téléchargé plusieurs
  modèles (un seul devrait suffire)

## 10. Quotas suggérés par utilisateur

Si tu ouvres le chatbot à plusieurs utilisateurs et que tu veux limiter
les coûts (Mistral/Groq) ou la charge VPS (local) :

| Profil | Suggéré |
|---|---|
| Démo / interne (5-10 users) | Pas de quota |
| Production légère (~50 users actifs/jour) | 50 messages/user/jour |
| Production forte (> 100 users) | 30 messages/user/jour + rate-limit 1 req/15s |

Implémentation : la vue `v_chat_usage_daily` aggrège tout ce qu'il faut.
Reste à brancher un middleware qui vérifie avant d'accepter un nouveau
`POST /chat/sessions/.../messages`. À faire dans une PR séparée si le
besoin se concrétise.
