#!/usr/bin/env python3
"""Panel d'évaluation complet du chatbot.

Couvre les questions critiques fournies par l'utilisateur :

   1. Quel est le produit le plus vendu ?
   2. Quel est le prix unitaire du produit le plus vendu ?
   3. Combien a-t-on vendu les derniers 30 jours ?
   4. Combien reste-t-il en stock ?
   5. Est-ce que le stock est bien géré ? Y a-t-il des réappros à faire ?
   6. Top 3 des produits
   7. Pire 3 produits vendus
   8. Prédiction des ventes d'un produit
   9. Est-ce que le prix du produit X convient ?
  10. Quels sont les livraisons et réappros en cours/en attente ?
  11. Quels sont les produits presque en rupture ?
  12. Quels sont les produits dont le stock est beaucoup trop élevé ?

Plus quelques variantes pour valider la robustesse.

Usage :
    python scripts/chat_eval_panel.py
    python scripts/chat_eval_panel.py --provider mistral
    python scripts/chat_eval_panel.py --verbose
"""

from __future__ import annotations

import argparse
import json
import os
import re
import sys
import time
import unicodedata
import urllib.error
import urllib.request
from dataclasses import dataclass, field
from typing import Any, Callable

if hasattr(sys.stdout, "reconfigure"):
    try:
        sys.stdout.reconfigure(encoding="utf-8")
    except Exception:
        pass


API = "http://localhost:8090"
EMAIL = "admin@example.com"
PASSWORD = "adminpass"
DB_URL = os.environ["DATABASE_URL"]


# ──────────────────────────────────────────────────────────────────────────
# Vérité depuis la base
# ──────────────────────────────────────────────────────────────────────────

def get_truth() -> dict[str, Any]:
    import psycopg2
    truth: dict[str, Any] = {}
    with psycopg2.connect(DB_URL) as conn:
        with conn.cursor() as cur:
            # Top produit par volume (toute la période disponible — comme l'API)
            cur.execute("""
                SELECT p.id_pro, p.name_pro, SUM(lo.quantity_lor) AS volume,
                       p.stock_quantity_pro
                FROM line_order_lor lo
                JOIN order_ord o ON lo.order_id_lor = o.id_ord
                JOIN products_pro p ON lo.product_id_lor = p.id_pro
                GROUP BY p.id_pro, p.name_pro, p.stock_quantity_pro
                ORDER BY volume DESC LIMIT 3
            """)
            rows = cur.fetchall()
            truth["top_by_volume"] = [
                {"id": r[0], "name": r[1], "volume": int(r[2]), "stock": int(r[3])}
                for r in rows
            ]

            # Top produit par revenu
            cur.execute("""
                SELECT p.id_pro, p.name_pro, SUM(lo.line_total_lor) AS revenue
                FROM line_order_lor lo
                JOIN order_ord o ON lo.order_id_lor = o.id_ord
                JOIN products_pro p ON lo.product_id_lor = p.id_pro
                GROUP BY p.id_pro, p.name_pro
                ORDER BY revenue DESC LIMIT 3
            """)
            truth["top_by_revenue"] = [
                {"id": r[0], "name": r[1], "revenue": float(r[2] or 0)}
                for r in cur.fetchall()
            ]

            # Prix unitaire du top par volume
            top_vol_id = truth["top_by_volume"][0]["id"]
            cur.execute(
                "SELECT price_prp FROM productprices_prp "
                "WHERE product_ref_prp = %s ORDER BY created_at DESC LIMIT 1",
                (top_vol_id,),
            )
            r = cur.fetchone()
            truth["top_volume_unit_price"] = float(r[0]) if r else None

            # Ventes des 30 derniers jours
            cur.execute("""
                SELECT COALESCE(SUM(lo.line_total_lor), 0)::float,
                       COUNT(DISTINCT o.id_ord)
                FROM line_order_lor lo
                JOIN order_ord o ON lo.order_id_lor = o.id_ord
                WHERE o.order_date_ord >= NOW() - INTERVAL '30 days'
            """)
            r = cur.fetchone()
            truth["sales_30d_revenue"] = float(r[0])
            truth["sales_30d_orders"] = int(r[1])

            # Stock total
            cur.execute(
                "SELECT COUNT(*), SUM(stock_quantity_pro) FROM products_pro"
            )
            r = cur.fetchone()
            truth["total_products"] = int(r[0])
            truth["total_stock_units"] = int(r[1] or 0)

            # Rupture / stock bas / surstock
            cur.execute("SELECT COUNT(*) FROM products_pro WHERE stock_quantity_pro = 0")
            truth["out_of_stock_count"] = int(cur.fetchone()[0])
            cur.execute("SELECT COUNT(*) FROM products_pro WHERE stock_quantity_pro BETWEEN 1 AND 10")
            truth["low_stock_count"] = int(cur.fetchone()[0])
            cur.execute("SELECT COUNT(*) FROM products_pro WHERE stock_quantity_pro > 100")
            truth["overstock_count_approx"] = int(cur.fetchone()[0])

            # Échantillon produit pour les questions "X"
            cur.execute(
                "SELECT id_pro, name_pro FROM products_pro WHERE id_pro = 8"
            )
            r = cur.fetchone()
            truth["sample_product"] = {"id": r[0], "name": r[1]}
    return truth


# ──────────────────────────────────────────────────────────────────────────
# Test cases
# ──────────────────────────────────────────────────────────────────────────

@dataclass
class TestCase:
    name: str
    question: str
    expected_intents: tuple[str, ...]
    expected_tools: tuple[str, ...] = ()
    expected_substrings_any: tuple[str, ...] = ()
    expected_substrings_all: tuple[str, ...] = ()
    forbidden_substrings: tuple[str, ...] = ()
    custom_check: Callable | None = None
    # Tolère qu'aucun outil ne soit appelé si la question est ambigüe et
    # qu'on attend que le bot demande des précisions.
    allow_no_tool: bool = False


def build_panel(truth: dict) -> list[TestCase]:
    top_vol_name = truth["top_by_volume"][0]["name"].lower()
    top_vol_price = truth["top_volume_unit_price"]
    sample_name = truth["sample_product"]["name"]

    return [
        # ───── PANEL UTILISATEUR ─────────────────────────────────────────
        TestCase(
            name="01. Produit le plus vendu",
            question="quel est le produit le plus vendu ?",
            expected_intents=("data",),
            expected_tools=("get_top_products", "get_top_product_full"),
            expected_substrings_any=(top_vol_name,),
            forbidden_substrings=("je vais chercher", "rechercher pour vous"),
        ),
        TestCase(
            name="02. Prix unitaire du produit le plus vendu",
            question="quel est le prix unitaire du produit le plus vendu ?",
            expected_intents=("data",),
            expected_tools=("get_top_product_full",),
            # La réponse doit contenir le nom ET un prix (€ ou euro)
            expected_substrings_all=(top_vol_name,),
            expected_substrings_any=("€", "euro"),
            forbidden_substrings=("je vais chercher", "40 120 unités",
                                   "40,120 unités"),
        ),
        TestCase(
            name="03. CA des 30 derniers jours",
            question="combien a-t-on vendu les 30 derniers jours ?",
            expected_intents=("data",),
            expected_tools=("get_total_sales", "get_global_kpis"),
            expected_substrings_any=("€", "euro", "chiffre d'affaires"),
            forbidden_substrings=("je vais chercher",),
        ),
        TestCase(
            name="04. Stock restant total",
            question="combien reste-t-il en stock ?",
            expected_intents=("data",),
            expected_tools=("get_stock_summary", "get_global_kpis", "get_low_stock"),
            forbidden_substrings=("je vais chercher",),
        ),
        TestCase(
            name="05. Stock bien géré + réappros à faire",
            question="est-ce que le stock est bien géré ? y a-t-il des réapprovisionnements à faire ?",
            expected_intents=("data",),
            expected_tools=("get_stock_summary", "get_urgent_restocks",
                            "get_low_stock", "get_global_kpis"),
            forbidden_substrings=("je vais chercher",),
        ),
        TestCase(
            name="06. Top 3 des produits",
            question="Top 3 des produits",
            expected_intents=("data",),
            expected_tools=("get_top_products",),
            expected_substrings_any=("produit", "top", "1"),
        ),
        TestCase(
            name="07. Pire 3 produits vendus",
            question="Pire 3 produits vendus",
            expected_intents=("data",),
            expected_tools=("get_top_products",),
            # Vérifie côté contenu que ce sont bien les MOINS vendus (pas les top)
            # → la réponse doit contenir des indicateurs de classement bas
            expected_substrings_any=("moins vendu", "moins venduS", "pire",
                                      "flop", "les moins"),
            forbidden_substrings=("je vais chercher",),
        ),
        TestCase(
            name="08. Prédiction ventes (sans produit précisé)",
            question="prédiction des ventes d'un produit",
            expected_intents=("data",),
            # Question ambigue : le bot peut soit demander quel produit, soit
            # lister les prévisions disponibles. On accepte les deux.
            allow_no_tool=True,
            expected_substrings_any=("quel produit", "préciser", "identifier",
                                      "prévision", "forecast", "demande"),
        ),
        TestCase(
            name="09. Prix du produit X convient ?",
            question=f"est-ce que le prix du produit {sample_name} convient ?",
            expected_intents=("data",),
            expected_tools=("get_product_by_name", "get_price_suggestions",
                            "get_product_detail"),
            expected_substrings_any=(sample_name.lower(),),
            forbidden_substrings=("je vais chercher",),
        ),
        TestCase(
            name="10. Livraisons / réappros en attente",
            question="quels sont les livraisons et réapprovisionnements en cours ou en attente ?",
            expected_intents=("data",),
            expected_tools=("get_pending_restocks", "get_urgent_restocks"),
            forbidden_substrings=("je vais chercher",),
        ),
        TestCase(
            name="11. Produits presque en rupture",
            question="quels sont les produits presque en rupture ?",
            expected_intents=("data",),
            expected_tools=("get_soon_out_of_stock", "get_low_stock"),
            forbidden_substrings=("je vais chercher",),
        ),
        TestCase(
            name="12. Produits avec stock trop élevé",
            question="quels sont les produits dont le stock est beaucoup trop élevé ?",
            expected_intents=("data",),
            expected_tools=("get_overstock",),
            forbidden_substrings=("je vais chercher",),
        ),

        # ───── VARIANTES bonus ───────────────────────────────────────────
        TestCase(
            name="13. Chitchat (sanity check)",
            question="Bonjour !",
            expected_intents=("chitchat",),
            expected_substrings_any=("bonjour", "aider"),
            forbidden_substrings=("rupture",),
        ),
        TestCase(
            name="14. Doc — ABC-XYZ",
            question="que veux dire ABC-XYZ",
            expected_intents=("doc",),
            expected_substrings_any=("classification", "chiffre d'affaires",
                                      "demande", "variabilité"),
            allow_no_tool=True,
        ),
        TestCase(
            name="15. Variante 'combien on a vendu cette semaine'",
            question="combien on a vendu cette semaine ?",
            expected_intents=("data",),
            expected_tools=("get_total_sales",),
        ),
    ]


# ──────────────────────────────────────────────────────────────────────────
# Helpers
# ──────────────────────────────────────────────────────────────────────────

def _http(method: str, url: str, *, token: str | None = None, body: dict | None = None) -> dict:
    headers = {"Content-Type": "application/json"}
    if token:
        headers["Authorization"] = f"Bearer {token}"
    data = json.dumps(body).encode() if body is not None else None
    req = urllib.request.Request(url, data=data, headers=headers, method=method)
    with urllib.request.urlopen(req, timeout=900) as resp:
        return json.loads(resp.read())


def _strip_accents(s: str) -> str:
    return ''.join(c for c in unicodedata.normalize('NFD', s)
                   if unicodedata.category(c) != 'Mn')


def _contains(haystack: str, needle: str) -> bool:
    return _strip_accents(needle.lower()) in _strip_accents(haystack.lower())


@dataclass
class TestResult:
    name: str
    passed: bool
    failures: list[str] = field(default_factory=list)
    latency_ms: int = 0
    intent: str = ""
    provider: str = ""
    shortcut: str | None = None
    tool_calls_fired: list[str] = field(default_factory=list)
    response_preview: str = ""
    cached: bool = False


def score(tc: TestCase, response: dict) -> TestResult:
    data = response.get("data", {}) if response else {}
    msg = data.get("assistant_message", {})
    content = msg.get("content", "")

    failures: list[str] = []

    intent = data.get("intent", "")
    if intent not in tc.expected_intents:
        failures.append(f"intent={intent!r} (attendu {tc.expected_intents})")

    shortcut = data.get("shortcut_used")
    tools_in_calls = [c.get("tool") for c in (data.get("tool_calls") or [])]
    tools_fired = ([shortcut] if shortcut else []) + tools_in_calls

    if tc.expected_tools:
        if not any(t in tools_fired for t in tc.expected_tools):
            if tc.allow_no_tool and not tools_fired:
                pass  # acceptable (question ambigue)
            else:
                failures.append(f"outils attendus {tc.expected_tools}, appelés: {tools_fired or 'aucun'}")

    if tc.expected_substrings_any:
        if not any(_contains(content, s) for s in tc.expected_substrings_any):
            failures.append(f"aucune phrase attendue trouvée: {tc.expected_substrings_any}")

    for s in tc.expected_substrings_all:
        if not _contains(content, s):
            failures.append(f"manque '{s}'")

    for s in tc.forbidden_substrings:
        if _contains(content, s):
            failures.append(f"phrase interdite présente: '{s}'")

    if tc.custom_check:
        ok, msg_custom = tc.custom_check(data)
        if not ok:
            failures.append(f"custom: {msg_custom}")

    return TestResult(
        name=tc.name,
        passed=not failures,
        failures=failures,
        latency_ms=int((data.get("usage") or {}).get("latency_ms", 0)),
        intent=intent,
        provider=data.get("provider_used", ""),
        shortcut=shortcut,
        tool_calls_fired=[t for t in tools_fired if t],
        response_preview=content[:200].replace("\n", " "),
        cached=bool(data.get("cached")),
    )


# ──────────────────────────────────────────────────────────────────────────
# Main
# ──────────────────────────────────────────────────────────────────────────

def main():
    p = argparse.ArgumentParser()
    p.add_argument("--api", default=API)
    p.add_argument("--provider", default="auto")
    p.add_argument("--verbose", action="store_true")
    p.add_argument("--only", type=str, default=None,
                   help="Filtre les tests par sous-string de leur nom")
    args = p.parse_args()

    print("─" * 80)
    print("  Chat StockS — Évaluation panel utilisateur")
    print("─" * 80)
    print("\n[1/4] Vérité depuis Postgres…")
    truth = get_truth()
    print(f"  ✓ Top vol: {truth['top_by_volume'][0]['name']} ({truth['top_by_volume'][0]['volume']} u)")
    print(f"  ✓ Top rev: {truth['top_by_revenue'][0]['name']} ({truth['top_by_revenue'][0]['revenue']:.0f}€)")
    print(f"  ✓ Prix unitaire top vol: {truth['top_volume_unit_price']} €")
    print(f"  ✓ CA 30j: {truth['sales_30d_revenue']:.0f}€ ({truth['sales_30d_orders']} commandes)")
    print(f"  ✓ Stock total: {truth['total_stock_units']} unités ({truth['total_products']} produits)")
    print(f"  ✓ Rupture/Bas/~Surstock: {truth['out_of_stock_count']}/{truth['low_stock_count']}/{truth['overstock_count_approx']}")

    print("\n[2/4] Login…")
    auth = _http("POST", f"{args.api}/auth/login",
                 body={"email": EMAIL, "password": PASSWORD})
    token = auth["data"]["token"]
    print(f"  ✓ Token OK")

    print(f"\n[3/4] Session (provider={args.provider})…")
    sess = _http("POST", f"{args.api}/chat/sessions", token=token,
                 body={"title": "panel", "provider": args.provider})
    sid = sess["data"]["session_id"]
    print(f"  ✓ {sid[:8]}…")

    cases = build_panel(truth)
    if args.only:
        cases = [c for c in cases if args.only.lower() in c.name.lower()]

    print(f"\n[4/4] Exécution de {len(cases)} cas (peut prendre 10-30 min en local)…\n")
    results: list[TestResult] = []
    for i, tc in enumerate(cases, 1):
        print(f"  [{i:>2}/{len(cases)}] {tc.name} … ", end="", flush=True)
        t0 = time.time()
        try:
            resp = _http("POST", f"{args.api}/chat/sessions/{sid}/messages",
                         token=token, body={"content": tc.question})
        except urllib.error.HTTPError as e:
            print(f"HTTP {e.code}")
            results.append(TestResult(name=tc.name, passed=False,
                                       failures=[f"HTTP {e.code}"]))
            continue
        except Exception as e:
            print(f"ERREUR ({e})")
            results.append(TestResult(name=tc.name, passed=False, failures=[str(e)]))
            continue

        elapsed = int((time.time() - t0) * 1000)
        res = score(tc, resp)
        if not res.latency_ms:
            res.latency_ms = elapsed

        status = "✓" if res.passed else "✗"
        info = f"{res.latency_ms/1000:.1f}s {res.provider}"
        if res.cached:
            info += " CACHED"
        if res.shortcut:
            info += f" shortcut={res.shortcut}"
        print(f"{status}  ({info})")
        if args.verbose:
            print(f"       → {res.response_preview}")
        if not res.passed and not args.verbose:
            for f in res.failures:
                print(f"       ⚠ {f}")
        results.append(res)

    # ─── Rapport ───
    print("\n" + "─" * 80)
    print("  RÉSULTATS")
    print("─" * 80)
    passed = sum(1 for r in results if r.passed)
    total = len(results)
    pct = 100 * passed / total if total else 0
    print(f"  Score : {passed}/{total} ({pct:.0f}%)")
    if results:
        sorted_lat = sorted([r.latency_ms for r in results])
        print(f"  Latence moyenne : {sum(r.latency_ms for r in results) / total / 1000:.1f}s")
        print(f"  Latence p95     : {sorted_lat[min(int(0.95*total), total-1)] / 1000:.1f}s")

    if passed < total:
        print(f"\n  ÉCHECS :")
        for r in results:
            if r.passed:
                continue
            print(f"\n  ✗ {r.name}")
            print(f"      provider={r.provider}  intent={r.intent}  shortcut={r.shortcut}  tools={r.tool_calls_fired or '∅'}")
            print(f"      → {r.response_preview}")
            for f in r.failures:
                print(f"      ⚠ {f}")

    print("\n  RÉCAPITULATIF :")
    for i, r in enumerate(results, 1):
        marker = "✓" if r.passed else "✗"
        print(f"  {marker}  {i:>2}. {r.name[:55]:<55} {r.latency_ms/1000:>5.0f}s  {(r.shortcut or '-')[:25]}")


if __name__ == "__main__":
    main()
