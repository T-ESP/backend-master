# Changer de modèle LLM local

Le code est conçu pour que swapper le modèle local soit **une simple
modification d'environnement** sans aucun changement de Python.

## Quand passer à un modèle plus gros

Si le panel d'évaluation montre des échecs persistants liés au **raisonnement
multi-étapes** que les shortcuts ne couvrent pas (par ex : « prix du
fournisseur du produit le plus vendu », question composite à 3 niveaux),
un modèle plus puissant est la solution.

## Options par taille (Q4_K_M sauf indication)

| Modèle | RAM résidente | URL HuggingFace | Remarques |
|---|---|---|---|
| **Qwen2.5-1.5B-Instruct** | 1.8 GB | [Qwen2.5-1.5B GGUF](https://huggingface.co/Qwen/Qwen2.5-1.5B-Instruct-GGUF/resolve/main/qwen2.5-1.5b-instruct-q4_k_m.gguf) | Léger, tool-call faible |
| **Qwen2.5-3B-Instruct** ← actuel | 2.5 GB | [Qwen2.5-3B GGUF](https://huggingface.co/Qwen/Qwen2.5-3B-Instruct-GGUF/resolve/main/qwen2.5-3b-instruct-q4_k_m.gguf) | Bon compromis |
| **Phi-3.5-mini-Instruct** (3.8B) | 2.8 GB | [Phi-3.5-mini GGUF](https://huggingface.co/microsoft/Phi-3.5-mini-instruct-GGUF/resolve/main/Phi-3.5-mini-instruct-Q4_K_M.gguf) | Reconnu fort en tool-calling structuré |
| **Qwen2.5-7B-Instruct Q3_K_M** | 3.8 GB | [Qwen2.5-7B Q3](https://huggingface.co/Qwen/Qwen2.5-7B-Instruct-GGUF/resolve/main/qwen2.5-7b-instruct-q3_k_m.gguf) | 7B paramètres quantifiés agressivement |
| **Qwen2.5-7B-Instruct Q4_K_M** | 4.7 GB | [Qwen2.5-7B Q4](https://huggingface.co/Qwen/Qwen2.5-7B-Instruct-GGUF/resolve/main/qwen2.5-7b-instruct-q4_k_m.gguf) | Tight sur 5 GB, désactiver le reranker |
| **Llama-3.2-3B-Instruct** | 2.5 GB | [Llama-3.2-3B GGUF](https://huggingface.co/bartowski/Llama-3.2-3B-Instruct-GGUF/resolve/main/Llama-3.2-3B-Instruct-Q4_K_M.gguf) | Anglais fort, FR correct |

## Comment switcher

Dans [`docker-compose.yml`](../../docker-compose.yml), changer deux lignes :

```yaml
LOCAL_LLM_MODEL_PATH: "/app/llm_models/qwen2.5-7b-instruct-q4_k_m.gguf"
LOCAL_LLM_MODEL_URL: "https://huggingface.co/Qwen/Qwen2.5-7B-Instruct-GGUF/resolve/main/qwen2.5-7b-instruct-q4_k_m.gguf"
```

Puis :

```sh
docker compose up -d --force-recreate --no-deps ai-service
```

Au premier démarrage, le nouveau modèle se télécharge (~3-5 GB) dans le volume
`llm_models`. Les ~10-15 minutes de download arrivent une seule fois — le
fichier persiste ensuite.

## Budget RAM — vérifier avant de switcher

Plafond cible : **5 GB** sur un VPS 8 GB.

Composants actuellement chargés :

| Composant | Toujours en RAM | Notes |
|---|---|---|
| ai-service Flask + Python | ~250 MB | Permanent |
| Embedder multilingue MiniLM | ~150 MB | Permanent (chargé au 1er appel RAG) |
| Reranker BGE-v2-m3 | ~280 MB | Permanent (chargé au 1er appel RAG) |
| Modèle local LLM | variable | Chargé au 1er chat, peut être déchargé |

**Total fixe (sans LLM) :** ~680 MB
**Reste pour le LLM :** 5000 - 680 = **4.3 GB**

### Tableau de décision

| Modèle | RAM totale ai-service | Tient sous 5 GB ? |
|---|---|---|
| Qwen 1.5B Q4 | ~2.5 GB | ✓ confortable |
| Qwen 3B Q4 (actuel) | ~3.2 GB | ✓ confortable |
| Phi-3.5-mini Q4 | ~3.5 GB | ✓ confortable |
| Qwen 7B Q3 | ~4.5 GB | ✓ tight |
| Qwen 7B Q4 | ~5.4 GB | ✗ dépasse |
| Qwen 7B Q4 + reranker OFF | ~5.1 GB | ⚠ tout juste |

### Désactiver le reranker pour libérer 280 MB

Si nécessaire pour faire passer un 7B Q4 :

```yaml
RAG_USE_RERANKER: "false"
```

Effet sur la qualité du RAG : les chunks vectoriels sont utilisés tels
quels (sans réordonnement cross-encoder). Pour ~14 documents indexés, la
perte est marginale.

### Réduire le contexte LLM

Une autre façon de libérer ~500 MB-1 GB : passer de `LOCAL_LLM_CTX=8192`
à `LOCAL_LLM_CTX=4096`. Mais attention, le contexte est utilisé par :
- prompt système + few-shot tool examples : ~600 tokens
- historique de conversation : ~500-2000 tokens
- RAG context (4 chunks) : ~1500 tokens
- résultat tool (parfois lourd, ex: low_stock) : ~3000 tokens
- réponse : ~600 tokens

À 4k de contexte, on dépasse facilement. Préférer désactiver le reranker.

## Mesurer après switch

```sh
docker stats --no-stream backend-ai-service-1
```

Vérifier que `MEM USAGE` reste sous 5 GiB après quelques tours de chat.

Puis re-lancer l'eval pour mesurer le gain qualitatif :

```sh
python scripts/chat_eval_panel.py --provider local
```

Comparer le score et la liste des échecs avec la baseline 3B documentée dans
[`evaluation-2026-05-15.md`](evaluation-2026-05-15.md).

## Si on dispose d'un GPU

Tout change : on peut faire tourner un 14B-32B confortablement. llama.cpp
supporte CUDA / ROCm via le flag `n_gpu_layers`. Adapter
`LOCAL_LLM_THREADS=1` (CPU pas utilisé) et ajouter une variable
`LOCAL_LLM_N_GPU_LAYERS=999` dans le compose.

Code change requis : ajouter `n_gpu_layers=int(os.getenv("LOCAL_LLM_N_GPU_LAYERS", "0"))`
dans le `Llama(...)` de [`local_provider.py`](../../ai-service/chat/llm/local_provider.py).

## Modèles API (sans GPU local)

Si la latence locale reste trop lente pour ton usage (>30 s par tour),
configure Mistral ou Groq :

```env
MISTRAL_API_KEY=sk-...
GROQ_API_KEY=gsk-...
LLM_PROVIDER=auto
```

`auto` essaiera Mistral d'abord (gratuit, multilingue fort, ~1-3 s par
tour), Groq ensuite (free tier ultra rapide, ~0.5-1 s), puis local en
fallback. Aucun changement de code ; juste ajouter les clés dans `.env`.
