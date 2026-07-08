# Modèles LLM implémentés

Le chatbot supporte **deux fournisseurs LLM** (API-only), sélectionnables par
variable d'environnement ou par requête (`provider` dans le body JSON).
L'interface `LLMProvider` les rend interchangeables — changer de fournisseur
ne demande aucun changement de code applicatif.

> **Note (2026-07)** — Le provider local (Qwen2.5 via llama.cpp) a été retiré.
> Sur nos VPS 8 GB, la latence CPU (5–20 s/tour), la fiabilité tool-calling
> médiocre et le poids du binaire llama-cpp-python ne justifiaient pas la
> complexité. Groq gratuit couvre tous nos volumes et Mistral sert de fallback.

## 1. Vue d'ensemble

| Fournisseur | Type | Modèle par défaut | Plan | Tool-calling | Latence typique |
|---|---|---|---|---|---|
| **Groq** (primaire) | API REST | `llama-3.3-70b-versatile` | Gratuit (~30 req/min, 1M tok/jour) | Natif OpenAI | 0.3–1 s |
| **Mistral** (fallback) | API REST | `mistral-small-latest` | Gratuit (~1 req/s, 500k tok/min) | Natif | 1–3 s |

## 2. Groq — API primaire

### Comment obtenir une clé

1. Aller sur https://console.groq.com
2. Créer un compte
3. « API Keys » → « Create API Key »
4. Coller dans `.env` : `GROQ_API_KEY=...`

### Modèle utilisé

Par défaut `llama-3.3-70b-versatile`. Le matériel de Groq (LPU) permet une
inférence **extrêmement rapide** — souvent < 1 seconde pour des réponses
longues, même sur un modèle 70B.

Pour utiliser un modèle plus petit / plus rapide :

```env
GROQ_MODEL=llama-3.1-8b-instant
```

### Limites du plan gratuit (2026)

- ~30 requêtes par minute (assez restrictif)
- ~1 M tokens par jour sur Llama-3.3-70B
- En cas de saturation, Groq renvoie un 429 — notre code fait fallback
  automatique sur Mistral si `MISTRAL_API_KEY` est défini.

## 3. Mistral — fallback

### Comment obtenir une clé

1. Aller sur https://console.mistral.ai
2. Créer un compte (gratuit, pas de carte bancaire)
3. Onglet « API keys » → « Create new key »
4. Coller dans `.env` : `MISTRAL_API_KEY=...`

### Modèle utilisé

Par défaut `mistral-small-latest`. Le tool-calling natif fonctionne
correctement. Utilisé uniquement quand Groq n'est pas dispo (429, panne
réseau) — donc rarement.

Pour utiliser un autre modèle (`ministral-3b-latest` est encore plus rapide) :

```env
MISTRAL_MODEL=ministral-3b-latest
```

### Limites du plan gratuit (2026)

- ~1 requête par seconde
- 500 000 tokens par minute
- 1 milliard de tokens par mois

## 4. Stratégie d'auto-fallback

Quand `LLM_PROVIDER=auto` (ou `groq`, le défaut), la chaîne est :

```
Groq  ──429 ou erreur──▶  Mistral
```

Côté code, c'est `chat_with_fallback()` dans `chat/llm/factory.py`. Chaque
fournisseur est testé pour disponibilité avant l'appel (`is_available()` —
vérifie clé API présente) ; les non-disponibles sont sautés silencieusement.

Recommandé en prod :

- **Défaut** : `LLM_PROVIDER=groq` avec `MISTRAL_API_KEY` défini pour le
  fallback. C'est la config la plus rapide et la plus robuste.
- **Économiser Mistral** : `LLM_PROVIDER=groq` sans `MISTRAL_API_KEY` — le
  service retournera une erreur si Groq est saturé au lieu de basculer.

## 5. Tool-calling

Les deux API supportent le **function calling natif** (format OpenAI), donc
l'appel d'outil est fiable à ~99 %. On envoie les schémas générés depuis
`chat/tools/registry.py`.

En complément, pour les 10–15 questions les plus courantes, on **n'envoie
même pas la décision au LLM** : on appelle directement l'outil basé sur une
regex, et le LLM ne sert qu'à formater le résultat en prose. Voir
`chat/agent/shortcuts.py`.

## 6. Embedder pour le RAG (séparé du LLM principal)

Le chatbot utilise aussi un **modèle d'embeddings** pour le RAG :

| | Détails |
|---|---|
| Modèle | `sentence-transformers/paraphrase-multilingual-MiniLM-L12-v2` |
| Dimensions | 384 |
| Langues | 50+ langues, FR + EN inclus |
| RAM | ~150 MB |
| Taille disque | ~120 MB (cache HuggingFace) |
| Vitesse | ~50 chunks/s sur CPU |

## 7. Reranker pour le RAG (optionnel mais activé)

Pour améliorer la pertinence des passages RAG :

| | Détails |
|---|---|
| Modèle | `BAAI/bge-reranker-v2-m3` |
| Type | Cross-encoder |
| Langues | Multilingue (FR + EN très bons) |
| RAM | ~280 MB |
| Latence | ~200 ms pour reranker 20 chunks |

Désactivable via `RAG_USE_RERANKER=false` si on veut récupérer la RAM.

## 8. Comparatif rapide pour choisir

| Besoin | Choix |
|---|---|
| Défaut recommandé | `LLM_PROVIDER=groq` + `MISTRAL_API_KEY` en fallback |
| Vitesse max | `groq` (Llama 3.3 70B en < 1 s) |
| Qualité française native | `mistral` |
| Économie de rate-limit Groq | `mistral` en primaire |

## 9. Ajouter un nouveau provider

L'architecture est conçue pour l'extension. Pour ajouter Claude, OpenAI, ou
un autre :

1. Créer `ai-service/chat/llm/<nom>_provider.py` qui étend `LLMProvider` :
   implémenter `is_available()` et `chat(messages, tools)`.
2. L'enregistrer dans `chat/llm/factory.py::_instance()` et l'ajouter à
   `_AUTO_CHAIN` si on veut qu'il participe au fallback.
3. C'est tout — l'orchestrateur l'utilisera automatiquement.

Compter ~120 lignes de code par provider sur le modèle de
`groq_provider.py`.
