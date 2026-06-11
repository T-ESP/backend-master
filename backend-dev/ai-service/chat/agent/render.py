"""Deterministic French rendering of tool results.

Motivation
----------
The local 3B model is unreliable at *transcribing* a JSON list into prose:
it invents product names ("Produit 1", "Produit 2"…) and mislabels columns.
For list-shaped tool results, the safest answer is one we build ourselves —
no LLM involved. 100 % faithful names/numbers, and instant (no 40 s call).

`render(tool_name, data)` returns ready-to-display French markdown, or
`None` when no deterministic renderer applies (the caller then falls back to
the LLM-formatted path).
"""

from __future__ import annotations

from typing import Any, Optional

NBSP = " "


# ----------------------------------------------------------------------
# Formatting helpers
# ----------------------------------------------------------------------

def _num(x: Any, decimals: int = 0) -> str:
    """Format a number the French way: space thousands sep, comma decimal."""
    if x is None or x == "":
        return "—"
    try:
        v = float(x)
    except (TypeError, ValueError):
        return str(x)
    s = f"{v:,.{decimals}f}"  # e.g. "1,234.56"
    return s.replace(",", NBSP).replace(".", ",")


def _eur(x: Any) -> str:
    """Euro amount: no decimals when large, 2 decimals when small."""
    if x is None or x == "":
        return "—"
    try:
        v = float(x)
    except (TypeError, ValueError):
        return str(x)
    return _num(v, 0 if abs(v) >= 100 else 2) + f"{NBSP}€"


def _pct(x: Any) -> str:
    if x is None or x == "":
        return "—"
    try:
        return _num(float(x), 1) + f"{NBSP}%"
    except (TypeError, ValueError):
        return str(x)


def _table(headers: list[str], rows: list[list[str]]) -> str:
    """Build a GitHub-flavoured markdown table."""
    head = "| " + " | ".join(headers) + " |"
    sep = "| " + " | ".join("---" for _ in headers) + " |"
    body = "\n".join("| " + " | ".join(r) + " |" for r in rows)
    return f"{head}\n{sep}\n{body}"


def _name(item: dict) -> str:
    return str(
        item.get("product_name")
        or item.get("name")
        or item.get("name_pro")
        or f"#{item.get('product_id') or item.get('id') or '?'}"
    )


def _as_list(data: Any, *keys: str) -> list:
    """Extract the primary list from a tool payload (direct list or under a key)."""
    if isinstance(data, list):
        return data
    if isinstance(data, dict):
        for k in keys:
            v = data.get(k)
            if isinstance(v, list):
                return v
    return []


# ----------------------------------------------------------------------
# Per-tool renderers
# ----------------------------------------------------------------------

def _r_dormant(data: dict) -> Optional[str]:
    items = _as_list(data, "produits_dormants")
    months = (data or {}).get("fenetre_mois", 3)
    if not items:
        return (f"Aucun produit ne ressort comme dormant sur les {months} "
                f"derniers mois — tout votre catalogue s'est vendu.")
    rows = [[_name(p), str(p.get("category") or "—"),
             _num(p.get("value"))] for p in items]
    intro = (f"Sur les **{months} derniers mois**, voici les produits qui se "
             f"sont le moins vendus (quantité vendue sur la période) :")
    note = ("\n\n_La colonne « Quantité vendue » est le nombre d'unités "
            "écoulées sur la période, pas le stock restant._")
    return (intro + "\n\n"
            + _table(["Produit", "Catégorie", "Quantité vendue"], rows)
            + note)


def _r_top_product_full(data: dict) -> Optional[str]:
    """Fiche détaillée du produit #1 (get_top_product_full)."""
    prod = (data or {}).get("product") or {}
    kpis = (data or {}).get("kpis") or {}
    if not prod:
        return None
    metric = (data or {}).get("metric", "volume")
    days = (data or {}).get("periode_jours")
    val = data.get("ranking_value")
    unit_label = (data or {}).get("ranking_unit", "")

    # Titre selon la métrique
    label = {"volume": "le plus vendu en quantité",
             "revenue": "qui rapporte le plus",
             "profit": "le plus profitable",
             "flop_sales": "le moins vendu"}.get(metric, "n°1")
    suffix = f" sur les {days} derniers jours" if days else ""
    titre = f"**{_name(prod)}** — produit {label}{suffix}"

    # Fiche produit
    pid = prod.get("id") or prod.get("product_id") or prod.get("id_pro")
    lines = [titre, ""]
    if pid:
        lines.append(f"- **ID produit** : {pid}")  # nécessaire pour la mémoire d'entité
    if prod.get("category"):
        lines.append(f"- **Catégorie** : {prod['category']}")
    if prod.get("reference"):
        lines.append(f"- **Référence** : {prod['reference']}")
    if prod.get("buying_price") is not None:
        lines.append(f"- **Prix d'achat** : {_eur(prod['buying_price'])}")
    if prod.get("stock_quantity") is not None:
        lines.append(f"- **Stock actuel** : {_num(prod['stock_quantity'])} unités")
    if prod.get("status"):
        lines.append(f"- **Statut** : {prod['status']}")

    # Valeur du classement
    if val is not None:
        is_money = metric in ("revenue", "profit", "flop_profit")
        val_fmt = _eur(val) if is_money else _num(val)
        col_label = {"volume": "Quantité vendue (période)",
                     "revenue": "CA (période)",
                     "profit": "Profit (période)",
                     "flop_sales": "Quantité vendue (période)",
                     "flop_profit": "Profit (période)"}.get(metric, "Valeur")
        lines.append(f"- **{col_label}** : {val_fmt}")

    # KPIs si disponibles
    sales = (kpis or {}).get("sales_rotation") or {}
    if sales.get("revenue"):
        lines.append(f"- **CA sur la période** : {_eur(sales['revenue'])}")
    if sales.get("quantity_sold"):
        lines.append(f"- **Quantité vendue** : {_num(sales['quantity_sold'])} unités")

    return "\n".join(lines)


def _r_negative_margin(data: dict) -> Optional[str]:
    a_perte = _as_list(data, "produits_a_perte")
    moins = _as_list(data, "produits_les_moins_profitables")
    if a_perte:
        rows = [[_name(p), str(p.get("category") or "—"), _eur(p.get("value"))]
                for p in a_perte]
        return ("⚠️ Ces produits sont **à perte** (profit négatif sur la "
                "période) :\n\n"
                + _table(["Produit", "Catégorie", "Profit"], rows))
    if moins:
        rows = [[_name(p), str(p.get("category") or "—"), _eur(p.get("value"))]
                for p in moins]
        return ("Bonne nouvelle : **aucun produit n'est strictement à "
                "perte**. Voici toutefois les **moins profitables** :\n\n"
                + _table(["Produit", "Catégorie", "Profit"], rows))
    return None


def _r_top_products(data: dict) -> Optional[str]:
    items = _as_list(data, "products")
    if not items:
        return "Aucun produit trouvé pour cette métrique."
    metric = (data or {}).get("metric", "revenue")
    days = (data or {}).get("periode_jours")
    is_eur = metric in ("revenue", "profit", "flop_profit")
    col = {"revenue": "Chiffre d'affaires", "profit": "Profit",
           "volume": "Quantité vendue", "turnover": "Rotation",
           "flop_sales": "Quantité vendue",
           "flop_profit": "Profit"}.get(metric, "Valeur")
    rows = []
    for i, p in enumerate(items, 1):
        val = _eur(p.get("value")) if is_eur else _num(p.get("value"))
        rows.append([str(i), _name(p), str(p.get("category") or "—"), val])
    flop = metric.startswith("flop")
    titre = ("les produits les **moins** performants" if flop
             else "les **meilleurs** produits")
    suffixe = f" sur les {days} derniers jours" if days else ""
    return (f"Voici {titre}{suffixe} :\n\n"
            + _table(["#", "Produit", "Catégorie", col], rows))


def _r_stock_list(data: Any, label: str) -> Optional[str]:
    items = _as_list(data, "products", "items")
    if not items:
        return f"Aucun produit {label}."
    rows = [[_name(p), str(p.get("category") or "—"),
             _num(p.get("stock_quantity")),
             str(p.get("reference") or "—")] for p in items]
    return (f"Voici les produits {label} ({len(items)}) :\n\n"
            + _table(["Produit", "Catégorie", "Stock", "Réf."], rows))


def _r_alerts(data: Any) -> Optional[str]:
    items = _as_list(data, "alerts")
    if not items:
        return "Aucune alerte active. 👍"
    rows = []
    for a in items:
        rows.append([
            str(a.get("severity") or "—"),
            str(a.get("product_name") or f"#{a.get('product_id') or '?'}"),
            str(a.get("message") or a.get("action_recommended") or "—"),
        ])
    return (f"**{len(items)}** alerte(s) active(s) :\n\n"
            + _table(["Sévérité", "Produit", "Message"], rows))


def _r_category(data: dict) -> Optional[str]:
    items = _as_list(data, "by_category", "categories")
    if not items:
        return None
    items = sorted(items, key=lambda c: c.get("revenue") or 0, reverse=True)
    rows = [[str(c.get("category") or "—"), _eur(c.get("revenue")),
             _eur(c.get("profit")), _pct(c.get("avg_margin_rate")),
             _num(c.get("products_count"))] for c in items]
    return ("Analyse par catégorie (triée par chiffre d'affaires) :\n\n"
            + _table(["Catégorie", "CA", "Profit", "Marge", "Produits"], rows))


def _r_supplier(data: dict) -> Optional[str]:
    items = _as_list(data, "by_supplier", "suppliers")
    if not items:
        return None
    items = sorted(items, key=lambda s: s.get("revenue") or 0, reverse=True)
    rows = [[str(s.get("supplier_name") or f"#{s.get('supplier_id')}"),
             _eur(s.get("revenue")), _eur(s.get("profit")),
             _num(s.get("avg_delivery_delay_days"), 1) + f"{NBSP}j",
             _pct(s.get("reliability_rate"))] for s in items]
    return ("Classement des fournisseurs (trié par chiffre d'affaires) :\n\n"
            + _table(["Fournisseur", "CA", "Profit", "Délai moyen",
                      "Fiabilité"], rows))


def _r_urgent_restocks(data: Any) -> Optional[str]:
    items = _as_list(data, "restocks", "items")
    if not items:
        return "Aucun produit à réapprovisionner en urgence. 👍"
    # Le plus pressant d'abord (rupture imminente).
    items = sorted(
        items,
        key=lambda r: (r.get("days_until_stockout")
                       if r.get("days_until_stockout") is not None else 999),
    )
    total = len(items)
    shown = items[:20]
    rows = []
    for r in shown:
        d = r.get("days_until_stockout")
        rupture = "déjà en rupture" if not d else f"{_num(d)}{NBSP}j"
        qty = r.get("reorder_quantity")
        if qty is None:
            qty = r.get("recommended_stock")
        rows.append([_name(r), _num(r.get("current_stock")),
                     rupture, _num(qty)])
    intro = f"**{total}** produit(s) à réapprovisionner en urgence"
    if total > len(shown):
        intro += f" — voici les {len(shown)} plus pressants"
    intro += " :"
    note = ("\n\n_« À commander » = quantité recommandée par la prévision de "
            "demande pour couvrir les ventes à venir (pas le stock actuel)._")
    return (intro + "\n\n"
            + _table(["Produit", "Stock actuel", "Rupture dans",
                      "À commander"], rows)
            + note)


def _r_daily_actions(data: dict) -> Optional[str]:
    actions = _as_list(data, "actions")
    if (data or {}).get("rien_a_signaler") or not actions:
        return "Rien d'urgent aujourd'hui — aucune action prioritaire. 👍"
    parts = ["Voici vos priorités du jour :\n"]
    for a in actions:
        line = (f"**{a.get('priorite')}. {a.get('categorie')}** "
                f"— {a.get('nombre')} élément(s)\n"
                f"   → {a.get('action')}")
        ex = a.get("exemples")
        if ex:
            line += "\n   _Ex. : " + " ; ".join(str(e) for e in ex) + "_"
        parts.append(line)
    return "\n\n".join(parts)


# ----------------------------------------------------------------------
# Dispatch
# ----------------------------------------------------------------------

_RENDERERS = {
    "get_dormant_stock": _r_dormant,
    "get_negative_margin_products": _r_negative_margin,
    "get_top_products": _r_top_products,
    "get_alerts": _r_alerts,
    "get_category_analysis": _r_category,
    "get_supplier_ranking": _r_supplier,
    "get_daily_action_list": _r_daily_actions,
    "get_urgent_restocks": _r_urgent_restocks,
    "get_top_product_full": _r_top_product_full,
    "get_low_stock": lambda d: _r_stock_list(d, "en stock bas"),
    "get_soon_out_of_stock": lambda d: _r_stock_list(d, "bientôt en rupture"),
    "get_overstock": lambda d: _r_stock_list(d, "en surstock"),
}


def render(tool_name: str, data: Any) -> Optional[str]:
    """Return deterministic French markdown for a tool result, or None.

    None means « no deterministic renderer » → caller falls back to the LLM.
    Any rendering exception also yields None (graceful degradation).
    """
    fn = _RENDERERS.get(tool_name)
    if fn is None:
        return None
    try:
        out = fn(data)
        return out if out and out.strip() else None
    except Exception:  # noqa: BLE001 — never let rendering break a turn
        return None
