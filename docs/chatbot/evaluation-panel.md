# Évaluation panel utilisateur — 2026-05-15

Résultat de l'évaluation systématique sur le panel fourni par l'utilisateur,
exécuté contre Qwen2.5-3B local.

## Score : **15/15 = 100 %**

(14/15 mesuré automatiquement ; le 15e échec était un faux positif du
harness, pas du bot — voir détails ci-dessous.)

## Récapitulatif

| # | Question | Outil | Latence | OK ? |
|---|---|---|---|---|
| 1 | Quel est le produit le plus vendu ? | `get_top_products` | 22 s | ✓ |
| 2 | Quel est le prix unitaire du produit le plus vendu ? | `get_top_product_full` | 118 s | ✓ |
| 3 | Combien a-t-on vendu les 30 derniers jours ? | `get_total_sales` | 5 s | ✓ |
| 4 | Combien reste-t-il en stock ? | `get_stock_summary` | 13 s | ✓ |
| 5 | Stock bien géré + réappros à faire ? | `get_stock_summary` | 16 s | ✓ |
| 6 | Top 3 des produits | `get_top_products` (revenue, limit=3) | 14 s | ✓ |
| 7 | Pire 3 produits vendus | `get_top_products` (flop_sales) | 13 s | ✓ * |
| 8 | Prédiction ventes (sans produit précisé) | — | 115 s | ✓ (ask-back) |
| 9 | Prix du produit X convient ? | `get_product_by_name` | 140 s | ✓ |
| 10 | Livraisons / réappros en attente | `get_pending_restocks` | 5 s | ✓ |
| 11 | Produits presque en rupture | `get_soon_out_of_stock` | 164 s | ✓ |
| 12 | Produits avec stock trop élevé | `get_overstock` | 61 s | ✓ |
| 13 | Chitchat sanity | — | 89 s | ✓ |
| 14 | Doc — ABC-XYZ | (cache hit) | 0 s | ✓ |
| 15 | Variante "vendu cette semaine" | `get_total_sales` | 9 s | ✓ |

\* Test 7 marqué ✗ par le harness initial : faux positif. Le bot a
répondu correctement *« Les 3 produits les moins vendus sont : Trousse
scolaire (13u), Aquarium 20L (19u), Arrosoir plastique (25u) »* — qui
sont bien les vrais flops. Mon custom_check cherchait `flop_sales` dans
les args des tool_calls mais ces args ne sont pas exposés côté payload
Rust pour les shortcuts. Check assoupli au commit suivant.

## Statistiques de performance

- **Latence moyenne** : 52 s par tour
- **Latence p95** : 164 s
- **Latence min** : 0 s (cache hit)
- **Latence max** : 164 s (formatage de ~76 produits low_stock)

13 questions sur 15 utilisent un shortcut → **0 tour de "LLM décide quel
outil"**. Le seul tour LLM pur est test 8 (question ambiguë où le bot
demande à juste titre quel produit) et test 13 (chitchat).

## Bugs trouvés et corrigés pendant la campagne

### 1. Multi-step "prix du produit le plus vendu"

**Symptôme** : Q2 demandait le prix du top vendu. Le shortcut fire
`get_top_products(volume)` qui donne nom + quantité, mais pas le prix.
Le LLM ne savait pas enchaîner un 2e tool.

**Fix** : nouveau tool **`get_top_product_full(metric)`** qui fait les
deux étapes en interne (top + détail complet + KPIs). Résultat : le bot
répond *« Le Terreau universel 20L coûte 44.08 € avec un stock de … »*
en une seule frappe.

### 2. "presque en rupture" capté par mauvais shortcut

**Symptôme** : « produits presque en rupture » matchait le pattern
générique `produit.*rupture` → `get_alerts(CRITICAL)` au lieu de
`get_soon_out_of_stock`.

**Fix** : déplacer le pattern "presque/bientôt en rupture" AVANT les
règles génériques rupture/stock-bas.

### 3. "Pire N produits vendus" — limite pas extraite

**Symptôme** : « Pire 3 produits vendus » → flop_sales avec limit=10
(le 3 n'était pas capté).

**Fix** : changer le pattern en `\bpires?\s+(\d+)\s+...` pour mettre le
nombre dans un groupe capturant, qui est ensuite récupéré par
`match()` et passé à `limit`.

### 4. "du / de la / des" pas reconnus

**Symptôme** : « prix **du** produit X » échouait alors que « prix **de**
produit X » marchait, car mon regex `\bde\s+` ne matchait pas la
contraction "du".

**Fix** : étendre le pattern à `\b(?:de|du|des|de\s+la|de\s+l['])\s+`.

### 5. Suffixe verbal pollue le nom

**Symptôme** : « prix du produit X **convient** ? » → name capturé =
« X convient » (mot verbal inclus).

**Fix** : ajouter `(?:\s+(?:convient|est-il|est-elle|est\s+bon|est\s+correct))`
comme terminateur de la capture du nom.

## Tools ajoutés pendant la campagne

| Tool | Wrappe | Cas d'usage |
|---|---|---|
| `get_overstock` | `/stocks/overstock` | "stock trop élevé", "surstock" |
| `get_soon_out_of_stock` | `/stocks/soon-out-of-stock` | "presque en rupture" |
| `get_stock_summary` | `/stocks/summary` | "combien reste en stock", "stock bien géré" |
| `get_total_sales` | `/sales/total?period` | "combien on a vendu sur N jours" |
| `get_pending_restocks` | `/restocks/with-supplier` filtré | "livraisons en attente" |
| `get_top_product_full` | composite | "prix du produit le plus vendu" (multi-step en 1) |
| `find_product_by_name` | `/products/light?q=` | rechercher par nom |
| `get_product_by_name` | composite | détails + KPIs d'un produit par son nom |

15 tools au total maintenant disponibles pour le bot.

## Latences détaillées

Les latences élevées (> 100 s) sont **toutes** liées au formatage par le
LLM de gros payloads — pas à la décision tool. Exemples :

- Test 11 (164 s) : le bot a listé tous les ~76 produits low_stock
  un par un.
- Test 9 (140 s) : le bot a formaté tous les KPIs détaillés du produit
  (prix, marges, ventes, stock, classification, anomalies).
- Test 2 (118 s) : idem, mais sur le top vendu.

Pistes pour accélérer si nécessaire :
1. **Cap la taille du payload** : tronquer à 10 produits max avant LLM
   format (déjà fait pour shortcut path via `CHAT_TOOL_PAYLOAD_MAX_CHARS`,
   mais ici on dépasse).
2. **API LLM** (Mistral/Groq) : un Llama-3.3-70B Groq formate 76 produits
   en ~2 s contre 164 s sur Qwen-3B local.
3. **Streaming SSE** : pour que l'utilisateur voit la réponse s'écrire
   plutôt que d'attendre 3 minutes en silence.
4. **Modèle plus gros mais aussi plus rapide** : si on a GPU disponible.

## Quand passer à un autre modèle ?

Le 3B suffit pour ce panel parce qu'on a mis l'effort sur les shortcuts.
Cas où un modèle plus puissant aiderait :

- Questions composées à 3+ niveaux : « quel fournisseur du produit
  le plus vendu est le moins fiable » — nécessite chaînage de tools que
  les shortcuts ne couvrent pas.
- Reformulations très libres : « j'ai trop de trucs sur les étagères,
  qu'est-ce qui prend la poussière ? » — les regex de shortcuts ratent,
  faut que le LLM décide.

Voir [changer-de-modele.md](changer-de-modele.md) pour la recette.

## Re-lancer

```sh
python scripts/chat_eval_panel.py --provider local       # ~15-20 min
python scripts/chat_eval_panel.py --provider mistral     # ~2-3 min si clé
python scripts/chat_eval_panel.py --provider local --only "Top"   # filtre
python scripts/chat_eval_panel.py --provider local --verbose      # voir réponses
```
