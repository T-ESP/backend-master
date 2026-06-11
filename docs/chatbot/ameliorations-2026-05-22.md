# Améliorations chatbot — 2026-05-22

Round « rendre le bot plus intelligent » : briefing proactif, comparaison
temporelle, garde-fou anti-hallucination, streaming SSE — puis round
« plus de questions couvertes » : 5 outils d'analyse et 3 actions
d'écriture avec passerelle de confirmation (section 5).

## 1. Briefing proactif

À l'ouverture d'une session, le bot montre l'état du magasin sans qu'on
demande.

- **Endpoint** : `GET /chat/briefing` (Rust) → `POST /chat/briefing` (Python)
- **Déterministe** : pas d'appel LLM, agrège alertes critiques + réappros
  urgents + stock bas. Réponse < 1 s.
- Le frontend l'affiche comme premier message à l'ouverture du chat.
- Compteurs : stock bas est **exact** (via `/stocks/summary`) ; alertes et
  réappros affichent « 50+ » quand l'outil plafonne, pour rester honnête.
- Renvoie aussi des **questions de relance** suggérées.

Exemple :
```
Bonjour 👋 Voici l'état de votre magasin :
- 🔴 50+ alerte(s) critique(s)
- 📦 50+ produit(s) à réapprovisionner en urgence
- ⚠️ 68 produit(s) en stock bas
```

## 2. Comparaison temporelle

Nouvel outil **`compare_sales(period_days)`** : compare la période récente
à la période précédente de même durée.

- Calcule la variation en % du CA, du profit, du nombre de commandes.
- Wrappe `/kpis/global-performance` appelé sur deux fenêtres.
- Shortcut prioritaire sur les mots « vs », « compar », « évolution »,
  « progresse », « mois dernier », « qu'avant ».

Exemple vérifié :
> « est-ce qu'on vend plus que le mois dernier ? »
> → « Le chiffre d'affaires est passé de 726 750 € à 741 397 €, soit +2,0 %. »

## 3. Garde-fou anti-hallucination

`chat/agent/verify.py` — `verify_numbers()` vérifie que les nombres
significatifs cités dans une réponse proviennent réellement des données
renvoyées par les outils.

- Extrait les nombres de la réponse ET du payload outil.
- Normalise FR/EN (« 4 167,47 » == « 4167.47 »).
- Tolère arrondis et reformatages.
- Ignore les petits nombres (< 100) et les années.
- Si un nombre significatif est **inexpliqué** → `numbers_verified = false`
  + log d'alerte. Une réponse non vérifiée n'est **pas mise en cache**.
- Champ `numbers_verified` exposé dans l'API REST et l'événement SSE `done`.

Motivation : on a observé le LLM local inventer des chiffres plausibles
(« 560 196 € », « 1400 € ») absents des données. Le garde-fou les détecte.

Vérifié : « 560 196 € » halluciné est bien flaggé ; « 4 167,47 € » réel
passe ; les arrondis (« environ 4167 ») passent ; sans données outil,
aucun faux positif.

> Note : le garde-fou **signale**, il ne corrige pas automatiquement. Le
> frontend peut afficher un avertissement quand `numbers_verified` est
> faux. Un retry correctif automatique est possible en évolution.

## 4. Streaming SSE

Déjà en place (implémenté lors d'un round précédent) :

- `POST /chat/sessions/:id/messages/stream` (Rust, proxy SSE) →
  `POST /chat/turn/stream` (Python).
- Événements émis : `ping`, `intent`, `shortcut`, `tool_call`, `cached`,
  `delta`, `pending_action`, `done`.
- L'événement `done` porte désormais `numbers_verified` et `shortcut_used`.

Limite connue : le `delta` arrive en un seul bloc (pas token-par-token).
Le vrai streaming token-par-token demande de transformer l'orchestrateur
en générateur — laissé pour une évolution. Les événements de progression
donnent déjà un ressenti « vivant ».

## 5. Nouveaux outils — analyse & actions d'écriture

Round « plus de questions couvertes » : 5 outils de lecture pour l'analyse,
3 outils d'écriture pour agir directement depuis le chat.

### 5.1 Outils de lecture

| Outil | Question type | Source |
|---|---|---|
| `get_category_analysis` | « quelle catégorie marche le mieux ? » | `/kpis/category-analysis` |
| `get_supplier_ranking` | « quel fournisseur est le pire ? » | `/kpis/supplier-analysis` |
| `get_daily_action_list` | « qu'est-ce que je dois faire aujourd'hui ? » | agrège 3 outils |
| `get_dormant_stock` | « quels produits ne bougent pas ? » | `get_top_products` (flop_sales) |
| `get_negative_margin_products` | « qu'est-ce qui me fait perdre de l'argent ? » | `get_top_products` (flop_profit) |

- **`get_daily_action_list`** est composite : il appelle `get_alerts`
  (CRITICAL), `get_urgent_restocks` et `get_sales_anomalies`, puis renvoie
  une liste **priorisée** (priorité 1→3) avec un compteur et des exemples.
- **`get_dormant_stock`** prend un paramètre optionnel `months` (1-12, défaut
  3) ; `value` = quantité vendue sur la fenêtre, seuil dormant ≤ 5 unités.
- **`get_negative_margin_products`** distingue `produits_a_perte`
  (profit < 0, strictement à perte) de `produits_les_moins_profitables`
  (top 10 des plus faibles) — honnête quand aucun produit n'est à perte.

Chacun a des shortcuts regex (`shortcuts.py`) pour court-circuiter le LLM ;
ceux qui acceptent une période sont dans `_PERIOD_AWARE_TOOLS`.

### 5.2 Outils d'écriture (passerelle de confirmation)

| Outil | Effet | API appelée |
|---|---|---|
| `create_restock` | Crée une commande de réapprovisionnement | `POST /restocks` |
| `resolve_alert` | Change le statut d'une alerte | `PUT /alerts/{id}/status` |
| `update_product` | Modifie prix d'achat / stock / statut | `PUT /products/{id}` |

Tous portent `requires_confirmation=True`. Le flux :

1. L'utilisateur formule l'action (« crée un réappro de 50 unités du
   produit 8 »).
2. `intent.py` la classe `action` (regex accent-optionnels) ; l'orchestrateur
   **saute les shortcuts de lecture** (`skip_shortcuts = intent == "action"`)
   pour qu'un outil de lecture ne détourne pas la demande.
3. Le bot renvoie un **`pending_action`** (l'action n'est PAS exécutée) avec
   `tool_name`, `tool_args`, `action_id`, et un message de confirmation FR.
4. Après « oui », Rust appelle `POST /chat/execute-tool` qui exécute
   réellement l'outil avec le JWT de l'utilisateur.

`create_restock` complète automatiquement `unit_price` (prix d'achat du
produit) et `supplier_id` (fournisseur du produit) si non fournis.

**Test live vérifié** (2026-05-22) :
> « cree un reapprovisionnement de 50 unites du produit 8 »
> → `intent=action`, `shortcut=None`,
> `pending_action = create_restock(product_id=8, quantity=50)`,
> statut `pending` — l'action attend la confirmation, rien n'est écrit.

## 6. Rendu déterministe des listes (anti-hallucination renforcé)

Problème observé en live : sur « qu'est-ce qui dort en stock depuis
6 mois », le modèle local 3B a **inventé** les noms (« Produit 1 »,
« Produit 2 »…) et une colonne « Stock » erronée — alors que l'outil
renvoyait les vrais produits. Le garde-fou nombres (section 3) ne couvre
pas les **noms** ni les petits nombres.

Correctif — `chat/agent/render.py` :

- Pour les résultats **en forme de liste** (produits dormants, top/flop,
  marge négative, stock bas, alertes, catégories, fournisseurs, réappros
  urgents, actions du jour), la réponse est **construite en Python**, pas
  par le LLM.
- Le chemin shortcut (`_shortcut_turn`) tente d'abord `render()` : si un
  rendu déterministe existe, il est renvoyé tel quel — `provider_used =
  "deterministic"`, **aucun appel LLM**.
- Effet : noms et chiffres 100 % fidèles, et latence qui passe de ~40 s à
  **< 1 s** sur ces questions.
- Formatage FR : séparateur de milliers (espace), virgule décimale, « € ».
- Repli : si aucun renderer ne s'applique (ou exception), retour à `None`
  → le LLM formate comme avant.

Deux corrections de shortcut associées :

- Le shortcut « stock dormant » matche désormais aussi « dort en stock »,
  « qui dort », « ne tournent pas », « stagnent » (avant : seulement
  « dorment »).
- La fenêtre en mois est extraite de la question (« depuis 6 mois » →
  `months=6`) au lieu du défaut figé à 3.

Vérifié en live : « qu'est-ce qui dort en stock depuis 6 mois » →
`shortcut=get_dormant_stock(months=6)`, `provider=deterministic`, 0,6 s,
vrais noms de produits, colonne « Quantité vendue » explicitée.

## Tests

10 nouveaux tests unitaires (verify + normalisation) puis 7 de plus
(rendu déterministe + shortcuts dormant). Suite chat : **64/64**.

## Variables d'environnement

Aucune nouvelle variable. Les seuils du garde-fou sont des constantes dans
`verify.py` (`_SIGNIFICANCE_THRESHOLD = 100`, `_REL_TOL = 0.02`).

## Endpoints ajoutés

| Méthode | Chemin | Rôle |
|---|---|---|
| `GET` | `/chat/briefing` | Briefing proactif de l'état du magasin |
| `POST` | `/chat/execute-tool` | Exécute un outil confirmé (actions d'écriture) |

## Outils ajoutés

| Outil | Type | Rôle |
|---|---|---|
| `compare_sales(period_days)` | lecture | Compare ventes période récente vs précédente |
| `get_category_analysis` | lecture | Analyse CA/profit/marge par catégorie |
| `get_supplier_ranking` | lecture | Classement complet des fournisseurs |
| `get_daily_action_list` | lecture | Liste priorisée des actions du jour |
| `get_dormant_stock(months?)` | lecture | Produits invendus / stock dormant |
| `get_negative_margin_products` | lecture | Produits à perte / peu rentables |
| `create_restock` | écriture | Crée une commande de réappro (confirmation) |
| `resolve_alert` | écriture | Change le statut d'une alerte (confirmation) |
| `update_product` | écriture | Modifie prix/stock/statut (confirmation) |
