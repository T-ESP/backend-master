"""Tool registry + executor.

Tools delegate to internal Rust API endpoints over the Docker network.
The user's JWT is forwarded so RBAC stays correct.
"""

from __future__ import annotations

import json
import os
from dataclasses import dataclass, field
from typing import Any, Callable, Optional

import requests

from utils.logger import get_logger

from ..types import ToolParam, ToolSpec
from ..rag import retrieve, format_context


STOCKS_API_URL = os.getenv("STOCKS_API_URL", "http://web:8080").rstrip("/")
HTTP_TIMEOUT = float(os.getenv("CHAT_TOOL_HTTP_TIMEOUT", "15"))

logger = get_logger("chat.tools")


@dataclass
class ToolContext:
    """Runtime context passed to every tool invocation.

    Multi-tenant: `commerce_id` scopes every Rust-API call to the caller's tenant
    (`/api/{commerce_id}/...`). `user_id` is kept only for backward compatibility
    and is unused (master auth is commerce-level)."""
    user_jwt: str
    session_id: str = ""
    commerce_id: str = ""
    slug: str = ""
    user_id: int = 0

    @property
    def auth_headers(self) -> dict[str, str]:
        return {"Authorization": f"Bearer {self.user_jwt}"}


# ai_predictions endpoints are nested under `/ai/predictions` in
# stocks_api_master, whereas the original flat API exposed them at `/ai/...`.
_AI_PREDICTION_PREFIXES = (
    "/ai/forecasts", "/ai/classifications", "/ai/clusters", "/ai/supplier-scores",
    "/ai/price-suggestions", "/ai/price-anomalies", "/ai/sales-anomalies",
    "/ai/urgent-restocks",
)


def _tenant_url(ctx: ToolContext, path: str) -> str:
    """Build a tenant-scoped Rust-API URL from a flat path.

    Prepends `/api/{commerce_id}` and remaps the ai_predictions endpoints to
    their nested `/ai/predictions/...` location in stocks_api_master.
    """
    for pref in _AI_PREDICTION_PREFIXES:
        if path == pref or path.startswith(pref + "/"):
            path = "/ai/predictions" + path[len("/ai"):]
            break
    base = f"/api/{ctx.commerce_id}" if ctx.commerce_id else ""
    return f"{STOCKS_API_URL}{base}{path}"


MAX_LIST_ITEMS = int(os.getenv("CHAT_TOOL_MAX_LIST_ITEMS", "12"))


def _parallel(*fns):
    from concurrent.futures import ThreadPoolExecutor
    with ThreadPoolExecutor(max_workers=len(fns)) as ex:
        futures = [ex.submit(fn) for fn in fns]
        out = []
        for f in futures:
            try:
                out.append(f.result())
            except Exception:
                out.append(None)
        return out


def _compress(value: Any, depth: int = 0) -> Any:
    """Réduit récursivement les grosses listes pour ne pas saturer le contexte
    du LLM. Une liste de N>MAX éléments est tronquée à MAX, avec un marqueur
    qui indique le total. Les questions volumineuses (76 produits low_stock)
    deviennent ainsi rapides à formater sans perte d'information clé."""
    if depth > 6:
        return value
    if isinstance(value, list):
        if len(value) > MAX_LIST_ITEMS:
            kept = [_compress(v, depth + 1) for v in value[:MAX_LIST_ITEMS]]
            kept.append({
                "_tronque": (
                    f"{len(value)} éléments au total ; {MAX_LIST_ITEMS} affichés. "
                    f"Mentionne le total ({len(value)}) à l'utilisateur et résume "
                    f"l'échantillon visible."
                )
            })
            return kept
        return [_compress(v, depth + 1) for v in value]
    if isinstance(value, dict):
        return {k: _compress(v, depth + 1) for k, v in value.items()}
    return value


@dataclass
class ToolResult:
    ok: bool
    data: Any = None
    error: Optional[str] = None

    def to_payload(self, compress: bool = True) -> str:
        """Serialize for injection back into the LLM context.

        `compress=True` réduit les grosses listes (voir _compress) pour
        accélérer le formatage par le LLM et éviter les dépassements de
        contexte."""
        if self.ok:
            data = _compress(self.data) if compress else self.data
            return json.dumps({"ok": True, "data": data}, ensure_ascii=False, default=str)
        return json.dumps({"ok": False, "error": self.error or "unknown error"}, ensure_ascii=False)


class ToolError(RuntimeError):
    pass


@dataclass
class _Tool:
    spec: ToolSpec
    func: Callable[[ToolContext, dict[str, Any]], ToolResult]


_REGISTRY: dict[str, _Tool] = {}


def register(spec: ToolSpec):
    """Decorator: register a tool implementation."""
    def deco(func: Callable[[ToolContext, dict[str, Any]], ToolResult]) -> Callable:
        _REGISTRY[spec.name] = _Tool(spec=spec, func=func)
        return func
    return deco


def catalog() -> list[ToolSpec]:
    """All registered tools (read + write)."""
    return [t.spec for t in _REGISTRY.values()]


def tool_specs(read_only: bool = False) -> list[ToolSpec]:
    """Return tool specs filtered by read/write."""
    if read_only:
        return [t.spec for t in _REGISTRY.values() if not t.spec.requires_confirmation]
    return [t.spec for t in _REGISTRY.values()]


def get_tool(name: str) -> Optional[_Tool]:
    return _REGISTRY.get(name)


def execute_tool(name: str, args: dict[str, Any], ctx: ToolContext) -> ToolResult:
    tool = _REGISTRY.get(name)
    if tool is None:
        return ToolResult(ok=False, error=f"Outil inconnu: {name}")
    try:
        return tool.func(ctx, args or {})
    except requests.HTTPError as e:
        body = ""
        if e.response is not None:
            body = e.response.text[:200]
        logger.warning("Tool %s HTTP error: %s — %s", name, e, body)
        return ToolResult(ok=False, error=f"HTTP {e.response.status_code if e.response else '?'}: {body}")
    except requests.RequestException as e:
        logger.warning("Tool %s request failed: %s", name, e)
        return ToolResult(ok=False, error=f"Erreur réseau: {e}")
    except Exception as e:
        logger.exception("Tool %s crashed", name)
        return ToolResult(ok=False, error=f"Exception: {e}")


# ----------------------------------------------------------------------
# HTTP helper
# ----------------------------------------------------------------------

def _api_get(ctx: ToolContext, path: str, params: dict | None = None) -> Any:
    url = _tenant_url(ctx, path)
    r = requests.get(url, headers=ctx.auth_headers, params=params, timeout=HTTP_TIMEOUT)
    r.raise_for_status()
    body = r.json()
    return body.get("data", body) if isinstance(body, dict) else body


def _period_params(args: dict, default_days: int = 30) -> tuple[dict, int]:
    """Construit les paramètres start_date/end_date pour les endpoints qui
    acceptent une période. Retourne (params, days_effectifs)."""
    from datetime import date, timedelta
    raw = args.get("period_days")
    if raw in (None, "", 0, "0"):
        days = default_days
    else:
        try:
            days = max(1, min(365, int(raw)))
        except (TypeError, ValueError):
            days = default_days
    end = date.today()
    start = end - timedelta(days=days)
    return {"start_date": start.isoformat(), "end_date": end.isoformat()}, days


def _api_post(ctx: ToolContext, path: str, payload: dict | None = None) -> Any:
    url = _tenant_url(ctx, path)
    r = requests.post(url, headers=ctx.auth_headers, json=payload or {}, timeout=HTTP_TIMEOUT)
    r.raise_for_status()
    if r.text:
        body = r.json()
        return body.get("data", body) if isinstance(body, dict) else body
    return None


# ======================================================================
# READ TOOLS
# ======================================================================

@register(ToolSpec(
    name="get_global_kpis",
    description="Récupère les KPI globaux agrégés (CA, profit, alertes, top produits, prévisions, santé du catalogue) pour les N derniers jours.",
    params=[
        ToolParam("period_days", "integer",
                  "Nombre de jours à analyser. Par défaut 30.",
                  required=False),
    ],
))
def _get_global_kpis(ctx: ToolContext, args: dict) -> ToolResult:
    days = int(args.get("period_days", 30))
    from datetime import date, timedelta
    end = date.today()
    start = end - timedelta(days=days)
    raw = _api_get(ctx, "/ai/insights",
                    params={"start_date": start.isoformat(), "end_date": end.isoformat()})

    # Renomme les clés sensibles en français explicite pour éviter que le
    # LLM mistranslate (ex: 'total_profit' rendu en 'pertes totales').
    perf = (raw or {}).get("global_performance") or {}
    if perf:
        renamed = {
            "chiffre_affaires_eur": perf.get("total_revenue"),
            "benefice_profit_positif_eur": perf.get("total_profit"),
            "taux_marge_moyen_pourcent": perf.get("avg_margin_rate"),
            "nombre_commandes": perf.get("total_orders_count"),
            "panier_moyen_eur": perf.get("global_avg_basket_value"),
            "valeur_stock_au_cout_eur": perf.get("total_stock_value_cost"),
            "valeur_stock_potentielle_vente_eur": perf.get("total_stock_value_potential"),
            "produits_total": perf.get("total_products_count"),
            "produits_actifs": perf.get("active_products_count"),
            "produits_inactifs": perf.get("inactive_products_count"),
            "produits_arretes_discontinues": perf.get("discontinued_products_count"),
        }
        # Remplace global_performance par la version FR ; conserve le reste.
        raw = {**raw, "global_performance": renamed}

    return ToolResult(ok=True, data=raw)


@register(ToolSpec(
    name="get_top_products",
    description=(
        "Liste les produits selon une métrique, sur une période choisie. "
        "Pour les MEILLEURS produits, utiliser revenue/profit/volume/turnover. "
        "Pour les PIRES (moins bien classés), utiliser flop_sales (moins "
        "vendus) ou flop_profit (moins profitables). Le paramètre period_days "
        "permet de filtrer ('7 derniers jours', 'ce mois'…)."
    ),
    params=[
        ToolParam("metric", "string",
                  "Métrique: revenue (CA), profit, volume (qté vendue), "
                  "turnover (rotation), flop_sales (les moins vendus), "
                  "flop_profit (les moins profitables).",
                  required=True,
                  enum=["revenue", "profit", "volume", "turnover",
                        "flop_sales", "flop_profit"]),
        ToolParam("limit", "integer", "Nombre de résultats (1-20). Par défaut 10.", required=False),
        ToolParam("period_days", "integer",
                  "Période d'analyse en jours (1-365). Par défaut 30.", required=False),
    ],
))
def _get_top_products(ctx: ToolContext, args: dict) -> ToolResult:
    params, days = _period_params(args, default_days=30)
    data = _api_get(ctx, "/kpis/top-flop", params=params)
    metric = (args.get("metric") or "revenue").lower()
    limit = max(1, min(20, int(args.get("limit", 10))))
    key_map = {
        "revenue": "top_10_by_revenue",
        "profit": "top_10_by_profit",
        "volume": "top_10_by_volume",
        "turnover": "top_10_by_turnover",
        "flop_sales": "flop_10_by_sales",
        "flop_profit": "flop_10_by_profit",
    }
    key = key_map.get(metric, "top_10_by_revenue")
    selected = (data or {}).get(key, [])[:limit]

    # Annoter explicitement l'unité de la métrique pour éviter que le LLM
    # confonde par exemple un chiffre d'affaires cumulé avec un prix unitaire.
    unit_label = {
        "revenue": "chiffre d'affaires cumulé en euros (somme de toutes les ventes sur la période)",
        "profit": "profit cumulé en euros sur la période",
        "volume": "quantité totale vendue (unités, pas euros)",
        "turnover": "taux de rotation du stock (sans unité)",
        "flop_sales": "ventes faibles — quantité totale vendue (unités) — produits les MOINS vendus",
        "flop_profit": "profit faible en euros — produits les MOINS profitables",
    }.get(metric, "valeur de la métrique")
    return ToolResult(ok=True, data={
        "metric": metric,
        "periode_jours": days,
        "unit": unit_label,
        "products": selected,
    })


@register(ToolSpec(
    name="get_product_detail",
    description=(
        "Récupère les informations détaillées d'un produit par son identifiant. "
        "Si tu n'as que le nom, utilise d'abord find_product_by_name pour "
        "récupérer l'identifiant."
    ),
    params=[
        ToolParam("product_id", "integer", "Identifiant du produit.", required=True),
    ],
))
def _get_product_detail(ctx: ToolContext, args: dict) -> ToolResult:
    pid = int(args["product_id"])
    detail, kpis = _parallel(
        lambda: _api_get(ctx, f"/products/{pid}"),
        lambda: _api_get(ctx, f"/products/{pid}/kpis/all"),
    )
    return ToolResult(ok=True, data={"product": detail, "kpis": kpis})


@register(ToolSpec(
    name="find_product_by_name",
    description=(
        "Recherche un produit par son nom (ou un fragment). Renvoie une liste "
        "de candidats avec leur id_pro, nom, catégorie, stock et prix. "
        "Utiliser quand l'utilisateur cite un produit par son nom sans donner "
        "l'identifiant numérique."
    ),
    params=[
        ToolParam("query", "string", "Nom ou partie du nom du produit à chercher.", required=True),
        ToolParam("limit", "integer", "Nombre max de résultats (1-20). Par défaut 5.", required=False),
    ],
))
def _find_product_by_name(ctx: ToolContext, args: dict) -> ToolResult:
    q = str(args.get("query", "")).strip()
    if not q:
        return ToolResult(ok=False, error="query est requis")
    limit = max(1, min(20, int(args.get("limit", 5))))
    matches = _api_get(ctx, "/products/light", params={"q": q})
    if isinstance(matches, list):
        matches = matches[:limit]
    return ToolResult(ok=True, data={"query": q, "matches": matches})


@register(ToolSpec(
    name="get_product_by_name",
    description=(
        "Récupère TOUT (détails + KPIs) sur un produit identifié par son nom. "
        "Fait la recherche par nom puis le get_product_detail en une seule étape. "
        "À utiliser quand l'utilisateur pose une question sur un produit cité "
        "par son nom (prix, stock, marge, ventes, etc.)."
    ),
    params=[
        ToolParam("name", "string", "Nom du produit (peut être partiel).", required=True),
    ],
))
def _get_product_by_name(ctx: ToolContext, args: dict) -> ToolResult:
    name = str(args.get("name", "")).strip()
    if not name:
        return ToolResult(ok=False, error="name est requis")
    matches = _api_get(ctx, "/products/light", params={"q": name})
    if not isinstance(matches, list) or not matches:
        return ToolResult(ok=False, error=f"Aucun produit trouvé pour: {name}")

    # L'endpoint /products/light renvoie {id, name, category} (pas id_pro).
    def _id(m: dict) -> int | None:
        v = m.get("id") or m.get("id_pro")
        try:
            return int(v) if v is not None else None
        except (TypeError, ValueError):
            return None

    def _name(m: dict) -> str:
        return (m.get("name") or m.get("name_pro") or "").strip()

    # Privilégie un match exact (case-insensitive) ; sinon le premier.
    best = next(
        (m for m in matches if _name(m).lower() == name.lower()),
        matches[0],
    )
    pid = _id(best)
    if not pid:
        return ToolResult(ok=False, error=f"Le produit '{_name(best)}' n'a pas d'identifiant utilisable")
    detail, kpis = _parallel(
        lambda: _api_get(ctx, f"/products/{pid}"),
        lambda: _api_get(ctx, f"/products/{pid}/kpis/all"),
    )
    other_matches = [m for m in matches if _id(m) != pid][:5]
    return ToolResult(ok=True, data={
        "queried_name": name,
        "matched_product": detail,
        "kpis": kpis,
        "other_matches": other_matches,
    })


def _resolve_product_id(ctx: ToolContext, args: dict) -> tuple[int | None, str | None]:
    """Résout un identifiant produit depuis args (product_id direct OU name).
    Retourne (id, erreur)."""
    pid_raw = args.get("product_id")
    if pid_raw not in (None, "", 0, "0"):
        try:
            return int(pid_raw), None
        except (TypeError, ValueError):
            pass
    name = str(args.get("name", "")).strip()
    if not name:
        return None, "product_id ou name requis"
    matches = _api_get(ctx, "/products/light", params={"q": name})
    if not isinstance(matches, list) or not matches:
        return None, f"Aucun produit trouvé pour: {name}"
    exact = next((m for m in matches
                  if (m.get("name") or "").lower() == name.lower()), matches[0])
    pid = exact.get("id") or exact.get("id_pro")
    try:
        return int(pid), None
    except (TypeError, ValueError):
        return None, f"Produit '{name}' sans identifiant"


@register(ToolSpec(
    name="get_product_sales",
    description=(
        "Chiffre d'affaires (revenu) ET quantité vendue d'UN produit précis "
        "sur une période. À utiliser pour 'combien a rapporté le produit X', "
        "'le revenu de ces N unités', 'combien on a vendu de X ce mois'. "
        "Le produit peut être désigné par son nom OU son identifiant."
    ),
    params=[
        ToolParam("name", "string", "Nom du produit (si pas d'identifiant).", required=False),
        ToolParam("product_id", "integer", "Identifiant du produit (prioritaire sur name).", required=False),
        ToolParam("period_days", "integer",
                  "Période d'analyse en jours (1-365). Par défaut 30.", required=False),
    ],
))
def _get_product_sales(ctx: ToolContext, args: dict) -> ToolResult:
    pid, err = _resolve_product_id(ctx, args)
    if err:
        return ToolResult(ok=False, error=err)
    params, days = _period_params(args, default_days=30)
    data = _api_get(ctx, f"/products/{pid}/kpis/sales-rotation", params=params)
    # On annote en clair pour éviter toute confusion avec le CA global.
    return ToolResult(ok=True, data={
        "product_id": pid,
        "periode_jours": days,
        "quantite_vendue_unites": (data or {}).get("quantity_sold"),
        "revenu_de_ce_produit_eur": (data or {}).get("revenue"),
        "nombre_de_commandes": (data or {}).get("order_count"),
        "tendance": (data or {}).get("sales_trend"),
        "_note": (
            "Ces chiffres concernent UNIQUEMENT ce produit sur la période — "
            "ce n'est PAS le chiffre d'affaires global du magasin."
        ),
    })


@register(ToolSpec(
    name="get_alerts",
    description="Liste les alertes actives (rupture, anomalie, etc.) filtrables par sévérité.",
    params=[
        ToolParam("severity", "string",
                  "Sévérité: CRITICAL | HIGH | MEDIUM | LOW. Optionnel.",
                  required=False,
                  enum=["CRITICAL", "HIGH", "MEDIUM", "LOW"]),
        ToolParam("limit", "integer", "Nombre d'alertes (1-50). Par défaut 20.", required=False),
    ],
))
def _get_alerts(ctx: ToolContext, args: dict) -> ToolResult:
    params: dict[str, Any] = {"limit": max(1, min(50, int(args.get("limit", 20))))}
    sev = args.get("severity")
    if sev:
        params["severity"] = str(sev).upper()
    data = _api_get(ctx, "/alerts", params=params)
    return ToolResult(ok=True, data=data)


@register(ToolSpec(
    name="get_low_stock",
    description="Liste les produits avec un stock bas ou en rupture.",
    params=[],
))
def _get_low_stock(ctx: ToolContext, args: dict) -> ToolResult:
    data = _api_get(ctx, "/stocks/low-stock")
    return ToolResult(ok=True, data=data)


@register(ToolSpec(
    name="get_soon_out_of_stock",
    description=(
        "Liste les produits 'bientôt en rupture' = stock faible avec forte "
        "demande. À utiliser quand l'utilisateur demande 'presque en rupture', "
        "'sur le point d'être en rupture', etc."
    ),
    params=[],
))
def _get_soon_out_of_stock(ctx: ToolContext, args: dict) -> ToolResult:
    data = _api_get(ctx, "/stocks/soon-out-of-stock")
    return ToolResult(ok=True, data=data)


@register(ToolSpec(
    name="get_overstock",
    description=(
        "Liste les produits avec un stock anormalement élevé par rapport à "
        "la vitesse de vente. Sert pour 'stock trop élevé', 'surstock', "
        "'produits qui dorment'."
    ),
    params=[],
))
def _get_overstock(ctx: ToolContext, args: dict) -> ToolResult:
    data = _api_get(ctx, "/stocks/overstock")
    return ToolResult(ok=True, data=data)


@register(ToolSpec(
    name="get_stock_summary",
    description=(
        "Synthèse globale du stock : total produits, ruptures, stock bas, "
        "surstock, valeur totale du stock. Utile pour 'est-ce que mon stock "
        "est bien géré ?'."
    ),
    params=[],
))
def _get_stock_summary(ctx: ToolContext, args: dict) -> ToolResult:
    data = _api_get(ctx, "/stocks/summary")
    return ToolResult(ok=True, data=data)


@register(ToolSpec(
    name="get_total_sales",
    description=(
        "Chiffre d'affaires, nombre de commandes, profit et panier moyen sur "
        "une période. Utiliser pour 'combien on a vendu les N derniers jours', "
        "'CA du mois', 'combien de commandes', etc."
    ),
    params=[
        ToolParam("period_days", "integer",
                  "Nombre de jours à analyser (1-365). Par défaut 30.", required=False),
    ],
))
def _get_total_sales(ctx: ToolContext, args: dict) -> ToolResult:
    days = max(1, min(365, int(args.get("period_days", 30))))
    from datetime import date, timedelta
    end = date.today()
    start = end - timedelta(days=days)
    period_params = {"start_date": start.isoformat(), "end_date": end.isoformat()}

    # /sales/total ne donne que le CA. On enrichit avec /kpis/global-performance
    # qui apporte le nombre de commandes, le profit, le panier moyen.
    kpis, revenue_raw = _parallel(
        lambda: _api_get(ctx, "/kpis/global-performance", params=period_params),
        lambda: _api_get(ctx, "/sales/total", params=period_params),
    )
    revenue = (revenue_raw or {}).get("total_revenue") if isinstance(revenue_raw, dict) else None

    summary = {
        "period_days": days,
        "total_revenue_eur": revenue if revenue is not None else (kpis or {}).get("total_revenue"),
        "total_orders": (kpis or {}).get("total_orders_count"),
        "total_profit_eur": (kpis or {}).get("total_profit"),
        "avg_basket_value_eur": (kpis or {}).get("global_avg_basket_value"),
        "avg_margin_rate_pct": (kpis or {}).get("avg_margin_rate"),
        "note_unites": (
            "Le total d'articles vendus (unités) n'est pas exposé par cette "
            "API. Si l'utilisateur le demande explicitement, indique le nombre "
            "de COMMANDES (total_orders) à la place et propose de regarder "
            "produit par produit avec get_top_products(metric='volume')."
        ),
    }
    return ToolResult(ok=True, data=summary)


@register(ToolSpec(
    name="compare_sales",
    description=(
        "Compare les ventes de la période récente avec la période précédente "
        "de même durée (CA, profit, commandes) et calcule l'évolution en %. "
        "À utiliser pour 'le CA progresse-t-il', 'ce mois vs le mois dernier', "
        "'est-ce qu'on vend plus qu'avant'."
    ),
    params=[
        ToolParam("period_days", "integer",
                  "Durée de chaque période en jours (1-180). Par défaut 30. "
                  "Ex: 30 compare les 30 derniers jours aux 30 jours d'avant.",
                  required=False),
    ],
))
def _compare_sales(ctx: ToolContext, args: dict) -> ToolResult:
    from datetime import date, timedelta
    raw = args.get("period_days")
    try:
        days = max(1, min(180, int(raw))) if raw not in (None, "", 0, "0") else 30
    except (TypeError, ValueError):
        days = 30
    today = date.today()
    cur_start = today - timedelta(days=days)
    prev_start = today - timedelta(days=days * 2)
    prev_end = cur_start

    def _kpis(s: date, e: date) -> dict:
        try:
            return _api_get(ctx, "/kpis/global-performance",
                            params={"start_date": s.isoformat(),
                                    "end_date": e.isoformat()}) or {}
        except requests.HTTPError:
            return {}

    cur = _kpis(cur_start, today)
    prev = _kpis(prev_start, prev_end)

    def _delta(metric: str) -> dict:
        c = cur.get(metric) or 0
        p = prev.get(metric) or 0
        variation = None
        if p:
            variation = round((c - p) / p * 100, 1)
        return {"actuel": c, "precedent": p, "variation_pct": variation}

    return ToolResult(ok=True, data={
        "periode_jours": days,
        "fenetre_actuelle": f"{cur_start.isoformat()} → {today.isoformat()}",
        "fenetre_precedente": f"{prev_start.isoformat()} → {prev_end.isoformat()}",
        "chiffre_affaires_eur": _delta("total_revenue"),
        "profit_eur": _delta("total_profit"),
        "nombre_commandes": _delta("total_orders_count"),
        "_note": (
            "variation_pct positif = hausse, négatif = baisse. Présente "
            "l'évolution clairement (ex: « +12,3 % vs la période précédente »)."
        ),
    })


@register(ToolSpec(
    name="get_pending_restocks",
    description=(
        "Liste les réapprovisionnements en attente ou en cours (livraisons "
        "fournisseurs à recevoir). Sert pour 'livraisons en attente', "
        "'réappro en cours'."
    ),
    params=[
        ToolParam("limit", "integer", "Nombre max (1-50). Par défaut 20.", required=False),
    ],
))
def _get_pending_restocks(ctx: ToolContext, args: dict) -> ToolResult:
    limit = max(1, min(50, int(args.get("limit", 20))))
    # On récupère tous puis on filtre côté Python car le filtre status n'est pas
    # toujours exposé sur l'endpoint. On garde Pending + Ordered + Shipped.
    data = _api_get(ctx, "/restocks/with-supplier", params={"limit": limit * 2})
    pending_statuses = {"Pending", "Ordered", "Shipped", "InTransit"}
    if isinstance(data, list):
        filtered = [r for r in data if (r.get("status") in pending_statuses)]
        return ToolResult(ok=True, data={"pending_count": len(filtered),
                                          "restocks": filtered[:limit]})
    return ToolResult(ok=True, data=data)


@register(ToolSpec(
    name="get_top_product_full",
    description=(
        "Récupère le produit le plus vendu (ou le plus rentable) AVEC tous "
        "ses détails (prix d'achat, prix de vente, stock, marge, etc.) en "
        "UNE seule étape. À utiliser pour 'prix du produit le plus vendu', "
        "'détails du meilleur produit', etc."
    ),
    params=[
        ToolParam("metric", "string",
                  "revenue | volume | profit | flop_sales (le moins vendu).",
                  required=False,
                  enum=["revenue", "volume", "profit", "flop_sales"]),
        ToolParam("period_days", "integer",
                  "Période d'analyse en jours (1-365). Par défaut 30.", required=False),
    ],
))
def _get_top_product_full(ctx: ToolContext, args: dict) -> ToolResult:
    metric = (args.get("metric") or "volume").lower()
    sub_args = {"metric": metric, "limit": 1}
    if args.get("period_days"):
        sub_args["period_days"] = args["period_days"]
    top = _get_top_products(ctx, sub_args)
    if not top.ok:
        return top
    products = (top.data or {}).get("products", [])
    if not products:
        return ToolResult(ok=False, error="Aucun produit trouvé pour cette métrique")
    p0 = products[0]
    pid = p0.get("product_id") or p0.get("id")
    if not pid:
        return ToolResult(ok=False, error=f"Pas d'id pour: {p0}")
    detail, kpis = _parallel(
        lambda: _api_get(ctx, f"/products/{pid}"),
        lambda: _api_get(ctx, f"/products/{pid}/kpis/all"),
    )
    return ToolResult(ok=True, data={
        "metric": metric,
        "periode_jours": (top.data or {}).get("periode_jours"),
        "ranking_value": p0.get("value"),
        "ranking_unit": (top.data or {}).get("unit"),
        "product": detail,
        "kpis": kpis,
    })


@register(ToolSpec(
    name="get_supplier_score",
    description="Récupère le profil et le score de performance d'un fournisseur.",
    params=[
        ToolParam("supplier_id", "integer", "Identifiant du fournisseur.", required=True),
    ],
))
def _get_supplier_score(ctx: ToolContext, args: dict) -> ToolResult:
    sid = int(args["supplier_id"])
    data = _api_get(ctx, f"/suppliers/{sid}/profile")
    return ToolResult(ok=True, data=data)


@register(ToolSpec(
    name="get_forecast",
    description="Récupère la prévision de demande pour un produit.",
    params=[
        ToolParam("product_id", "integer", "Identifiant du produit.", required=True),
    ],
))
def _get_forecast(ctx: ToolContext, args: dict) -> ToolResult:
    pid = int(args["product_id"])
    data = _api_get(ctx, f"/ai/forecasts/{pid}")
    return ToolResult(ok=True, data=data)


@register(ToolSpec(
    name="get_classification",
    description="Récupère la classification ABC-XYZ d'un produit.",
    params=[
        ToolParam("product_id", "integer", "Identifiant du produit.", required=True),
    ],
))
def _get_classification(ctx: ToolContext, args: dict) -> ToolResult:
    pid = int(args["product_id"])
    data = _api_get(ctx, f"/ai/classifications/{pid}")
    return ToolResult(ok=True, data=data)


@register(ToolSpec(
    name="search_docs",
    description="Recherche dans la documentation interne (concepts, méthodes, guides) avec RAG sémantique. Utile pour expliquer ABC-XYZ, comment fonctionne le forecast, etc.",
    params=[
        ToolParam("query", "string", "La question ou les mots-clés à chercher.", required=True),
        ToolParam("top_k", "integer", "Nombre de chunks à retourner (1-10). Par défaut 5.", required=False),
    ],
))
def _search_docs(ctx: ToolContext, args: dict) -> ToolResult:
    q = str(args.get("query", "")).strip()
    if not q:
        return ToolResult(ok=False, error="query est requis")
    k = max(1, min(10, int(args.get("top_k", 5))))
    hits = retrieve(q, top_k=k)
    payload = [
        {
            "source": h.source_path,
            "heading": h.heading,
            "similarity": round(h.similarity, 4),
            "content": h.content[:1200],
        }
        for h in hits
    ]
    return ToolResult(ok=True, data={"hits": payload, "context_block": format_context(hits)})


@register(ToolSpec(
    name="compare_products",
    description="Compare deux produits côte-à-côte (prix, marges, ventes, classification, prévision).",
    params=[
        ToolParam("product_id_a", "integer", "Premier produit.", required=True),
        ToolParam("product_id_b", "integer", "Second produit.", required=True),
    ],
))
def _compare_products(ctx: ToolContext, args: dict) -> ToolResult:
    pa = int(args["product_id_a"])
    pb = int(args["product_id_b"])

    def fetch(pid: int) -> dict:
        product, kpis, forecast = _parallel(
            lambda: _api_get(ctx, f"/products/{pid}"),
            lambda: _api_get(ctx, f"/products/{pid}/kpis/all"),
            lambda: _api_get(ctx, f"/ai/forecasts/{pid}"),
        )
        return {"id": pid, "product": product, "kpis": kpis, "forecast": forecast}

    a, b = _parallel(lambda: fetch(pa), lambda: fetch(pb))
    return ToolResult(ok=True, data={"a": a, "b": b})


@register(ToolSpec(
    name="get_sales_anomalies",
    description="Liste les anomalies de ventes détectées récemment (pics ou chutes inhabituels).",
    params=[
        ToolParam("limit", "integer", "Nombre maximum d'anomalies (1-50). Par défaut 10.", required=False),
    ],
))
def _get_sales_anomalies(ctx: ToolContext, args: dict) -> ToolResult:
    limit = max(1, min(50, int(args.get("limit", 10))))
    data = _api_get(ctx, "/ai/sales-anomalies", params={"limit": limit})
    return ToolResult(ok=True, data=data)


@register(ToolSpec(
    name="get_price_anomalies",
    description="Liste les anomalies de prix détectées (variations suspectes par rapport à l'historique).",
    params=[
        ToolParam("limit", "integer", "Nombre maximum d'anomalies (1-50). Par défaut 10.", required=False),
    ],
))
def _get_price_anomalies(ctx: ToolContext, args: dict) -> ToolResult:
    limit = max(1, min(50, int(args.get("limit", 10))))
    data = _api_get(ctx, "/ai/price-anomalies", params={"limit": limit})
    return ToolResult(ok=True, data=data)


@register(ToolSpec(
    name="get_urgent_restocks",
    description="Liste les produits à réapprovisionner en urgence (stock bas + forte demande prévue).",
    params=[],
))
def _get_urgent_restocks(ctx: ToolContext, args: dict) -> ToolResult:
    data = _api_get(ctx, "/ai/urgent-restocks")
    return ToolResult(ok=True, data=data)


@register(ToolSpec(
    name="get_price_suggestions",
    description="Liste les suggestions d'optimisation de prix pour augmenter la marge ou les ventes.",
    params=[
        ToolParam("limit", "integer", "Nombre de suggestions (1-50). Par défaut 10.", required=False),
    ],
))
def _get_price_suggestions(ctx: ToolContext, args: dict) -> ToolResult:
    limit = max(1, min(50, int(args.get("limit", 10))))
    data = _api_get(ctx, "/ai/price-suggestions", params={"limit": limit})
    return ToolResult(ok=True, data=data)


@register(ToolSpec(
    name="get_category_analysis",
    description=(
        "Analyse par catégorie : CA, profit, marge, nombre de produits, "
        "rotation, répartition du stock. Sert pour 'quelle catégorie marche "
        "le mieux', 'compare deux catégories', 'top catégories'."
    ),
    params=[],
))
def _get_category_analysis(ctx: ToolContext, args: dict) -> ToolResult:
    data = _api_get(ctx, "/kpis/category-analysis")
    return ToolResult(ok=True, data=data)


@register(ToolSpec(
    name="get_supplier_ranking",
    description=(
        "Classement de TOUS les fournisseurs : CA, profit, délai de livraison "
        "moyen, taux de fiabilité, taux d'annulation. Sert pour 'quel "
        "fournisseur est le meilleur / le pire', 'qui a les pires délais', "
        "'fournisseur le plus fiable'."
    ),
    params=[],
))
def _get_supplier_ranking(ctx: ToolContext, args: dict) -> ToolResult:
    data = _api_get(ctx, "/kpis/supplier-analysis")
    return ToolResult(ok=True, data=data)


@register(ToolSpec(
    name="get_daily_action_list",
    description=(
        "Liste priorisée des actions à mener aujourd'hui : alertes critiques, "
        "produits à réapprovisionner en urgence, anomalies. À utiliser pour "
        "'qu'est-ce que je dois faire aujourd'hui', 'mes priorités', "
        "'par quoi je commence'."
    ),
    params=[],
))
def _get_daily_action_list(ctx: ToolContext, args: dict) -> ToolResult:
    actions: list[dict] = []

    r_alerts, r_restocks, r_anomalies = _parallel(
        lambda: execute_tool("get_alerts", {"severity": "CRITICAL", "limit": 50}, ctx),
        lambda: execute_tool("get_urgent_restocks", {}, ctx),
        lambda: execute_tool("get_sales_anomalies", {"limit": 20}, ctx),
    )

    if r_alerts and r_alerts.ok:
        items = r_alerts.data if isinstance(r_alerts.data, list) else (r_alerts.data or {}).get("alerts", [])
        if isinstance(items, list) and items:
            actions.append({
                "priorite": 1,
                "categorie": "Alertes critiques",
                "nombre": len(items),
                "action": "Traiter les alertes critiques en priorité",
                "exemples": [
                    (a.get("message") or a.get("product_name") or str(a))[:80]
                    for a in items[:3]
                ],
            })

    if r_restocks and r_restocks.ok:
        items = r_restocks.data if isinstance(r_restocks.data, list) else (r_restocks.data or {}).get("restocks", [])
        if isinstance(items, list) and items:
            actions.append({
                "priorite": 2,
                "categorie": "Réapprovisionnements urgents",
                "nombre": len(items),
                "action": "Lancer les commandes de réapprovisionnement",
                "exemples": [
                    (a.get("product_name") or a.get("name") or str(a))[:80]
                    for a in items[:3]
                ],
            })

    if r_anomalies and r_anomalies.ok:
        items = r_anomalies.data if isinstance(r_anomalies.data, list) else (r_anomalies.data or {}).get("anomalies", [])
        if isinstance(items, list) and items:
            actions.append({
                "priorite": 3,
                "categorie": "Anomalies de ventes",
                "nombre": len(items),
                "action": "Examiner les variations de ventes inhabituelles",
            })

    return ToolResult(ok=True, data={
        "actions": actions,
        "rien_a_signaler": not actions,
        "_note": "Présente cette liste de façon claire et priorisée en français.",
    })


@register(ToolSpec(
    name="get_dormant_stock",
    description=(
        "Produits qui se vendent très peu ou pas du tout sur une période "
        "donnée (stock dormant). Sert pour 'produits qui ne bougent pas', "
        "'qu'est-ce qui dort en stock', 'produits invendus depuis N mois'."
    ),
    params=[
        ToolParam("months", "integer",
                  "Fenêtre d'analyse en mois (1-12). Par défaut 3.", required=False),
    ],
))
def _get_dormant_stock(ctx: ToolContext, args: dict) -> ToolResult:
    try:
        months = max(1, min(12, int(args.get("months", 3))))
    except (TypeError, ValueError):
        months = 3
    sub = _get_top_products(ctx, {"metric": "flop_sales", "limit": 20,
                                  "period_days": months * 30})
    products = (sub.data or {}).get("products", []) if sub.ok else []
    dormant = [p for p in products if (p.get("value") or 0) <= 5]
    return ToolResult(ok=True, data={
        "fenetre_mois": months,
        "produits_dormants": dormant or products[:10],
        "_note": (
            f"Produits avec très peu de ventes sur {months} mois. "
            "value = quantité vendue sur la période."
        ),
    })


@register(ToolSpec(
    name="get_negative_margin_products",
    description=(
        "Produits les moins profitables — potentiellement à perte. Sert pour "
        "'qu'est-ce qui me fait perdre de l'argent', 'produits non rentables', "
        "'produits à marge négative'."
    ),
    params=[],
))
def _get_negative_margin_products(ctx: ToolContext, args: dict) -> ToolResult:
    sub = _get_top_products(ctx, {"metric": "flop_profit", "limit": 20})
    products = (sub.data or {}).get("products", []) if sub.ok else []
    a_perte = [p for p in products if (p.get("value") or 0) < 0]
    return ToolResult(ok=True, data={
        "produits_a_perte": a_perte,
        "produits_les_moins_profitables": products[:10],
        "_note": (
            "value = profit en euros sur la période. Négatif = le produit "
            "fait perdre de l'argent. Si produits_a_perte est vide, aucun "
            "produit n'est strictement à perte — cite alors les moins "
            "profitables."
        ),
    })


# ======================================================================
# WRITE TOOLS — require user confirmation
# ======================================================================

@register(ToolSpec(
    name="trigger_ai_run",
    description="Déclenche un cycle complet de tous les modèles IA (prévisions, classifications, anomalies). Action coûteuse — nécessite confirmation.",
    params=[],
    requires_confirmation=True,
))
def _trigger_ai_run(ctx: ToolContext, args: dict) -> ToolResult:
    # Trigger is on the ai-service itself, not the Rust API.
    ai_url = f"http://localhost:{os.getenv('AI_SERVICE_PORT', '8001')}/ai/run"
    r = requests.post(ai_url, timeout=HTTP_TIMEOUT)
    r.raise_for_status()
    return ToolResult(ok=True, data=r.json())


@register(ToolSpec(
    name="create_restock",
    description=(
        "Crée une commande de réapprovisionnement pour un produit. "
        "ACTION — nécessite confirmation de l'utilisateur."
    ),
    params=[
        ToolParam("product_id", "integer", "Identifiant du produit.", required=True),
        ToolParam("quantity", "integer", "Quantité à commander.", required=True),
        ToolParam("unit_price", "number", "Prix unitaire d'achat. Si absent, "
                  "le prix d'achat actuel du produit est utilisé.", required=False),
        ToolParam("supplier_id", "integer", "Fournisseur. Si absent, celui du "
                  "produit.", required=False),
    ],
    requires_confirmation=True,
))
def _create_restock(ctx: ToolContext, args: dict) -> ToolResult:
    pid = int(args["product_id"])
    qty = int(args["quantity"])
    if qty <= 0:
        return ToolResult(ok=False, error="quantity doit être > 0")
    # Complète prix/fournisseur depuis le produit si non fournis.
    unit_price = args.get("unit_price")
    supplier_id = args.get("supplier_id")
    if unit_price is None or supplier_id is None:
        prod = _api_get(ctx, f"/products/{pid}")
        if unit_price is None:
            unit_price = (prod or {}).get("buying_price") or 0
        if supplier_id is None:
            supplier_id = (prod or {}).get("supplier_id")
    payload = {
        "supplier_id": supplier_id,
        "lines": [{"product_id": pid, "quantity": qty,
                   "unit_price": float(unit_price)}],
        "status": "Pending",
    }
    data = _api_post(ctx, "/restocks", payload)
    return ToolResult(ok=True, data={"restock_cree": data})


@register(ToolSpec(
    name="resolve_alert",
    description=(
        "Change le statut d'une alerte (la marquer comme traitée, résolue, "
        "etc.). ACTION — nécessite confirmation."
    ),
    params=[
        ToolParam("alert_id", "integer", "Identifiant de l'alerte.", required=True),
        ToolParam("status", "string",
                  "Nouveau statut : acknowledged | in_progress | resolved | dismissed.",
                  required=True,
                  enum=["acknowledged", "in_progress", "resolved", "dismissed"]),
    ],
    requires_confirmation=True,
))
def _resolve_alert(ctx: ToolContext, args: dict) -> ToolResult:
    aid = int(args["alert_id"])
    status = str(args.get("status", "resolved"))
    import json as _json
    url = _tenant_url(ctx, f"/alerts/{aid}/status")
    r = requests.put(url, headers=ctx.auth_headers, json={"status": status},
                     timeout=HTTP_TIMEOUT)
    r.raise_for_status()
    body = r.json() if r.text else {"ok": True}
    return ToolResult(ok=True, data=body.get("data", body) if isinstance(body, dict) else body)


@register(ToolSpec(
    name="update_product",
    description=(
        "Modifie un produit : prix d'achat, quantité de stock, ou statut. "
        "ACTION — nécessite confirmation. Pour 'corrige le stock de X', "
        "'change le prix d'achat de X', 'marque X comme arrêté'."
    ),
    params=[
        ToolParam("product_id", "integer", "Identifiant du produit.", required=True),
        ToolParam("buying_price", "number", "Nouveau prix d'achat.", required=False),
        ToolParam("stock_quantity", "integer", "Nouvelle quantité de stock.", required=False),
        ToolParam("status", "string",
                  "Nouveau statut : in_stock | out_of_stock | discontinued | ordered.",
                  required=False,
                  enum=["in_stock", "out_of_stock", "discontinued", "ordered"]),
    ],
    requires_confirmation=True,
))
def _update_product(ctx: ToolContext, args: dict) -> ToolResult:
    pid = int(args["product_id"])
    payload: dict = {}
    if args.get("buying_price") is not None:
        payload["buying_price"] = float(args["buying_price"])
    if args.get("stock_quantity") is not None:
        payload["stock_quantity"] = int(args["stock_quantity"])
    if args.get("status"):
        payload["status"] = str(args["status"])
    if not payload:
        return ToolResult(ok=False, error="Aucun champ à modifier "
                          "(buying_price, stock_quantity ou status requis)")
    url = _tenant_url(ctx, f"/products/{pid}")
    r = requests.put(url, headers=ctx.auth_headers, json=payload, timeout=HTTP_TIMEOUT)
    r.raise_for_status()
    body = r.json() if r.text else {"ok": True}
    return ToolResult(ok=True, data=body.get("data", body) if isinstance(body, dict) else body)
