"""Deterministic shortcuts for very common French questions.

The cheapest, fastest, most reliable answer is the one we don't ask the LLM
to figure out. For a small set of well-known phrasings, we can:

  1. Match the question against a regex.
  2. Pick the right tool + args directly.
  3. Call the tool.
  4. Hand the result + a one-line system instruction to the LLM, which only
     needs to format prose around fresh data.

Net effect: tool-call reliability becomes ~100% for these patterns, and
latency drops because the model only does one pass instead of decide-call-decide.
"""

from __future__ import annotations

import re
from dataclasses import dataclass
from typing import Optional


@dataclass
class Shortcut:
    """A direct mapping from a phrasing to a tool call."""
    tool: str
    args: dict


# Order matters — first match wins. Keep simple, unambiguous phrasings only;
# anything fuzzy goes to the LLM-based router.
_SHORTCUTS: list[tuple[re.Pattern, Shortcut]] = [
    # ---- prix / coût / stock / marge du produit "le plus vendu" / "qui rapporte le plus"
    # PRIORITÉ HAUTE — doit gagner contre le shortcut générique "produit le plus vendu"
    (re.compile(r"\b(prix|tarif|co[ûu]t|stock|marge|d[ée]tails?)\b.*\bproduit\b.*\b(le\s+)?plus\s+vendu", re.IGNORECASE),
     Shortcut("get_top_product_full", {"metric": "volume"})),
    (re.compile(r"\b(prix|tarif|co[ûu]t|stock|marge|d[ée]tails?)\b.*\bproduit\b.*\bqui\s+rapporte\b.*\bplus", re.IGNORECASE),
     Shortcut("get_top_product_full", {"metric": "revenue"})),
    (re.compile(r"\b(prix|tarif|co[ûu]t|stock|marge|d[ée]tails?)\b.*\bmeilleur\s+produit", re.IGNORECASE),
     Shortcut("get_top_product_full", {"metric": "revenue"})),

    # ---- "quel est LE produit le plus vendu" (singulier) → fiche complète du #1
    # PRIORITÉ HAUTE — doit gagner contre le shortcut pluriel ci-dessous.
    # Singulier = "le produit" / "l'article" / "quel est" sans "les"/"des".
    (re.compile(r"\bquel\s+(est\s+)?l[e']?\s*(produit|article)\b.*\bplus\s+vendus?\b", re.IGNORECASE),
     Shortcut("get_top_product_full", {"metric": "volume"})),
    (re.compile(r"\bquel\s+(est\s+)?l[e']?\s*(produit|article)\b.*\bmoins\s+vendus?\b", re.IGNORECASE),
     Shortcut("get_top_product_full", {"metric": "flop_sales"})),
    (re.compile(r"\bquel\s+(est\s+)?l[e']?\s*(produit|article)\b.*\bqui\s+(se\s+vend|rapporte)\s+le\s+plus\b", re.IGNORECASE),
     Shortcut("get_top_product_full", {"metric": "revenue"})),
    # "le produit le plus vendu" sans "quel est" mais toujours au singulier
    (re.compile(r"\bproduit\b(?!\s*s\b).*\ble\s+plus\s+vendu\b", re.IGNORECASE),
     Shortcut("get_top_product_full", {"metric": "volume"})),
    (re.compile(r"\bproduit\b(?!\s*s\b).*\ble\s+moins\s+vendu\b", re.IGNORECASE),
     Shortcut("get_top_product_full", {"metric": "flop_sales"})),

    # ---- "quels produits / les produits les plus vendus" (pluriel) → top-10
    # Doit gagner contre le shortcut get_total_sales ("combien vendu ... jours")
    # car ici on veut le PRODUIT, pas le CA global. La période est extraite
    # automatiquement par match() si présente ("ces 7 jours", "ce mois"...).
    (re.compile(r"\b(produits?|articles?)\b.*\b(le|les)\s+plus\s+vendus?\b", re.IGNORECASE),
     Shortcut("get_top_products", {"metric": "volume", "limit": 10})),
    (re.compile(r"\b(produits?|articles?)\b.*\b(le|les)\s+moins\s+vendus?\b", re.IGNORECASE),
     Shortcut("get_top_products", {"metric": "flop_sales", "limit": 10})),
    (re.compile(r"\b(produits?|articles?)\b.*\bqui\s+(se\s+vend|rapporte)\b", re.IGNORECASE),
     Shortcut("get_top_products", {"metric": "volume", "limit": 10})),

    # ---- presque/bientôt en rupture — DOIT être avant les règles génériques rupture/stock-bas
    (re.compile(r"\b(presque|bient[ôo]t|sur\s+le\s+point)\s+(en\s+|d[''e]\s*[êe]tre\s+en\s+|d['']\s*[êe]tre\s+en\s+)?ruptures?\b", re.IGNORECASE),
     Shortcut("get_soon_out_of_stock", {})),
    (re.compile(r"\b(quels?|liste|donne)\b.*\b(produits?|articles?)\b.*\b(presque|bient[ôo]t)\b.*\brupture\b", re.IGNORECASE),
     Shortcut("get_soon_out_of_stock", {})),

    # ---- low-stock / rupture variations
    (re.compile(r"\b(quels|liste|donne(?:-moi)?|montre|affiche)\b.*\b(produits?|articles?)\b.*\b(en\s+)?stock\s*(bas|faible|critique)\b", re.IGNORECASE),
     Shortcut("get_low_stock", {})),
    (re.compile(r"\b(quels|liste|donne(?:-moi)?|montre|affiche)\b.*\b(produits?|articles?)\b.*\b(en\s+)?rupture\b", re.IGNORECASE),
     Shortcut("get_alerts", {"severity": "CRITICAL"})),
    (re.compile(r"\bcombien\b.*\b(produits?|articles?)\b.*\b(rupture|stock\s*bas)\b", re.IGNORECASE),
     Shortcut("get_low_stock", {})),

    # (les règles "presque en rupture" sont définies plus haut, avant les
    # règles génériques low-stock / rupture pour qu'elles gagnent.)

    # ---- surstock / stock trop élevé
    (re.compile(r"\b(surstock|sur-stock)\b", re.IGNORECASE),
     Shortcut("get_overstock", {})),
    (re.compile(r"\bstock\b.*\b(trop|tr[èe]s)\s+(élev[ée]|haut|grand|important)\b", re.IGNORECASE),
     Shortcut("get_overstock", {})),
    (re.compile(r"\b(produits?|articles?)\b.*\bqui\s+dorment\b", re.IGNORECASE),
     Shortcut("get_overstock", {})),
    (re.compile(r"\b(produits?|articles?)\b.*\b(trop|excès\s+de)\s+stock\b", re.IGNORECASE),
     Shortcut("get_overstock", {})),

    # ---- liste d'actions du jour
    (re.compile(r"\b(qu['e ].*(dois|faut|faire).*aujourd|que\s+dois-je\s+faire|"
                r"mes\s+priorit[ée]s|par\s+quoi\s+(je\s+)?commenc|"
                r"actions?\s+(du\s+jour|à\s+(faire|mener))|à\s+faire\s+aujourd)",
                re.IGNORECASE),
     Shortcut("get_daily_action_list", {})),

    # ---- analyse par catégorie
    (re.compile(r"\b(cat[ée]gorie)s?\b.*\b(marche|performe|rentab|meilleur|"
                r"compar|analyse|top)\b", re.IGNORECASE),
     Shortcut("get_category_analysis", {})),
    (re.compile(r"\b(quelle?|meilleure?)\s+cat[ée]gorie\b", re.IGNORECASE),
     Shortcut("get_category_analysis", {})),
    (re.compile(r"\bcompar\w*\b.*\bcat[ée]gorie", re.IGNORECASE),
     Shortcut("get_category_analysis", {})),

    # ---- classement fournisseurs
    (re.compile(r"\b(meilleurs?|pires?|top|class\w+|fiabl\w+|moins\s+fiabl)\b"
                r".*\bfournisseur", re.IGNORECASE),
     Shortcut("get_supplier_ranking", {})),
    (re.compile(r"\bfournisseurs?\b.*\b(meilleurs?|pires?|fiabl\w+|d[ée]lais?|"
                r"class\w+|compar\w+|le\s+plus|le\s+moins)\b", re.IGNORECASE),
     Shortcut("get_supplier_ranking", {})),
    (re.compile(r"\b(quel|quels)\s+fournisseurs?\b", re.IGNORECASE),
     Shortcut("get_supplier_ranking", {})),

    # ---- stock dormant ("dort/dorment en stock", "qui dort", "stagne"…)
    (re.compile(r"\b(stock\s+dormant|produits?\s+dormants?|qui\s+(ne\s+)?"
                r"bougent?\s+pas|qui\s+dor[tms]\w*|invendus?|dor\w*\s+en\s+"
                r"stock|ne\s+se\s+vendent?\s+pas|stagnent?|ne\s+tournent?\s+"
                r"pas)\b", re.IGNORECASE),
     Shortcut("get_dormant_stock", {"months": 3})),

    # ---- produits à perte / marge négative
    (re.compile(r"\b(perdre|perte|perds|non\s+rentabl|pas\s+rentabl|"
                r"marge\s+n[ée]gativ|à\s+perte|me\s+co[ûu]te)\b.*\b(argent|"
                r"produits?|articles?)\b", re.IGNORECASE),
     Shortcut("get_negative_margin_products", {})),
    (re.compile(r"\b(produits?|articles?)\b.*\b(perte|non\s+rentabl|"
                r"marge\s+n[ée]gativ|font\s+perdre)\b", re.IGNORECASE),
     Shortcut("get_negative_margin_products", {})),

    # ---- comparaison temporelle — AVANT get_total_sales (mots "vs", "progresse"…)
    (re.compile(r"\b(vs|versus|compar|par\s+rapport|évolution|evolution|progress|"
                r"mois\s+dernier|p[ée]riode\s+pr[ée]c[ée]dente|qu['']avant|"
                r"plus\s+qu['']avant|tendance)\b", re.IGNORECASE),
     Shortcut("compare_sales", {"period_days": 30})),

    # ---- chiffre d'affaires sur N jours / "combien on a vendu"
    (re.compile(r"\b(combien|quel)\b.*\b(vendu|chiffre\s+d['']affaires|ca|revenus?)\b.*\b(\d+)?\s*(derniers?\s+)?(jours?|mois|semaines?)\b", re.IGNORECASE),
     Shortcut("get_total_sales", {"period_days": 30})),
    (re.compile(r"\bca\s+(des\s+|sur\s+les?\s+)?(\d+)?\s*(derniers?\s+)?(jours?|mois|semaines?)\b", re.IGNORECASE),
     Shortcut("get_total_sales", {"period_days": 30})),
    (re.compile(r"\b(combien|quel\s+est)\b.*\b(j['']ai|nous\s+avons|on\s+a)\b.*\bvendu", re.IGNORECASE),
     Shortcut("get_total_sales", {"period_days": 30})),

    # ---- stock global / résumé / "stock bien géré" / "stock total"
    (re.compile(r"\b(résumé|recap|summary|état)\b.*\bstock", re.IGNORECASE),
     Shortcut("get_stock_summary", {})),
    (re.compile(r"\bstock\b.*\b(bien\s+géré|sant[ée]|en\s+forme)", re.IGNORECASE),
     Shortcut("get_stock_summary", {})),
    # "combien (de) stock... reste" et "combien reste... stock" (les deux ordres)
    (re.compile(r"\bcombien\b.*\breste\b.*\bstock", re.IGNORECASE),
     Shortcut("get_stock_summary", {})),
    (re.compile(r"\bcombien\b.*\bstock\b.*\breste\b", re.IGNORECASE),
     Shortcut("get_stock_summary", {})),
    # "stock total", "stock global", "stock au total"
    (re.compile(r"\bstock\s+(total|global|au\s+total|en\s+tout)\b", re.IGNORECASE),
     Shortcut("get_stock_summary", {})),
    (re.compile(r"\b(total|globalit[ée])\s+(de\s+)?(notre\s+|du\s+|le\s+)?stock", re.IGNORECASE),
     Shortcut("get_stock_summary", {})),
    # "il reste combien" / "il nous reste"
    (re.compile(r"\b(il\s+)?(nous\s+)?reste\b.*\b(combien|quoi)\b.*\bstock", re.IGNORECASE),
     Shortcut("get_stock_summary", {})),

    # ---- réapprovisionnements en cours/en attente
    (re.compile(r"\b(livraisons?|réapprovisionnements?|réappros?)\b.*\b(en\s+cours|en\s+attente|pendants?|à\s+venir)\b", re.IGNORECASE),
     Shortcut("get_pending_restocks", {"limit": 20})),
    (re.compile(r"\bquels?\s+(livraisons?|réapprovisionnements?)\b", re.IGNORECASE),
     Shortcut("get_pending_restocks", {"limit": 20})),
    (re.compile(r"\bréapprovisionnements?\s+(à\s+faire|nécessaires?|urgents?)\b", re.IGNORECASE),
     Shortcut("get_urgent_restocks", {})),

    # ---- alerts
    (re.compile(r"\b(quelles?|liste|donne(?:-moi)?|montre)\b.*\balertes?\s+critiques?\b", re.IGNORECASE),
     Shortcut("get_alerts", {"severity": "CRITICAL", "limit": 20})),
    (re.compile(r"\b(quelles?|liste|donne(?:-moi)?|montre)\b.*\balertes?\b(?!\s+critiques?)", re.IGNORECASE),
     Shortcut("get_alerts", {"limit": 20})),

    # ---- "Top N produits" sans métrique → défaut revenue
    (re.compile(r"\btop\s+(\d+)\s+(des\s+|de\s+)?(produits?|articles?)\s*\??$", re.IGNORECASE),
     Shortcut("get_top_products", {"metric": "revenue", "limit": 10})),
    (re.compile(r"\btop\s+(\d+)\s+(des\s+|de\s+)?(produits?|articles?)\b(?!.*\b(vendus?|volume|profit|marge|ca|chiffre|argent)\b)", re.IGNORECASE),
     Shortcut("get_top_products", {"metric": "revenue", "limit": 10})),

    # ---- "Pire N produits vendus" / "N pires produits"
    # On capture le nombre dans un groupe pour que match() le passe à `limit`.
    (re.compile(r"\bpires?\s+(\d+)\s+(des\s+|de\s+)?(produits?|articles?)\b.*\b(vendus?|ventes?)\b", re.IGNORECASE),
     Shortcut("get_top_products", {"metric": "flop_sales", "limit": 10})),
    (re.compile(r"\b(\d+)\s+pires?\b.*\b(produits?|articles?)\b", re.IGNORECASE),
     Shortcut("get_top_products", {"metric": "flop_sales", "limit": 10})),
    # Variante sans nombre : "pire produits vendus", "produits les pires"
    (re.compile(r"\bpires?\b.*\b(produits?|articles?)\b.*\b(vendus?|ventes?)\b", re.IGNORECASE),
     Shortcut("get_top_products", {"metric": "flop_sales", "limit": 10})),

    # ---- top products — par chiffre d'affaires / revenu / argent rapporté
    (re.compile(r"\btop\s*(\d+)?\b.*\bproduits?\b.*\b(chiffre|ca|revenu|revenue|argent)\b", re.IGNORECASE),
     Shortcut("get_top_products", {"metric": "revenue", "limit": 10})),
    # "produit qui rapporte le plus (d'argent)" / "rapporte le plus"
    (re.compile(r"\b(produits?|articles?)\b.*\b(rapporte|génère|generates?)\b.*\b(le\s+)?plus\b", re.IGNORECASE),
     Shortcut("get_top_products", {"metric": "revenue", "limit": 10})),
    # "X produits qui rapportent le plus"
    (re.compile(r"\b(\d+)?\s*(produits?|articles?)\b.*\bqui\s+rapportent?\b", re.IGNORECASE),
     Shortcut("get_top_products", {"metric": "revenue", "limit": 10})),

    # ---- top products — par profit / marge
    (re.compile(r"\btop\s*(\d+)?\b.*\bproduits?\b.*\b(profit|marge)\b", re.IGNORECASE),
     Shortcut("get_top_products", {"metric": "profit", "limit": 10})),
    (re.compile(r"\b(produits?|articles?)\b.*\bplus\s+(rentables?|profitables?)\b", re.IGNORECASE),
     Shortcut("get_top_products", {"metric": "profit", "limit": 10})),

    # ---- top products — par volume / quantité (le plus vendu)
    (re.compile(r"\b(meilleurs?|best)\b.*\bvente(?:ur)?s?\b", re.IGNORECASE),
     Shortcut("get_top_products", {"metric": "volume", "limit": 10})),
    # "produit le plus vendu" / "produits les plus vendus" / "qui se vend le plus"
    (re.compile(r"\b(produits?|articles?)\b.*\b(le\s+|les\s+)?plus\s+vendus?\b", re.IGNORECASE),
     Shortcut("get_top_products", {"metric": "volume", "limit": 10})),
    (re.compile(r"\b(produits?|articles?)\b.*\bse\s+vendent?\b.*\b(le\s+)?(plus|mieux)\b", re.IGNORECASE),
     Shortcut("get_top_products", {"metric": "volume", "limit": 10})),
    # "X produits les plus vendus"
    (re.compile(r"\b(\d+)?\s*(produits?|articles?)\b.*\b(plus|mieux)\s+vendus?\b", re.IGNORECASE),
     Shortcut("get_top_products", {"metric": "volume", "limit": 10})),

    # ---- bottom / flop — "produit le moins vendu", "qui se vend mal"
    (re.compile(r"\b(produits?|articles?)\b.*\b(le\s+|les\s+)?moins\s+vendus?\b", re.IGNORECASE),
     Shortcut("get_top_products", {"metric": "flop_sales", "limit": 10})),
    (re.compile(r"\b(produits?|articles?)\b.*\bse\s+vendent?\s+(le\s+)?(moins|mal|pas)\b", re.IGNORECASE),
     Shortcut("get_top_products", {"metric": "flop_sales", "limit": 10})),
    (re.compile(r"\b(produits?|articles?)\b.*\b(rentables?|profitables?)\b.*\bmoins\b", re.IGNORECASE),
     Shortcut("get_top_products", {"metric": "flop_profit", "limit": 10})),

    # ---- KPI overview / "comment va le business" / "santé de l'entreprise"
    (re.compile(r"\b(résumé|recap|recapitulatif|vue d['']ensemble|overview|tableau de bord|dashboard)\b", re.IGNORECASE),
     Shortcut("get_global_kpis", {"period_days": 30})),
    (re.compile(r"\b(performance|kpis?|indicateurs?)\b.*\b(global(?:e|ux|aux)?|général(?:e|aux)?)\b", re.IGNORECASE),
     Shortcut("get_global_kpis", {"period_days": 30})),
    # "comment va mon business / mon stock / mes affaires / mon commerce"
    (re.compile(r"\bcomment\s+va\b.*\b(business|stock|affaires?|commerce|entreprise|magasin|boutique)\b", re.IGNORECASE),
     Shortcut("get_global_kpis", {"period_days": 30})),
    (re.compile(r"\b(état|santé|sant[ée]|forme|bilan)\b.*\b(business|stock|affaires?|commerce|entreprise|magasin|boutique)\b", re.IGNORECASE),
     Shortcut("get_global_kpis", {"period_days": 30})),
    (re.compile(r"\bdonne(?:-moi)?\b.*\b(état|aperçu|bilan|résumé|stats|chiffres)\b", re.IGNORECASE),
     Shortcut("get_global_kpis", {"period_days": 30})),
    # "ça se passe bien" / "tout va bien"
    (re.compile(r"\b(ça\s+se\s+passe|tout\s+va)\b.*\b(bien|comment)\b", re.IGNORECASE),
     Shortcut("get_global_kpis", {"period_days": 30})),

    # ---- product detail (numeric ID)
    (re.compile(r"\b(détails?|infos?|fiche)\b.*\bproduit\b.*\b(\d+)\b", re.IGNORECASE),
     None),  # Special: needs to extract id, handled below
    (re.compile(r"\bproduit\s*#?(\d+)\b", re.IGNORECASE),
     None),  # Same

    # ---- product by name (prix/stock/marge/info "de NOM_DE_PRODUIT")
    # On utilise un groupe nommé `name` pour que l'extraction ne se fasse pas
    # sur le mot-clé (prix, stock, etc.) qui précède.
    # Accepte "de", "du", "des", "de la", "de l'" (contractions FR).
    (re.compile(
        r"\b(?:prix|tarif|co[ûu]t|stock|marge|d[ée]tails?|infos?|fiche|caract[ée]ristiques?)\b"
        r".*?\b(?:de|du|des|de\s+la|de\s+l[''])\s+(?:produit\s+|article\s+)?"
        r"(?P<name>.+?)(?:\s*\?|$|\s+(?:convient|est-il|est-elle|est\s+bon|est\s+correct))",
        re.IGNORECASE),
     None),
    (re.compile(
        r"\b(?:combien\s+co[ûu]te|quel\s+est\s+le\s+prix\s+(?:unitaire\s+)?(?:de|du|des))\b"
        r"\s+(?:le\s+|la\s+|les\s+|l[''])?(?:produit\s+|article\s+)?"
        r"(?P<name>.+?)(?:\s*\?|$|\s+(?:convient|est-il|est-elle))",
        re.IGNORECASE),
     None),
    # "combien (d'unités/il en) reste du/de [NOM]" / "stock restant du [NOM]"
    # "il me reste combien de [NOM]" / "combien j'ai de [NOM]"
    (re.compile(
        r"\b(?:combien\b.*?\b(?:reste[nt]?|units?|unit[ée]s?|en\s+stock|j['']ai|il\s+(?:me|nous)\s+reste[nt]?)|"
        r"stock\s+(?:restant|actuel|disponible))\b"
        r".*?\b(?:de|du|des|de\s+la|de\s+l[''])\s+(?:le\s+|la\s+|l[''])?(?:produit\s+|article\s+)?"
        r"(?P<name>[A-ZÀ-Ÿ0-9][\wÀ-ÿ0-9 -]{2,50}?)(?:\s*\?|$)",
        re.IGNORECASE),
     None),
]


_NAME_STOPWORDS = {
    "le", "la", "les", "un", "une", "des", "de", "du", "ce", "cet", "cette",
    "mon", "ma", "mes", "ton", "ta", "tes", "son", "sa", "ses",
}


def _clean_name(raw: str) -> str:
    """Nettoie un nom de produit capturé par regex (espaces, ponctuation finale,
    articles en tête, mots-outils en queue)."""
    s = raw.strip().rstrip("?.!,;:").strip()
    # Strippe les articles en tête : "le X", "la X", "les X", "l'X", "un X", etc.
    s = re.sub(r"^(le|la|les|l'|un|une|des|du|de\s+la|de\s+l')\s+", "", s, flags=re.IGNORECASE)
    # Coupe au premier connecteur typique qui signale la fin du nom
    for sep in [" pour ", " avec ", " de la marque ", " stp", " svp", " merci"]:
        idx = s.lower().find(sep)
        if idx > 0:
            s = s[:idx].strip()
    return s.strip()


# Outils qui acceptent un filtre de période (period_days).
_PERIOD_AWARE_TOOLS = {
    "get_top_products", "get_top_product_full", "get_total_sales",
    "get_global_kpis", "compare_sales", "get_product_sales",
}

# Outils dont la fenêtre s'exprime en mois (param `months`, pas `period_days`).
_MONTHS_AWARE_TOOLS = {"get_dormant_stock"}


def extract_period_days(text: str) -> Optional[int]:
    """Détecte une période dans une question FR et la convertit en jours.

    Exemples : 'ces 7 derniers jours' → 7, 'cette semaine' → 7,
    'ce mois' → 30, 'cette année' → 365, 'hier' → 1, '3 mois' → 90.
    Retourne None si aucune période n'est mentionnée.
    """
    t = text.lower()
    # "N jours / semaines / mois / ans"
    # Le mot entre le nombre et l'unité (ex: "derniers", "dernieers", "prochains",
    # "dernières"…) est optionnel ET tolérant aux fautes de frappe — on matche
    # n'importe quel mot unique (1-15 chars) éventuellement présent.
    m = re.search(r"\b(\d+)\s+(?:[a-zA-ZÀ-ÿ]{1,15}\s+)?(jours?|semaines?|mois|ans?|années?)\b", t)
    if not m:
        # Fallback : "N jours" collé sans espace intermédiaire (ex: "7jours")
        m = re.search(r"\b(\d+)(jours?|semaines?|mois|ans?|années?)\b", t)
    if m:
        n = int(m.group(1))
        unit = m.group(2)
        if unit.startswith("jour"):
            return max(1, min(365, n))
        if unit.startswith("semaine"):
            return max(1, min(365, n * 7))
        if unit.startswith("mois"):
            return max(1, min(365, n * 30))
        if unit.startswith(("an", "ann", "ané", "ann")):
            return max(1, min(365, n * 365))
    # Expressions sans nombre
    if re.search(r"\b(aujourd['']hui|ce\s+jour)\b", t):
        return 1
    if re.search(r"\bhier\b", t):
        return 2
    if re.search(r"\bcette\s+semaine\b|\b7\s+derniers\b|\bsemaine\s+derni[èe]re\b", t):
        return 7
    if re.search(r"\bce\s+mois\b|\bmois\s+dernier\b|\bdu\s+mois\b", t):
        return 30
    if re.search(r"\bce\s+trimestre\b|\btrimestre\b", t):
        return 90
    if re.search(r"\bcette\s+ann[ée]e\b|\bann[ée]e\b", t):
        return 365
    return None


def match(question: str) -> Optional[Shortcut]:
    """Return the matching shortcut, or None if no rule fires."""
    text = question.strip()
    if not text:
        return None
    period = extract_period_days(text)

    for pattern, shortcut in _SHORTCUTS:
        m = pattern.search(text)
        if not m:
            continue

        if shortcut is None:
            # Cas 1 : un groupe nommé `name` (= prix/stock/info DE <nom>).
            groups_dict = m.groupdict()
            named = groups_dict.get("name")
            if named:
                cleaned = _clean_name(named)
                if len(cleaned) >= 3 and cleaned.lower() not in _NAME_STOPWORDS:
                    return Shortcut("get_product_by_name", {"name": cleaned})
                continue
            # Cas 2 : extraction d'un ID numérique.
            for g in m.groups():
                if g and g.isdigit():
                    return Shortcut("get_product_detail", {"product_id": int(g)})
            continue

        # Construit les args finaux : copie + overrides éventuels.
        args = dict(shortcut.args)
        # Injecte la période détectée si l'outil la supporte.
        if period is not None and shortcut.tool in _PERIOD_AWARE_TOOLS:
            args["period_days"] = period
        # Outils en mois : convertit la période détectée en mois (1-12).
        if period is not None and shortcut.tool in _MONTHS_AWARE_TOOLS:
            args["months"] = max(1, min(12, round(period / 30)))
        # Top-N override: if pattern has a captured number, use it as limit.
        for g in m.groups():
            if g and g.isdigit() and "limit" in args:
                # On ne confond pas le nombre de la période avec la limite :
                # si ce nombre fait partie de l'expression de période, on saute.
                if period is not None and int(g) in (period, period // 7, period // 30):
                    continue
                args["limit"] = int(g)
                break
        return Shortcut(shortcut.tool, args)
    return None
