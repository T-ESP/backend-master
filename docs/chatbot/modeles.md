# Modèles LLM implémentés

Le chatbot supporte **trois familles de fournisseurs LLM**, sélectionnables par
variable d'environnement ou par requête (`provider` dans le body JSON).
L'interface `LLMProvider` les rend interchangeables — changer de fournisseur
ne demande aucun changement de code applicatif.

## 1. Vue d'ensemble

| Fournisseur | Type | Modèles | Plan | Tool-calling | Latence typique |
|---|---|---|---|---|---|
| **Mistral** | API REST | `mistral-small-latest`, `ministral-3b-latest`, `open-mistral-7b` | Gratuit (~1 req/s, 500k tok/min) | Natif | 1–3 s |
| **Groq** | API REST | `llama-3.3-70b-versatile`, `llama-3.1-8b-instant`, `mixtral-8x7b-32768` | Gratuit (~30 req/min, 6k tok/min) | Natif | 0.3–1 s |
| **Local** | llama-cpp-python en process | Qwen2.5-3B (défaut), Qwen2.5-1.5B, Phi-3.5-mini, SmolLM2-1.7B, Llama-3.2-3B | Illimité | Via grammaire GBNF | 5–20 s (CPU) |

## 2. Modèle local par défaut — Qwen2.5-3B-Instruct

C'est le compromis recommandé pour un VPS 8 GB :

| Caractéristique | Valeur |
|---|---|
| Taille sur disque (Q4_K_M) | ~1.9 GB |
| RAM occupée chargé | ~2.5 GB |
| Tokens/sec (4 cœurs CPU) | ~5–8 |
| Contexte natif | 32 k tokens (on utilise 8 k pour économiser la RAM) |
| Langues | Multilingue, **français très bon** |
| Tool-calling | Acceptable (1.5B était insuffisant) |
| URL HuggingFace | https://huggingface.co/Qwen/Qwen2.5-3B-Instruct-GGUF |

### Téléchargement automatique

Au premier démarrage, ai-service détecte que le fichier `.gguf` n'existe pas
dans le volume `llm_models` et le télécharge automatiquement depuis
HuggingFace. C'est une opération **one-shot d'environ 2 GB** ; ensuite le
modèle reste en cache dans le volume Docker.

Pour désactiver le download (préchargement manuel) :

```env
LOCAL_LLM_AUTO_DOWNLOAD=false
```

## 3. Modèles locaux alternatifs (swap d'une ligne)

Tous testés et compatibles avec notre interface. Pour changer, modifier
`LOCAL_LLM_MODEL_PATH` et `LOCAL_LLM_MODEL_URL` dans `docker-compose.yml`
ou `.env`.

### Qwen2.5-1.5B-Instruct (le plus léger)

- RAM : ~1.8 GB
- Vitesse : ~10-15 tok/s
- Tool-calling : faible, mais OK avec GBNF + raccourcis déterministes
- Quand l'utiliser : VPS très contraint (< 4 GB libres), ou pour la chitchat
  pure
- URL : `https://huggingface.co/Qwen/Qwen2.5-1.5B-Instruct-GGUF/resolve/main/qwen2.5-1.5b-instruct-q4_k_m.gguf`

### Phi-3.5-mini-instruct (3.8B)

- RAM : ~2.8 GB
- Vitesse : ~5-7 tok/s
- Tool-calling : meilleur raisonnement structuré
- Quand l'utiliser : si l'anglais est OK et qu'on veut une qualité de
  réponse supérieure
- URL : `https://huggingface.co/microsoft/Phi-3.5-mini-instruct-GGUF/resolve/main/Phi-3.5-mini-instruct-Q4_K_M.gguf`

### SmolLM2-1.7B-Instruct

- RAM : ~1.8 GB
- Vitesse : ~12 tok/s
- Tool-calling : très moyen
- Quand l'utiliser : alternative récente au Qwen 1.5B, parfois meilleure
  sur le suivi d'instructions courtes
- URL : `https://huggingface.co/HuggingFaceTB/SmolLM2-1.7B-Instruct-GGUF/resolve/main/smollm2-1.7b-instruct-q4_k_m.gguf`

### Llama-3.2-3B-Instruct

- RAM : ~2.5 GB
- Vitesse : ~6-9 tok/s
- Tool-calling : moyen
- Quand l'utiliser : si on préfère l'écosystème Meta, généraliste bien
  documenté
- URL : `https://huggingface.co/bartowski/Llama-3.2-3B-Instruct-GGUF/resolve/main/Llama-3.2-3B-Instruct-Q4_K_M.gguf`

## 4. Quantification — pourquoi Q4_K_M

Tous les modèles ci-dessus sont en **Q4_K_M** (4-bit, K-quants, M-variant) :

- **Q4 (4-bit)** : la précision la plus basse qui préserve >99 % de la
  qualité d'un modèle d'instruct sur les tâches typiques.
- **K-quants** : schéma de quantification avancé qui groupe les poids par
  blocs pour minimiser la dégradation.
- **M-variant** : équilibre vitesse/qualité (vs S = small/rapide, L =
  large/qualité).

À taille de modèle égale, Q4_K_M = **2x plus petit en disque/RAM** que Q8
(8-bit) ou FP16, avec une perte de qualité négligeable.

Pour qualité maximale, Q6_K ou Q8_0 sont aussi disponibles (chercher dans
le même repo HuggingFace) — ça multiplie la RAM par ~1.5x.

## 5. Mistral — API gratuite

### Comment obtenir une clé

1. Aller sur https://console.mistral.ai
2. Créer un compte (gratuit, pas de carte bancaire)
3. Onglet « API keys » → « Create new key »
4. Coller dans `.env` : `MISTRAL_API_KEY=...`

### Modèle utilisé

Par défaut `mistral-small-latest` (équivalent à un ~7B en qualité, mais
exécuté côté Mistral en latence ~1-2 s). Le tool-calling natif fonctionne
parfaitement.

Pour utiliser un autre modèle (`ministral-3b-latest` est encore plus rapide) :

```env
MISTRAL_MODEL=ministral-3b-latest
```

### Limites du plan gratuit (2026)

- ~1 requête par seconde
- 500 000 tokens par minute
- 1 milliard de tokens par mois (largement suffisant pour un usage interne)

## 6. Groq — API gratuite ultra-rapide

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
- ~6 000 tokens par minute
- En cas de saturation, Groq renvoie un 429 — notre code fait fallback
  automatique sur Mistral puis local.

## 7. Stratégie d'auto-fallback

Quand `LLM_PROVIDER=auto` (le défaut), la chaîne est :

```
Mistral  ──429 ou erreur──▶  Groq  ──429 ou erreur──▶  Local
```

Côté code, c'est `chat_with_fallback()` dans `chat/llm/factory.py`. Chaque
fournisseur est testé pour disponibilité avant l'appel (`is_available()` —
vérifie clé API présente / fichier modèle existant) ; les non-disponibles
sont sautés silencieusement.

Recommandé en prod :

- Si tu veux **gratuit + rapide + privacy-friendly** : `auto`, et avoir les
  trois fournisseurs configurés. Mistral prend la majorité des requêtes,
  Groq sert quand on dépasse 1 req/s, local sert en offline.
- Si tu veux **100 % privé / offline** : `LLM_PROVIDER=local`. Aucune
  donnée ne sort du VPS.
- Si tu veux **vitesse max** : `LLM_PROVIDER=groq`. Les réponses arrivent
  en < 1 s.

## 8. Tool-calling — la spécificité petits modèles

Les API Mistral et Groq supportent le **function calling natif** (format
OpenAI), donc l'appel d'outil est fiable à ~99 %.

Les petits modèles locaux (1.5B – 3B) ont une fiabilité naturelle plus
basse (~70–85 %). Deux mécanismes compensent :

### A) Grammaire GBNF (llama.cpp)

On contraint le **décodeur** à respecter une grammaire formelle (BNF) qui
définit exactement ce qui est acceptable. Le modèle ne peut **physiquement
pas** émettre de JSON invalide. La grammaire est dans
`ai-service/chat/llm/grammar.py`.

Activée par défaut (`LOCAL_LLM_USE_GRAMMAR=true`).

### B) Few-shot dans le prompt système

Le prompt envoyé au modèle local contient **5 exemples concrets** de
question → appel d'outil correct. Les petits modèles imitent bien quand
ils voient des modèles à reproduire.

Voir `chat/llm/local_provider.py` constante `TOOL_INSTRUCTION_TEMPLATE`.

### C) Raccourcis déterministes (le plus efficace)

Pour les 10–15 questions les plus courantes, on **n'envoie même pas la
décision au LLM** : on appelle directement l'outil basé sur une regex, et
le LLM ne sert qu'à formater le résultat en prose. Voir
`chat/agent/shortcuts.py`.

Combinaison : tool-calling utilisable même sur Qwen2.5-1.5B.

## 9. Embedder pour le RAG (séparé du LLM principal)

Le chatbot utilise aussi un **modèle d'embeddings** pour le RAG :

| | Détails |
|---|---|
| Modèle | `sentence-transformers/paraphrase-multilingual-MiniLM-L12-v2` |
| Dimensions | 384 |
| Langues | 50+ langues, FR + EN inclus |
| RAM | ~150 MB |
| Taille disque | ~120 MB (cache HuggingFace) |
| Vitesse | ~50 chunks/s sur CPU |

## 10. Reranker pour le RAG (optionnel mais activé)

Pour améliorer la pertinence des passages RAG :

| | Détails |
|---|---|
| Modèle | `BAAI/bge-reranker-v2-m3` |
| Type | Cross-encoder |
| Langues | Multilingue (FR + EN très bons) |
| RAM | ~280 MB |
| Latence | ~200 ms pour reranker 20 chunks |

Désactivable via `RAG_USE_RERANKER=false` si on veut récupérer la RAM.

## 11. Comparatif rapide pour choisir

| Besoin | Choix |
|---|---|
| Démarrer rapidement, voir si ça marche | `auto` (utilisera local par défaut si pas de clé) |
| Production, demande modérée, gratuit | `auto` avec Mistral + Groq + local configurés |
| 100 % offline / privé | `local` avec Qwen2.5-3B |
| Vitesse max | `groq` (Llama 3.3 70B en < 1 s) |
| Qualité max | `mistral` (Mistral Small ou Large) |
| VPS très contraint (< 4 GB libres) | `local` avec Qwen2.5-1.5B + raccourcis |

## 12. Ajouter un nouveau provider

L'architecture est conçue pour l'extension. Pour ajouter Claude, OpenAI, ou
un autre :

1. Créer `ai-service/chat/llm/<nom>_provider.py` qui étend `LLMProvider` :
   implémenter `is_available()` et `chat(messages, tools)`.
2. L'enregistrer dans `chat/llm/factory.py::_instance()` (un seul `elif`).
3. C'est tout — l'orchestrateur l'utilisera automatiquement.

Compter ~120 lignes de code par provider sur le modèle de
`mistral_provider.py`.
