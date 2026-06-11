# Améliorations chatbot — 2026-05-21

Round d'optimisations « intelligence + efficacité » sous contrainte RAM
(VPS 8 GB, plafond 5 GB).

## Résumé

| Amélioration | Statut | Gain | Coût RAM |
|---|---|---|---|
| Prompt caching | ✅ gardé | -5 à -15 s / tour | ~256 MB (cache tunable) |
| KV cache quantifié | ❌ abandonné | — | bloquait le CPU |
| Pré-résumé des gros payloads | ✅ gardé | vitesse + pas d'overflow | 0 |
| Shortcuts sémantiques | ✅ gardé | capture les paraphrases | 0 (embedder déjà chargé) |
| Mémoire d'entités | ✅ gardé | questions de suivi | 0 |
| Dockerfile torch CPU-only | ✅ gardé | image -2 GB, builds rapides | — |

## 1. Prompt caching (gardé)

`LlamaRAMCache` réutilise l'état KV du préfixe commun à chaque tour (prompt
système + few-shot d'exemples de tool-calls = ~800 tokens identiques). Sans
cache, ce préfixe est re-traité à chaque requête ; avec, llama.cpp restaure
l'état directement.

- Activé par défaut. Désactivable : `LOCAL_LLM_PROMPT_CACHE=false`.
- Taille du cache : `LOCAL_LLM_CACHE_MB` (défaut 256).
- Gain attendu : 5-15 s par tour sur CPU 4 cœurs.

## 2. KV cache quantifié — ABANDONNÉ

**Idée initiale** : quantifier le cache KV en Q8_0 (`type_k`/`type_v`) pour
diviser par ~2 sa consommation RAM.

**Pourquoi abandonné** : la quantification du KV cache exige flash attention
activé. Or flash attention sur **CPU** (VPS sans GPU) fait spinner certaines
builds de llama.cpp — observé en live : **300 % CPU pendant 6 minutes par
requête, sans produire de réponse**.

**Décision** : conformément au principe « si c'est trop lourd, on laisse
tomber », le code est retiré. Le KV cache reste en f16 (le défaut sûr et
fonctionnel). Le bénéfice RAM annoncé (~500 MB-1 GB) est abandonné — sans
conséquence, on était déjà sous le plafond 5 GB.

## 3. Pré-résumé des gros payloads (gardé)

Quand un outil renvoie une grosse liste (ex : 76 produits en stock bas),
`_compress()` la tronque récursivement à 12 éléments et ajoute un marqueur
`_tronque` qui indique le total réel.

Effet : le LLM reçoit un payload borné → formatage plus rapide, pas de
dépassement du contexte de 8k tokens. Le bot mentionne quand même le total
exact (« 76 produits au total ») grâce au marqueur.

- Réglable : `CHAT_TOOL_MAX_LIST_ITEMS` (défaut 12).

## 4. Shortcuts sémantiques (gardé)

Les shortcuts regex de `shortcuts.py` sont rapides mais fragiles : il faut
anticiper chaque formulation. Les **shortcuts sémantiques** prennent
l'approche inverse.

`semantic_shortcuts.py` définit ~38 questions canoniques, chacune mappée à
un (tool, args). Au démarrage, elles sont embeddées avec le modèle déjà
chargé pour le RAG → **coût RAM nul**. À l'exécution : on embed la question
de l'utilisateur, on cherche la canonique la plus proche (cosine), et si la
similarité dépasse 0.62 on utilise le shortcut associé.

Pipeline complet :
```
question
  → shortcut regex   (exact, 0 ms)
  → shortcut sémantique  (robuste aux paraphrases)
  → routage LLM classique
```

- Seuil : `CHAT_SEMANTIC_SHORTCUT_THRESHOLD` (défaut 0.62).
- Effet : « montre-moi ce qui se vend le mieux » est compris même sans
  regex dédiée, parce que sémantiquement proche de la canonique « quels
  sont les meilleurs produits ».

## 5. Mémoire d'entités (gardé)

`entity_memory.py` permet les questions de suivi naturelles :

```
User : quel est le prix de Terreau universel 20L ?
Bot  : 44.08 €
User : et son stock ?            ← « son » = Terreau universel 20L
```

Mécanisme : on scanne l'historique récent pour le dernier produit /
fournisseur cité. Si la question courante contient une référence
anaphorique (« son », « le », « ce produit ») **sans** nommer d'entité
explicite, on injecte une note de contexte dans le prompt système qui
indique au LLM de quoi on parle.

100 % Python, aucun coût RAM, aucun appel réseau.

## 6. Dockerfile — torch CPU-only

`sentence-transformers` dépend de `torch`. Par défaut, `pip install torch`
tire la variante CUDA — ~2 GB de bibliothèques NVIDIA **inutiles sur un VPS
sans GPU**.

Le Dockerfile installe désormais torch CPU-only en premier
(`--index-url https://download.pytorch.org/whl/cpu`), avant le reste des
dépendances. pip considère ensuite torch comme satisfait et ne retire pas
la version CUDA.

Effet : image Docker allégée d'environ 2 GB, builds nettement plus rapides.

## Budget RAM après ce round

| Composant | RAM |
|---|---|
| ai-service Flask + Python | ~250 MB |
| Embedder MiniLM | ~150 MB |
| Reranker BGE | ~280 MB |
| LLM local Qwen2.5-3B (f16 KV cache) | ~2.5 GB |
| Prompt cache | ~256 MB |
| **Total chat** | **~3.4 GB** |

Toujours sous le plafond de 5 GB. Les shortcuts sémantiques et la mémoire
d'entités n'ajoutent rien (réutilisent l'embedder).

## Variables d'environnement nouvelles

| Variable | Défaut | Effet |
|---|---|---|
| `LOCAL_LLM_PROMPT_CACHE` | `true` | Active le prompt caching |
| `LOCAL_LLM_CACHE_MB` | `256` | Taille du cache prompt en MB |
| `CHAT_TOOL_MAX_LIST_ITEMS` | `12` | Troncature des listes dans les payloads |
| `CHAT_SEMANTIC_SHORTCUT_THRESHOLD` | `0.62` | Seuil de similarité shortcut sémantique |

## Tests

7 nouveaux tests unitaires (mémoire d'entités + compression). Total 39/39.
