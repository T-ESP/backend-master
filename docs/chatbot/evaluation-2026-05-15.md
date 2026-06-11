# Évaluation systématique du chatbot — 2026-05-15

Résultats d'une session de tests automatisés contre la stack live (provider
local Qwen2.5-3B), pour mesurer fiabilité, justesse des réponses et latence
sur un panel représentatif de questions.

## Harness

Script : [`scripts/chat_eval.py`](../../scripts/chat_eval.py).

Principe :

1. Lit la **vérité** directement depuis Postgres (top produit par CA, top par
   volume, nb de produits en rupture, etc.).
2. Pose **11 questions** dont on connaît la bonne réponse.
3. Vérifie pour chacune :
   - `intent` correctement classifié ?
   - bon outil appelé (ou shortcut utilisé) ?
   - faits attendus dans la réponse ?
   - phrases interdites (ex : « je vais chercher » sans appel d'outil) ?
4. Imprime un tableau de scores + p95 latence + détails échecs.

## Avant / après

Deux campagnes successives, séparées par un round de fixes ciblés.

| | Baseline | Après fixes |
|---|---|---|
| Score global | **7/11 (64 %)** | **10/11 (91 %)**, plus 1 faux positif dans le harness |
| Latence moyenne | 74 s | 79 s |
| Latence p95 | 247 s | 210 s |

## Tableau détaillé (après fixes)

| # | Question | Intent | Outil | Latence | OK ? |
|---|---|---|---|---|---|
| 1 | « Bonjour ! » | chitchat | — | 15 s | ✓ |
| 2 | « Qu'est-ce que la classification ABC-XYZ ? » | doc | (cache) | 11 s | ✓ |
| 3 | « que veux dire ABC-XYZ » | doc | — | 112 s | ✓* |
| 4 | « Quel est le produit qui rapporte le plus d'argent ? » | data | `get_top_products` (revenue) | 26 s | ✓ |
| 5 | « Donne-moi les 3 produits qui rapportent le plus. » | data | `get_top_products` (revenue, limit=3) | 15 s | ✓ |
| 6 | « Quel est mon produit le plus vendu (en quantité) ? » | data | `get_top_products` (volume) | 21 s | ✓ |
| 7 | « Quels produits sont en stock bas ? » | data | `get_low_stock` | 197 s | ✓ |
| 8 | « Combien de produits sont en rupture critique ? » | data | `get_low_stock` | 158 s | ✓ |
| 9 | « Donne-moi les détails du produit 1 » | data | `get_product_detail` | 99 s | ✓ |
| 10 | « Donne-moi un résumé de mon activité. » | data | `get_global_kpis` | 210 s | ✓ |
| 11 | (rejoue Q2) | doc | (cache) | **0.1 s** | ✓ |

\* Test 3 affiché en ✗ dans la sortie originale — faux positif côté harness :
le bot a parfaitement répondu mais le check substring cherchait `("revenu",
"variabilité", "pareto")` alors que la réponse contenait `"chiffre d'affaires"`
et `"demande"`. Élargi dans le commit suivant.

## Bugs trouvés et corrigés

### 1. Intent classifier rate « que veux dire »

**Symptôme** : « que veux dire ABC-XYZ » classé en **`data`** au lieu de
`doc`, ce qui mène à un appel d'outil au lieu de RAG. Le bot dit *« Je vais
chercher »* sans rien faire derrière.

**Cause** : ma regex `\bque (signifie|veut dire)\b` matche seulement
`veut dire` (forme avec un 't' singulier), pas `veux dire` (forme orale
courante / faute).

**Fix** : ajout de `veux dire` à l'alternation + nouvelles règles
`ça veut dire quoi`, `c'est quoi`, `définition`, `kpi`.

### 2. Shortcuts trop étroits

**Symptôme** : pour « produit le plus vendu » / « qui rapporte le plus », aucun
shortcut ne matche → le LLM doit décider, le LLM dit *« Je vais utiliser
`get_top_products` »* sans appeler le tool. Ou pire, **hallucination** —
le LLM invente "Produit 123 : CA de 123 000 €".

**Cause** : mes shortcuts ne couvraient que `top N produits par X`. Pas les
formulations naturelles.

**Fix** : 6 nouvelles regex shortcuts :
- `(produit|article).*(rapporte|génère).*(le plus)` → revenue
- `(\d+).*produits.*qui rapportent` → revenue, limit capturé
- `(produit|article).*(le|les) plus vendus?` → volume
- `(produit|article).*(le|les) plus rentables?` → profit
- `(produit|article).*(le|les) moins vendus?` → flop_sales (NOUVEAU)
- `(produit|article).*se vendent? (mal|moins|pas)` → flop_sales

### 3. Hallucination "40 120 € = prix"

**Symptôme** : le bot dit à l'utilisateur que `40 120` est le prix de vente
du produit, alors que c'est en réalité le **chiffre d'affaires cumulé sur 30
jours**.

**Cause** : `get_top_products` renvoie `{"value": 40120}` sans unité ni
contexte. Le LLM, livré à lui-même, devine — et se trompe.

**Fix** : la réponse de `get_top_products` contient désormais un champ `unit`
explicite :
- revenue → « chiffre d'affaires cumulé en euros (somme de toutes les ventes
  sur la période) »
- profit → « profit cumulé en euros sur la période »
- volume → « quantité totale vendue (unités, pas euros) »
- etc.

### 4. Métrique « moins vendu » manquante

**Symptôme** : « quel est le produit le moins vendu ? » n'avait aucun moyen
d'être répondu correctement — la métrique de classement ascendant n'existait
pas dans `get_top_products`.

**Fix** : ajout des métriques `flop_sales` et `flop_profit` qui mappent vers
`flop_10_by_sales` et `flop_10_by_profit` de l'API `/kpis/top-flop`.

### 5. Recherche d'un produit par son nom impossible

**Symptôme** : « quel est le prix unitaire de Terreau universel 20L » → bot dit
*« Je vais rechercher »* puis rien. Aucun outil ne savait partir d'un nom — tous
exigeaient un `product_id`.

**Cause** : tous les tools `get_product_detail`, `get_classification`,
`get_forecast` exigent un identifiant numérique. Quand l'utilisateur cite un
produit par son nom, le LLM doit théoriquement enchaîner deux appels (chercher
puis détailler), mais le Qwen2.5-3B local n'est pas fiable pour ce genre
d'enchaînement.

**Fix** :

- Nouveau tool **`find_product_by_name(query)`** qui wrappe `/products/light`
- Nouveau tool **`get_product_by_name(name)`** qui fait le 2-step en interne
  (search → detail + KPIs) — déterministe, pas besoin que le LLM enchaîne
- 2 nouveaux shortcuts regex avec **groupes nommés** `(?P<name>...)` pour
  extraire le nom propre quand l'utilisateur dit « prix/stock/marge **de** X »
  ou « combien coûte X »
- `_clean_name()` strippe les articles français en tête (« le », « la »,
  « les », « l' », etc.)

**Vérifié en live** :

| Question | Shortcut | Réponse |
|---|---|---|
| « quel est le prix unitaire de Terreau universel 20L » | `get_product_by_name` | « **42.72 €** » ✓ |
| « combien coute Chargeur Rapide 20W » | `get_product_by_name` | « **182.41 € d'achat**, **380.16 € vente**, **158.31 € profit/vente** » ✓ |
| « stock de Sirop de grenadine 75cl » | `get_product_by_name` | « **3 unités** » ✓ (avec petite gaffe d'unité « litres » influencée par « 75cl » dans le nom) |

### Note sur « 40 120 unités » qui semblait halluciné

Faux positif. Le bot disait *« Terreau universel 20L, 40 120 unités vendues »*
alors que mon eval comptait 1 343 unités sur 30 jours. L'API `/kpis/top-flop`
calcule en réalité un volume cumulatif (sur toute la période disponible des
commandes seed), donc 40 120 est le bon chiffre côté API. C'est mon eval
qui comparait à une mauvaise référence — pas une hallucination du bot.

## Observations sur la performance

### Distribution des latences

- **Cache hit** (test 2, 11) : ~0.1-0.2 s — instantané.
- **Chitchat / doc avec cache** : ~10-15 s sur local CPU.
- **Shortcut → tool call → formatage LLM** (tests 4, 5, 6, 9) : 15-100 s
  selon taille du payload de l'outil.
- **Shortcut → gros tool result** (tests 7, 8, 10 : low-stock 76 produits,
  KPIs globaux) : 150-210 s. Le coût n'est pas l'outil mais la mise en
  forme par le LLM sur 4 cores CPU.

### Distribution des shortcuts

Sur les 11 tests, **5 shortcuts ont matché** (45 %), sauvant 5 tours
décide-puis-appelle qui auraient ajouté ~30 s chacun et risqué l'hallucination.
**0 tour de tool-loop LLM n'a été nécessaire** — toutes les questions data
ont été routées soit par shortcut soit par cache.

Les 4 tests qui sont les plus lents (7, 8, 9, 10) sont ceux dont le tool
result est volumineux. Pistes pour les accélérer :

1. **Top-K dans la requête** : limiter le résultat tool à ~10 entrées max
   pour les requêtes générales.
2. **Streaming SSE** : déjà à la roadmap. Le perceived latency descendrait
   énormément même si le wall-clock reste identique.
3. **API LLM** (Mistral / Groq) au lieu de local : latence p95 chuterait
   à ~5 s sur les questions volumineuses.

### Cache de réponses — effet massif

Test 11 démontre l'effet du cache pour les questions doc/concept répétées :
**0.1 s** contre 60-110 s sans cache. Le cache est fuzzy (cosine distance
< 0.05 sur l'embedding de la question), donc des formulations équivalentes
hittent aussi.

## Comment relancer

```sh
# Stack doit être up (docker compose up -d)
python scripts/chat_eval.py --provider local         # ~12-15 min
python scripts/chat_eval.py --provider auto          # ~3-5 min si clés API
python scripts/chat_eval.py --provider local --verbose   # voir les réponses
```

Le harness imprime un tableau récapitulatif à la fin + les détails de chaque
échec.

## Pistes pour la suite

- **Ajouter plus de cas de test** : actuellement 11 questions. Cibler ~30
  pour bien couvrir tous les outils et phrasings courants.
- **Comparer providers** : lancer l'eval avec `--provider mistral|groq|local`
  côte à côte pour quantifier le compromis qualité/latence.
- **Tests de non-régression dans CI** : faire tourner l'eval automatiquement
  sur les MR, bloquer le merge si score < 80 %.
- **Détection d'hallucination** : ajouter une vérif automatique « si
  `cached=false` et `shortcut_used` et `tool_calls=∅`, c'est suspect » —
  on n'a pas réussi à appeler l'outil malgré l'intent data.
