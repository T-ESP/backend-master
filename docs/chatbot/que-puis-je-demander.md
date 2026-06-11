# Que puis-je demander au chatbot ?

Catalogue **complet** de tout ce que l'assistant StockS sait faire, avec des
exemples de questions formulées comme un vrai utilisateur les poserait.

Le bot répond en français. Il n'y a pas de syntaxe à apprendre : on pose la
question naturellement. En interne, chaque demande est routée vers un des
**33 outils** ou vers la recherche documentaire (RAG). Ce document liste les
33 outils regroupés par thème.

> Astuce : les formulations ci-dessous sont des exemples. Le bot comprend les
> variantes, les fautes de frappe courantes et les accents manquants. On peut
> aussi enchaîner (« et le produit 8 ? » après une première question — le bot
> garde le contexte).

---

## 1. Ventes & chiffre d'affaires

| Ce qu'on veut savoir | Exemples de questions |
|---|---|
| CA / profit / commandes sur une période | « combien on a vendu ce mois ? » · « quel est le chiffre d'affaires des 7 derniers jours ? » · « notre profit sur 90 jours ? » · « combien de commandes cette semaine ? » · « c'est quoi le panier moyen ? » |
| Comparer deux périodes | « est-ce qu'on vend plus que le mois dernier ? » · « le CA progresse ou pas ? » · « compare ce mois vs le mois d'avant » · « notre évolution sur 60 jours » |
| Vue d'ensemble chiffrée | « donne-moi les KPI du magasin » · « un résumé global » · « comment va le business ? » |

→ outils : `get_total_sales`, `compare_sales`, `get_global_kpis`

## 2. Meilleurs & pires produits

| Ce qu'on veut savoir | Exemples de questions |
|---|---|
| Top produits | « quels sont les produits les plus vendus ? » · « top 5 par chiffre d'affaires » · « les produits les plus rentables » · « meilleure rotation de stock » |
| Le n°1 avec ses détails | « quel est le produit le plus vendu, et son prix ? » · « détails du meilleur produit » |
| Pires produits | « quels sont les produits qui se vendent le moins ? » · « les flops » · « produits les moins rentables » |
| Produits non rentables | « qu'est-ce qui me fait perdre de l'argent ? » · « produits à perte » · « produits à marge négative » |
| Stock dormant | « quels produits ne bougent pas ? » · « qu'est-ce qui dort en stock ? » · « produits invendus depuis 6 mois » |

→ outils : `get_top_products`, `get_top_product_full`, `get_negative_margin_products`, `get_dormant_stock`

## 3. Un produit précis

| Ce qu'on veut savoir | Exemples de questions |
|---|---|
| Fiche d'un produit | « parle-moi du produit 8 » · « infos sur le clavier mécanique » · « prix et stock du produit 42 » |
| Recherche par nom | « trouve-moi les produits qui contiennent "souris" » · « cherche les casques » |
| Ventes d'un produit | « combien a rapporté le produit 8 ? » · « le revenu du clavier ce mois » · « combien on a vendu d'unités du produit 12 ? » |
| Comparer deux produits | « compare le produit 8 et le produit 15 » · « lequel est mieux entre le clavier et la souris ? » |

→ outils : `get_product_detail`, `find_product_by_name`, `get_product_by_name`, `get_product_sales`, `compare_products`

## 4. Gestion du stock

| Ce qu'on veut savoir | Exemples de questions |
|---|---|
| Stock bas / rupture | « quels produits sont en rupture ? » · « produits en stock bas » |
| Bientôt en rupture | « qu'est-ce qui est presque en rupture ? » · « produits sur le point de manquer » |
| Surstock | « quels produits ont un stock trop élevé ? » · « du surstock ? » |
| Synthèse du stock | « est-ce que mon stock est bien géré ? » · « résumé de l'état du stock » · « valeur totale de mon stock » |
| Réapprovisionnements urgents | « qu'est-ce que je dois recommander en urgence ? » · « réappros urgents » |
| Livraisons attendues | « quelles livraisons sont en attente ? » · « réappros en cours » |

→ outils : `get_low_stock`, `get_soon_out_of_stock`, `get_overstock`, `get_stock_summary`, `get_urgent_restocks`, `get_pending_restocks`

## 5. Alertes & anomalies

| Ce qu'on veut savoir | Exemples de questions |
|---|---|
| Alertes actives | « quelles sont les alertes ? » · « montre-moi les alertes critiques » · « alertes de sévérité haute » |
| Anomalies de ventes | « y a-t-il des ventes anormales ? » · « des pics ou chutes inhabituels ? » |
| Anomalies de prix | « des prix suspects ? » · « anomalies de prix détectées » |

→ outils : `get_alerts`, `get_sales_anomalies`, `get_price_anomalies`

## 6. Catégories & fournisseurs

| Ce qu'on veut savoir | Exemples de questions |
|---|---|
| Analyse par catégorie | « quelle catégorie marche le mieux ? » · « compare mes catégories » · « top catégories par profit » |
| Classement des fournisseurs | « quel fournisseur est le meilleur ? » · « qui a les pires délais de livraison ? » · « fournisseur le plus fiable » |
| Profil d'un fournisseur | « score du fournisseur 3 » · « profil du fournisseur 7 » |

→ outils : `get_category_analysis`, `get_supplier_ranking`, `get_supplier_score`

## 7. Intelligence artificielle (prévisions & classification)

| Ce qu'on veut savoir | Exemples de questions |
|---|---|
| Prévision de demande | « quelle est la prévision de ventes du produit 8 ? » · « combien on va vendre du clavier le mois prochain ? » |
| Classification ABC-XYZ | « quelle est la classe ABC-XYZ du produit 8 ? » · « ce produit est dans quelle catégorie de gestion ? » |
| Suggestions de prix | « as-tu des suggestions de prix ? » · « comment optimiser mes marges ? » |

→ outils : `get_forecast`, `get_classification`, `get_price_suggestions`

## 8. Comprendre les concepts (aide / documentation)

Le bot explique les notions métier en cherchant dans la documentation interne
(RAG sémantique).

| Exemples de questions |
|---|
| « c'est quoi la classification ABC-XYZ ? » · « comment fonctionne le forecast ? » · « que veut dire le taux de rotation ? » · « explique-moi les KPI » · « comment marche le clustering ? » |

→ outil : `search_docs`

## 9. Pilotage quotidien

| Ce qu'on veut savoir | Exemples de questions |
|---|---|
| Mes priorités du jour | « qu'est-ce que je dois faire aujourd'hui ? » · « par quoi je commence ? » · « mes priorités » |
| Briefing d'ouverture | Affiché **automatiquement** à l'ouverture du chat : alertes critiques, réappros urgents, stock bas. Aucune question à poser. |

→ outils : `get_daily_action_list`, briefing proactif (`/chat/briefing`)

## 10. Actions d'écriture (le bot agit, après confirmation)

Le bot ne se contente pas de répondre : il peut **modifier les données**.
Toute action d'écriture passe par une **confirmation explicite** — le bot
décrit ce qu'il va faire, et n'exécute qu'après un « oui ».

| Action | Exemples de questions |
|---|---|
| Créer un réapprovisionnement | « crée un réappro de 50 unités du produit 8 » · « commande 100 claviers » · « passe une commande de 30 unités du produit 12 chez le fournisseur 3 » |
| Traiter une alerte | « marque l'alerte 14 comme résolue » · « passe l'alerte 7 en traitée » · « ignore l'alerte 22 » |
| Modifier un produit | « change le prix d'achat du produit 8 à 12,50 » · « corrige le stock du produit 5 à 200 » · « marque le produit 30 comme arrêté » |
| Relancer les modèles IA | « relance les prévisions » · « recalcule les modèles IA » |

→ outils : `create_restock`, `resolve_alert`, `update_product`, `trigger_ai_run`
(tous `requires_confirmation=True` — voir [ameliorations-2026-05-22.md](ameliorations-2026-05-22.md) section 5.2)

## 11. Conversation

Le bot gère aussi le bavardage : « bonjour », « merci », « comment ça va ? ».
Il reste cadré sur la gestion de stock et recentre poliment si on s'égare.

---

## Récapitulatif — les 33 outils

| # | Outil | Thème | Lecture / Écriture |
|---|---|---|---|
| 1 | `get_total_sales` | Ventes | Lecture |
| 2 | `compare_sales` | Ventes | Lecture |
| 3 | `get_global_kpis` | Ventes | Lecture |
| 4 | `get_top_products` | Produits | Lecture |
| 5 | `get_top_product_full` | Produits | Lecture |
| 6 | `get_negative_margin_products` | Produits | Lecture |
| 7 | `get_dormant_stock` | Produits | Lecture |
| 8 | `get_product_detail` | Produit précis | Lecture |
| 9 | `find_product_by_name` | Produit précis | Lecture |
| 10 | `get_product_by_name` | Produit précis | Lecture |
| 11 | `get_product_sales` | Produit précis | Lecture |
| 12 | `compare_products` | Produit précis | Lecture |
| 13 | `get_low_stock` | Stock | Lecture |
| 14 | `get_soon_out_of_stock` | Stock | Lecture |
| 15 | `get_overstock` | Stock | Lecture |
| 16 | `get_stock_summary` | Stock | Lecture |
| 17 | `get_urgent_restocks` | Stock | Lecture |
| 18 | `get_pending_restocks` | Stock | Lecture |
| 19 | `get_alerts` | Alertes | Lecture |
| 20 | `get_sales_anomalies` | Anomalies | Lecture |
| 21 | `get_price_anomalies` | Anomalies | Lecture |
| 22 | `get_category_analysis` | Catégories | Lecture |
| 23 | `get_supplier_ranking` | Fournisseurs | Lecture |
| 24 | `get_supplier_score` | Fournisseurs | Lecture |
| 25 | `get_forecast` | IA | Lecture |
| 26 | `get_classification` | IA | Lecture |
| 27 | `get_price_suggestions` | IA | Lecture |
| 28 | `search_docs` | Aide / concepts | Lecture |
| 29 | `get_daily_action_list` | Pilotage | Lecture |
| 30 | `create_restock` | Action | **Écriture** |
| 31 | `resolve_alert` | Action | **Écriture** |
| 32 | `update_product` | Action | **Écriture** |
| 33 | `trigger_ai_run` | Action | **Écriture** |

29 outils de lecture, 4 d'écriture (confirmation obligatoire).

## Ce que le bot ne fait PAS

Pour rester honnête sur les limites :

- Il ne crée pas de produits, de catégories ou de fournisseurs (lecture seule
  sur ces entités).
- Il ne supprime rien.
- Il ne gère pas les utilisateurs ni les droits.
- Le total d'**articles vendus** (unités, tous produits confondus) n'est pas
  exposé par l'API — le bot répond alors avec le **nombre de commandes** et
  propose un détail produit par produit.
- Il ne devine pas : si une question est hors de son périmètre (stock /
  ventes / IA), il le dit plutôt que d'inventer.
