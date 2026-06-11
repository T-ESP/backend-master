#!/usr/bin/env python3
"""Harness d'évaluation du chatbot StockS.

Principe :
  1. Récupère la "vérité" depuis la base (top produit, ruptures, etc.)
  2. Pose au bot des questions dont on connaît la bonne réponse
  3. Vérifie pour chaque question :
       - intent correctement classifié ?
       - outil correct appelé (ou raccourci utilisé) ?
       - les faits attendus apparaissent-ils dans la réponse ?
       - phrases interdites (ex: "je vais chercher" sans rien appeler) ?
  4. Imprime un tableau de scores + détails des échecs.

Usage :
  python scripts/chat_eval.py
  python scripts/chat_eval.py --provider mistral   # nécessite MISTRAL_API_KEY
  python scripts/chat_eval.py --verbose            # affiche les réponses complètes
"""

from __future__ import annotations

import argparse
import json
import re
import sys
import time
import urllib.error
import urllib.request
from dataclasses import dataclass, field
from typing import Any, Callable

# UTF-8 sur Windows
if hasattr(sys.stdout, "reconfigure"):
    try:
        sys.stdout.reconfigure(encoding="utf-8")
    except Exception:
        pass


# ──────────────────────────────────────────────────────────────────────────
# Configuration
# ──────────────────────────────────────────────────────────────────────────

API = "http://localhost:8090"
EMAIL = "admin@example.com"
PASSWORD = "adminpass"
DB_URL = "postgres://user:pass@localhost:5432/stocks"


# ──────────────────────────────────────────────────────────────────────────
# Vérité depuis la base
# ──────────────────────────────────────────────────────────────────────────

def get_truth() -> dict[str, Any]:
    """Récupère les valeurs de référence directement depuis Postgres."""
    import psycopg2
    truth: dict[str, Any] = {}
    with psycopg2.connect(DB_URL) as conn:
        with conn.cursor() as cur:
            # Top produit par revenu (sur les 30 derniers jours)
            cur.execute("""
                SELECT p.id_pro, p.name_pro, p.category_pro,
                       SUM(lo.line_total_lor) AS revenue
                FROM line_order_lor lo
                JOIN order_ord o ON lo.order_id_lor = o.id_ord
                JOIN products_pro p ON lo.product_id_lor = p.id_pro
                WHERE o.order_date_ord >= NOW() - INTERVAL '30 days'
                GROUP BY p.id_pro, p.name_pro, p.category_pro
                ORDER BY revenue DESC NULLS LAST
                LIMIT 5
            """)
            rows = cur.fetchall()
            truth["top_by_revenue"] = [
                {"id": r[0], "name": r[1], "category": r[2], "revenue": float(r[3] or 0)}
                for r in rows
            ]

            # Top produit par volume
            cur.execute("""
                SELECT p.id_pro, p.name_pro, SUM(lo.quantity_lor) AS volume
                FROM line_order_lor lo
                JOIN order_ord o ON lo.order_id_lor = o.id_ord
                JOIN products_pro p ON lo.product_id_lor = p.id_pro
                WHERE o.order_date_ord >= NOW() - INTERVAL '30 days'
                GROUP BY p.id_pro, p.name_pro
                ORDER BY volume DESC
                LIMIT 5
            """)
            rows = cur.fetchall()
            truth["top_by_volume"] = [
                {"id": r[0], "name": r[1], "volume": int(r[2])}
                for r in rows
            ]

            # Nombre de produits en rupture (stock = 0)
            cur.execute("SELECT COUNT(*) FROM products_pro WHERE stock_quantity_pro = 0")
            truth["out_of_stock_count"] = int(cur.fetchone()[0])

            # Nombre de produits en stock bas (1-10 unités)
            cur.execute("SELECT COUNT(*) FROM products_pro WHERE stock_quantity_pro BETWEEN 1 AND 10")
            truth["low_stock_count"] = int(cur.fetchone()[0])

            # Nombre total de produits
            cur.execute("SELECT COUNT(*) FROM products_pro")
            truth["total_products"] = int(cur.fetchone()[0])

            # Nombre d'alertes critiques actives
            cur.execute("SELECT COUNT(*) FROM notifications WHERE severity = 'CRITICAL' AND status::text = 'new'")
            truth["critical_alerts"] = int(cur.fetchone()[0])

            # Détails d'un produit précis (le premier)
            cur.execute("""
                SELECT p.id_pro, p.name_pro, p.category_pro, p.stock_quantity_pro,
                       p.buying_price_pro,
                       (SELECT price_prp FROM productprices_prp
                        WHERE product_ref_prp = p.id_pro
                        ORDER BY created_at DESC LIMIT 1) AS sell_price
                FROM products_pro p
                ORDER BY p.id_pro LIMIT 1
            """)
            r = cur.fetchone()
            truth["sample_product"] = {
                "id": r[0], "name": r[1], "category": r[2],
                "stock": int(r[3]), "buy_price": float(r[4] or 0),
                "sell_price": float(r[5] or 0),
            }

    return truth


# ──────────────────────────────────────────────────────────────────────────
# Cas de test
# ──────────────────────────────────────────────────────────────────────────

@dataclass
class TestCase:
    name: str
    question: str
    # Intent attendu (ou un set d'intents acceptables)
    expected_intents: tuple[str, ...]
    # Au moins un de ces tools/shortcuts doit être appelé (vide = pas requis)
    expected_tools: tuple[str, ...] = ()
    # Doit contenir au moins UN de ces substrings (insensible casse + accents)
    expected_substrings_any: tuple[str, ...] = ()
    # Doit contenir TOUS ces substrings
    expected_substrings_all: tuple[str, ...] = ()
    # Ne doit PAS contenir ces substrings (phrases qui signalent un échec)
    forbidden_substrings: tuple[str, ...] = ()
    # Fonction custom (response_data) -> (bool, message)
    custom_check: Callable | None = None


def build_test_cases(truth: dict) -> list[TestCase]:
    top_rev = truth["top_by_revenue"][0]["name"] if truth["top_by_revenue"] else "?"
    top_vol = truth["top_by_volume"][0]["name"] if truth["top_by_volume"] else "?"
    return [
        TestCase(
            name="1. Chitchat",
            question="Bonjour !",
            expected_intents=("chitchat",),
            expected_substrings_any=("bonjour", "salut", "aider"),
            forbidden_substrings=("rupture", "produit le plus"),
        ),
        TestCase(
            name="2. Question doc — ABC-XYZ apostrophe",
            question="Qu'est-ce que la classification ABC-XYZ ?",
            expected_intents=("doc",),
            expected_substrings_any=("revenu", "variabilité", "classification"),
            forbidden_substrings=("je vais chercher", "désolé je ne sais pas"),
        ),
        TestCase(
            name="3. Question doc — ABC-XYZ sans apostrophe",
            question="que veux dire ABC-XYZ",
            expected_intents=("doc",),
            # Élargi : "chiffre d'affaires", "demande", "classification", "abc"
            # peuvent tous apparaître dans une bonne explication
            expected_substrings_any=("revenu", "variabilité", "pareto",
                                     "chiffre d'affaires", "classification", "demande"),
        ),
        TestCase(
            name="4. Top produit par CA",
            question="Quel est le produit qui rapporte le plus d'argent ?",
            expected_intents=("data",),
            expected_tools=("get_top_products",),
            expected_substrings_any=(top_rev.lower(),),  # nom du vrai top doit apparaître
            forbidden_substrings=("je vais chercher",),
        ),
        TestCase(
            name="5. Top 3 par CA",
            question="Donne-moi les 3 produits qui rapportent le plus.",
            expected_intents=("data",),
            expected_tools=("get_top_products",),
            expected_substrings_any=(top_rev.lower(),),
        ),
        TestCase(
            name="6. Top par volume",
            question="Quel est mon produit le plus vendu (en quantité) ?",
            expected_intents=("data",),
            expected_tools=("get_top_products",),
            expected_substrings_any=(top_vol.lower(),),
        ),
        TestCase(
            name="7. Stock bas (shortcut)",
            question="Quels produits sont en stock bas ?",
            expected_intents=("data",),
            expected_tools=("get_low_stock",),
        ),
        TestCase(
            name="8. Combien en rupture",
            question="Combien de produits sont en rupture critique ?",
            expected_intents=("data",),
            expected_tools=("get_alerts", "get_low_stock"),
            forbidden_substrings=("je vais chercher",),
        ),
        TestCase(
            name="9. Détails produit par ID",
            question=f"Donne-moi les détails du produit {truth['sample_product']['id']}",
            expected_intents=("data",),
            expected_tools=("get_product_detail",),
            expected_substrings_any=(truth["sample_product"]["name"].lower(),),
        ),
        TestCase(
            name="10. KPI globaux",
            question="Donne-moi un résumé de mon activité.",
            expected_intents=("data",),
            expected_tools=("get_global_kpis",),
        ),
        TestCase(
            name="11. Cache hit (rejoue Q2)",
            question="Qu'est-ce que la classification ABC-XYZ ?",
            expected_intents=("doc",),
            custom_check=lambda d: (
                bool(d.get("cached")),
                f"cached={d.get('cached')} (attendu True)"
            ),
        ),
    ]


# ──────────────────────────────────────────────────────────────────────────
# Helpers HTTP + scoring
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
    import unicodedata
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

    # Intent
    intent = data.get("intent", "")
    if intent not in tc.expected_intents:
        failures.append(f"intent={intent!r}, attendu un de {tc.expected_intents}")

    # Outils appelés (ou raccourci)
    shortcut = data.get("shortcut_used")
    tools_in_calls = [tc_call.get("tool") for tc_call in (data.get("tool_calls") or [])]
    tools_fired = ([shortcut] if shortcut else []) + tools_in_calls
    if tc.expected_tools:
        if not any(t in tools_fired for t in tc.expected_tools):
            failures.append(f"aucun outil attendu appelé. Attendu: {tc.expected_tools}, appelés: {tools_fired or 'aucun'}")

    # Substrings any
    if tc.expected_substrings_any:
        if not any(_contains(content, s) for s in tc.expected_substrings_any):
            failures.append(f"aucune des phrases attendues trouvée: {tc.expected_substrings_any}")

    # Substrings all
    for s in tc.expected_substrings_all:
        if not _contains(content, s):
            failures.append(f"manque '{s}' dans la réponse")

    # Substrings interdits
    for s in tc.forbidden_substrings:
        if _contains(content, s):
            failures.append(f"contient phrase interdite: '{s}'")

    # Custom
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
        response_preview=content[:160].replace("\n", " "),
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
    args = p.parse_args()

    print("─" * 80)
    print("  Chat StockS — Évaluation systématique")
    print("─" * 80)

    print("\n[1/4] Récupération de la vérité depuis Postgres…")
    try:
        truth = get_truth()
    except Exception as e:
        print(f"  ⚠ Impossible de lire la DB: {e}")
        print("     Vérifie que la stack tourne et que psycopg2 est dispo.")
        sys.exit(1)
    print(f"  ✓ Top revenu: {truth['top_by_revenue'][0]['name']} ({truth['top_by_revenue'][0]['revenue']:.0f} €)")
    print(f"  ✓ Top volume: {truth['top_by_volume'][0]['name']} ({truth['top_by_volume'][0]['volume']} u)")
    print(f"  ✓ Stock bas: {truth['low_stock_count']} produits")
    print(f"  ✓ Rupture: {truth['out_of_stock_count']} produits")
    print(f"  ✓ Alertes critiques actives: {truth['critical_alerts']}")
    print(f"  ✓ Produit échantillon: {truth['sample_product']['name']} (id={truth['sample_product']['id']})")

    print("\n[2/4] Login…")
    auth = _http("POST", f"{args.api}/auth/login",
                 body={"email": EMAIL, "password": PASSWORD})
    token = auth["data"]["token"]
    print(f"  ✓ Token: {token[:20]}…")

    print(f"\n[3/4] Création session (provider={args.provider})…")
    session = _http("POST", f"{args.api}/chat/sessions", token=token,
                    body={"title": "eval", "provider": args.provider})
    sid = session["data"]["session_id"]
    print(f"  ✓ Session: {sid[:8]}…")

    cases = build_test_cases(truth)
    print(f"\n[4/4] Exécution de {len(cases)} cas de test (peut prendre plusieurs minutes en local)…\n")

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
                                       failures=[f"HTTP {e.code}: {e.read().decode()[:200]}"]))
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
        print(f"{status}  ({res.latency_ms/1000:.1f}s, {res.provider}, intent={res.intent}, "
              f"tools={res.tool_calls_fired or '∅'}{', CACHED' if res.cached else ''})")
        if args.verbose:
            print(f"       réponse: {res.response_preview}")
        if not res.passed and not args.verbose:
            for f in res.failures:
                print(f"       ⚠ {f}")
        results.append(res)

    # ─── Rapport final ─────────────────────────────────────────────
    print("\n" + "─" * 80)
    print("  RÉSULTATS")
    print("─" * 80)
    passed = sum(1 for r in results if r.passed)
    total = len(results)
    pct = 100 * passed / total if total else 0
    print(f"  Score global : {passed}/{total} ({pct:.0f}%)")
    print(f"  Latence moyenne : {sum(r.latency_ms for r in results) / max(1,total) / 1000:.1f}s")
    print(f"  Latence p95 : {sorted([r.latency_ms for r in results])[int(0.95*total)] / 1000:.1f}s")

    if passed < total:
        print(f"\n  ÉCHECS ({total - passed}) :")
        for r in results:
            if r.passed:
                continue
            print(f"\n  ✗ {r.name}")
            print(f"      provider={r.provider}  intent={r.intent}  tools={r.tool_calls_fired or '∅'}")
            print(f"      réponse: {r.response_preview}")
            for f in r.failures:
                print(f"      ⚠ {f}")

    # Tableau récapitulatif
    print("\n  TABLEAU :")
    print(f"  {'#':<3} {'Nom':<45} {'Intent':<10} {'Tool':<22} {'Latence':<8} {'OK?'}")
    for i, r in enumerate(results, 1):
        print(f"  {i:<3} {r.name[:45]:<45} {r.intent[:10]:<10} "
              f"{(','.join(r.tool_calls_fired) or '-')[:22]:<22} "
              f"{r.latency_ms/1000:>5.1f}s   {'✓' if r.passed else '✗'}")


if __name__ == "__main__":
    main()
