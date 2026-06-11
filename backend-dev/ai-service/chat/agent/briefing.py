"""Briefing proactif — résumé de l'état du magasin au démarrage d'une session.

Quand l'utilisateur ouvre le chat, on lui montre tout de suite ce qui mérite
son attention (alertes critiques, réappros urgents, ruptures) sans qu'il ait
à demander.

Le briefing est **déterministe** (pas d'appel LLM) : il agrège quelques
endpoints et formate un texte court en français. Rapide (< 1 s) et fiable.
"""

from __future__ import annotations

from utils.logger import get_logger

from ..tools import ToolContext, execute_tool


logger = get_logger("chat.briefing")


def _count(data) -> int:
    """Compte les éléments d'un résultat outil (liste directe ou enveloppée)."""
    if isinstance(data, list):
        return len(data)
    if isinstance(data, dict):
        for k in ("alerts", "products", "restocks", "items"):
            v = data.get(k)
            if isinstance(v, list):
                return len(v)
    return 0


def build_briefing(ctx: ToolContext) -> dict:
    """Construit le briefing proactif. Retourne un dict avec un texte prêt à
    afficher + les compteurs bruts pour le frontend."""
    critical_alerts = 0
    urgent_restocks = 0
    low_stock = 0
    capped: set[str] = set()   # compteurs plafonnés par la limite de l'outil
    errors: list[str] = []

    # --- Alertes critiques (l'outil plafonne à 50) ---
    r = execute_tool("get_alerts", {"severity": "CRITICAL", "limit": 50}, ctx)
    if r.ok:
        critical_alerts = _count(r.data)
        if critical_alerts >= 50:
            capped.add("alerts")
    else:
        errors.append("alertes")

    # --- Réappros urgents ---
    r = execute_tool("get_urgent_restocks", {}, ctx)
    if r.ok:
        urgent_restocks = _count(r.data)
        if urgent_restocks >= 50:
            capped.add("restocks")
    else:
        errors.append("réappros urgents")

    # --- Stock bas : compte EXACT via le résumé de stock ---
    r = execute_tool("get_stock_summary", {}, ctx)
    if r.ok and isinstance(r.data, dict):
        # /stocks/summary expose des compteurs exacts (pas de plafond).
        d = r.data
        low_stock = (d.get("low_stock_count") or d.get("low_stock")
                     or d.get("lowStock") or 0)
        try:
            low_stock = int(low_stock)
        except (TypeError, ValueError):
            low_stock = 0
    else:
        # Repli : l'outil get_low_stock (plafonné).
        r2 = execute_tool("get_low_stock", {}, ctx)
        if r2.ok:
            low_stock = _count(r2.data)
            if low_stock >= 50:
                capped.add("low_stock")
        else:
            errors.append("stock bas")

    def _fmt(n: int, key: str) -> str:
        return f"{n}+" if key in capped else str(n)

    # --- Formatage du texte ---
    lines: list[str] = ["Bonjour 👋 Voici l'état de votre magasin :"]
    bullets: list[str] = []
    if critical_alerts:
        bullets.append(f"🔴 **{_fmt(critical_alerts, 'alerts')}** alerte(s) critique(s)")
    if urgent_restocks:
        bullets.append(f"📦 **{_fmt(urgent_restocks, 'restocks')}** produit(s) à réapprovisionner en urgence")
    if low_stock:
        bullets.append(f"⚠️ **{_fmt(low_stock, 'low_stock')}** produit(s) en stock bas")

    if bullets:
        lines.extend("- " + b for b in bullets)
        lines.append("\nDemandez-moi le détail de l'un de ces points, "
                      "ou posez votre question.")
    else:
        lines.append("✅ Rien d'urgent à signaler. Comment puis-je vous aider ?")

    if errors:
        logger.warning("briefing: échec partiel sur %s", ", ".join(errors))

    return {
        "text": "\n".join(lines),
        "critical_alerts": critical_alerts,
        "urgent_restocks": urgent_restocks,
        "low_stock": low_stock,
        "has_attention_items": bool(bullets),
        "suggestions": _suggestions(critical_alerts, urgent_restocks, low_stock),
    }


def _suggestions(alerts: int, restocks: int, low: int) -> list[str]:
    """Questions de relance proposées à l'utilisateur."""
    s: list[str] = []
    if alerts:
        s.append("Quelles sont les alertes critiques ?")
    if restocks:
        s.append("Quels produits réapprovisionner en urgence ?")
    if low:
        s.append("Montre-moi les produits en stock bas")
    # Toujours utile
    s.append("Quel est le produit le plus vendu ces 7 derniers jours ?")
    return s[:4]
