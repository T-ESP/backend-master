# Ressources nécessaires

Tout ce qu'il faut savoir pour dimensionner un serveur capable de faire
tourner le chatbot StockS — RAM, CPU, disque, réseau.

> Depuis 2026-07, le provider LLM local a été retiré. Toute l'inférence LLM
> passe par Groq (primaire) + Mistral (fallback), donc l'ai-service n'a plus
> de gros modèle en RAM ni de fichier GGUF sur disque.

## 1. Cible de référence : VPS 2 cœurs / 4 GB

Depuis le retrait du LLM local, la stack tient largement dans ce format.
Un VPS bon marché (5–10 €/mois) suffit.

## 2. Budget RAM

| Composant | RAM |
|---|---|
| OS + Docker overhead | ~600 MB |
| PostgreSQL 16 + pgvector | ~500 MB |
| API Rust (stocks_api) | ~80 MB |
| ai-service Flask | ~250 MB |
| Embedder MiniLM (toujours chargé) | ~150 MB |
| Reranker BGE-reranker-v2-m3 | ~280 MB |
| pgadmin (optionnel) | ~200 MB |
| **Total idle** | **~1.5 GB** |
| **Total en charge (chat + cron ML)** | **~2.6 GB** |

## 3. Budget CPU

| Activité | Cœurs occupés |
|---|---|
| Idle | < 5 % d'un cœur |
| Une requête chat (Groq/Mistral) | 1 cœur pendant 0.5-2 s |
| Indexation RAG (au démarrage) | 1 cœur pendant ~30 s |
| Cron ML (batch jobs Prophet/sklearn) | 2 cœurs pendant 5-15 min |

Les requêtes Groq/Mistral sont I/O bound (réseau) — on peut en faire
beaucoup en parallèle. La concurrence est limitée par le rate-limit du
provider, pas par le CPU local.

## 4. Budget disque

| Élément | Taille | Notes |
|---|---|---|
| Image Docker `backend-web` (Rust) | ~180 MB | Binaires statiques |
| Image Docker `backend-ai-service` | ~2 GB | Python + torch CPU + sentence-transformers + prophet |
| Volume `db_data` (Postgres) | ~200 MB | Avec seed data |
| Volume `ai_models` (modèles ML cron) | ~50 MB | Modèles Prophet / sklearn entraînés |
| Volume `embed_cache` (HuggingFace) | ~700 MB | MiniLM + BGE-reranker + tokenizers |
| **Total minimum disque libre** | **~4 GB** | Pour démarrer proprement |

Prévoir 10 GB pour la marge (builds Docker temporaires, rotation de logs,
backups).

## 5. Budget réseau

### Premier démarrage

- Images Docker depuis Docker Hub (~2 GB de couches)
- Embedder + reranker depuis HuggingFace (~400 MB)

**Total premier boot : ~2.5 GB.**

### Runtime

- Chaque tour de chat : ~5 KB sortant vers Groq (ou Mistral).
- Aucun trafic si le cache de réponses répond.

## 6. Disque dur — SSD vs HDD

**SSD recommandé** pour PostgreSQL (les recherches pgvector sont I/O
sensibles). SATA SSD suffit largement.

## 7. Latence attendue selon la configuration

Pour une question moyenne (RAG + génération de ~500 tokens) :

| Configuration | Latence par tour |
|---|---|
| Cache hit (réponse déjà vue) | **< 50 ms** |
| Raccourci + Groq (formatage) | ~500 ms |
| Raccourci + Mistral (formatage) | ~1-2 s |
| RAG complet + Groq | ~1-2 s |
| RAG complet + Mistral | ~2-4 s |

## 8. Surveiller la consommation en production

```sh
# RAM par container
docker stats --no-stream

# Logs ai-service (voir les fallbacks Groq→Mistral)
docker logs -f backend-ai-service-1

# État Postgres
docker exec backend-db-1 psql -U user -d stocks -c "\dt+ "

# Taille des volumes
docker system df -v
```

Indicateurs d'alerte :

- ai-service > 2 GB → probable fuite ou reranker mal libéré
- Postgres > 1 GB → vérifier `chat_messages` (purge si on dépasse 100 k lignes)
- Rate-limit Groq atteint fréquemment → configurer `MISTRAL_API_KEY` pour
  l'auto-fallback

## 9. Quotas suggérés par utilisateur

Si tu ouvres le chatbot à plusieurs utilisateurs et que tu veux limiter
la consommation des quotas Groq/Mistral :

| Profil | Suggéré |
|---|---|
| Démo / interne (5-10 users) | Pas de quota |
| Production légère (~50 users actifs/jour) | 50 messages/user/jour |
| Production forte (> 100 users) | 30 messages/user/jour + rate-limit 1 req/15s |

Implémentation : la vue `v_chat_usage_daily` aggrège tout ce qu'il faut.
Reste à brancher un middleware qui vérifie avant d'accepter un nouveau
`POST /chat/sessions/.../messages`.
