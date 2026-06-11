"""Génération déterministe de questions de suivi suggérées.

Après chaque réponse d'outil, on génère 2-4 questions pertinentes
pré-résolues (tool + args déjà remplis). Le frontend peut les afficher
comme boutons — cliquer déclenche directement /chat/execute-tool, sans
passer par le LLM, réponse instantanée.
"""
from __future__ import annotations

from dataclasses import dataclass, asdict
from typing import Any, Optional


@dataclass
class Suggestion:
    text: str       # Texte FR affiché à l'utilisateur
    tool: str       # Outil à appeler directement
    args: dict      # Arguments pré-résolus


def _pid(d: dict) -> Optional[int]:
    for k in ("id", "product_id", "id_pro"):
        v = d.get(k)
        if v is not None:
            try:
                return int(v)
            except (TypeError, ValueError):
                pass
    return None


def _pname(d: dict) -> Optional[str]:
    for k in ("name", "product_name", "name_pro"):
        v = d.get(k)
        if v:
            return str(v)
    return None


def _as_list(data: Any, *keys: str) -> list:
    if isinstance(data, list):
        return data
    if isinstance(data, dict):
        for k in keys:
            v = data.get(k)
            if isinstance(v, list):
                return v
    return []


def generate(tool_name: str, data: Any) -> list[dict]:
    """Retourne une liste de suggestions (max 4) sérialisables en JSON."""
    sug: list[Suggestion] = []

    # ── Fiche produit (get_top_product_full, get_product_detail, get_product_by_name)
    if tool_name in ("get_top_product_full", "get_product_detail",
                     "get_product_by_name"):
        prod = {}
        if tool_name == "get_top_product_full":
            prod = (data or {}).get("product") or {}
        elif tool_name == "get_product_detail":
            prod = (data or {}).get("product") or {}
        elif tool_name == "get_product_by_name":
            prod = (data or {}).get("matched_product") or {}

        pid = _pid(prod)
        name = _pname(prod) or "ce produit"
        short = name[:30] + ("…" if len(name) > 30 else "")

        if pid:
            sug.append(Suggestion(
                text=f"Jusqu'à quand le stock de {short} va durer ?",
                tool="get_forecast",
                args={"product_id": pid},
            ))
            sug.append(Suggestion(
                text=f"CA de {short} sur les 30 derniers jours",
                tool="get_product_sales",
                args={"product_id": pid, "period_days": 30},
            ))
            sug.append(Suggestion(
                text=f"Classification ABC-XYZ de {short}",
                tool="get_classification",
                args={"product_id": pid},
            ))
            # Réappro — uniquement si stock bas
            stock = (prod.get("stock_quantity") or 0)
            if isinstance(stock, (int, float)) and stock < 50:
                sug.append(Suggestion(
                    text=f"Créer un réapprovisionnement pour {short}",
                    tool="create_restock",
                    args={"product_id": pid, "quantity": 50},
                ))

    # ── Top produits (get_top_products)
    elif tool_name == "get_top_products":
        metric = (data or {}).get("metric", "volume")
        days = (data or {}).get("periode_jours", 30)
        opposite = "flop_sales" if metric != "flop_sales" else "revenue"
        opposite_label = "les moins vendus" if opposite == "flop_sales" else "par chiffre d'affaires"
        sug.append(Suggestion(
            text=f"Et les produits {opposite_label} sur la même période ?",
            tool="get_top_products",
            args={"metric": opposite, "limit": 10, "period_days": days},
        ))
        sug.append(Suggestion(
            text="Comparer cette période avec la précédente",
            tool="compare_sales",
            args={"period_days": days},
        ))
        sug.append(Suggestion(
            text="Analyse par catégorie",
            tool="get_category_analysis",
            args={},
        ))

    # ── Réappros urgents (get_urgent_restocks)
    elif tool_name == "get_urgent_restocks":
        items = _as_list(data, "restocks", "items")
        for item in items[:2]:
            pid = item.get("product_id")
            pname = (item.get("product_name") or f"produit {pid}")[:25]
            qty = int(item.get("reorder_quantity") or item.get("recommended_stock") or 1)
            if pid:
                sug.append(Suggestion(
                    text=f"Commander {qty} unités — {pname}",
                    tool="create_restock",
                    args={"product_id": pid, "quantity": qty},
                ))
        sug.append(Suggestion(
            text="Voir toutes les alertes critiques",
            tool="get_alerts",
            args={"severity": "CRITICAL", "limit": 20},
        ))

    # ── Alertes (get_alerts)
    elif tool_name == "get_alerts":
        items = _as_list(data, "alerts")
        if items:
            first = items[0]
            aid = first.get("id")
            if aid:
                sug.append(Suggestion(
                    text=f"Marquer l'alerte #{aid} comme traitée",
                    tool="resolve_alert",
                    args={"alert_id": aid, "status": "resolved"},
                ))
        sug.append(Suggestion(
            text="Produits à réapprovisionner en urgence",
            tool="get_urgent_restocks",
            args={},
        ))
        sug.append(Suggestion(
            text="Actions prioritaires du jour",
            tool="get_daily_action_list",
            args={},
        ))

    # ── Actions du jour (get_daily_action_list)
    elif tool_name == "get_daily_action_list":
        sug.append(Suggestion(
            text="Réappros urgents en détail",
            tool="get_urgent_restocks",
            args={},
        ))
        sug.append(Suggestion(
            text="Alertes critiques",
            tool="get_alerts",
            args={"severity": "CRITICAL", "limit": 20},
        ))

    # ── KPI global (get_global_kpis)
    elif tool_name == "get_global_kpis":
        sug.append(Suggestion(
            text="Comparer avec la période précédente",
            tool="compare_sales",
            args={"period_days": 30},
        ))
        sug.append(Suggestion(
            text="Quels produits marchent le mieux ?",
            tool="get_top_products",
            args={"metric": "volume", "limit": 10, "period_days": 30},
        ))
        sug.append(Suggestion(
            text="Actions prioritaires du jour",
            tool="get_daily_action_list",
            args={},
        ))

    # ── Catégories (get_category_analysis)
    elif tool_name == "get_category_analysis":
        sug.append(Suggestion(
            text="Top produits par chiffre d'affaires",
            tool="get_top_products",
            args={"metric": "revenue", "limit": 10},
        ))
        sug.append(Suggestion(
            text="Classement des fournisseurs",
            tool="get_supplier_ranking",
            args={},
        ))

    # ── Fournisseurs (get_supplier_ranking)
    elif tool_name == "get_supplier_ranking":
        sug.append(Suggestion(
            text="Analyse par catégorie",
            tool="get_category_analysis",
            args={},
        ))
        sug.append(Suggestion(
            text="Réapprovisionnements en attente",
            tool="get_pending_restocks",
            args={"limit": 20},
        ))

    return [asdict(s) for s in sug[:4]]
