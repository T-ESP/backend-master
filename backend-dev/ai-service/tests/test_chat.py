"""Unit tests for the chat module — pieces that don't need DB / network."""

from __future__ import annotations

import json
import sys
from pathlib import Path

# Add ai-service root to path so `from chat...` works when running pytest.
ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))


# ---------------------------------------------------------------------------
# chat.agent.intent
# ---------------------------------------------------------------------------

def test_intent_doc_question_french():
    from chat.agent.intent import classify
    assert classify("Qu'est-ce que la classification ABC-XYZ ?") == "doc"
    assert classify("Comment fonctionne le forecast ?") == "doc"
    assert classify("Explique-moi le clustering") == "doc"


def test_intent_data_question():
    from chat.agent.intent import classify
    assert classify("Combien de produits sont en rupture ?") == "data"
    assert classify("Top 5 fournisseurs ?") == "data"
    # The leading word "Quelles" doesn't match any specific rule, but "ventes"
    # in the data rules should catch it.
    assert classify("Quelles ventes pour janvier ?") == "data"


def test_intent_action():
    from chat.agent.intent import classify
    assert classify("Lance le forecast") == "action"
    assert classify("Crée une alerte pour le produit 42") == "action"
    assert classify("Recalcule les prévisions") == "action"


def test_intent_chitchat():
    from chat.agent.intent import classify
    assert classify("Bonjour !") == "chitchat"
    assert classify("Merci beaucoup") == "chitchat"
    assert classify("hello") == "chitchat"


def test_intent_default_to_data():
    from chat.agent.intent import classify
    # An unmatched stocky question defaults to data.
    assert classify("Stock du produit ?") == "data"


# ---------------------------------------------------------------------------
# chat.rag.chunker
# ---------------------------------------------------------------------------

def test_chunker_keeps_short_section_intact():
    from chat.rag.chunker import chunk_markdown
    text = "## Intro\n\nUn court paragraphe d'introduction.\n"
    chunks = chunk_markdown(text)
    assert len(chunks) == 1
    assert "Intro" in chunks[0].heading
    assert chunks[0].chunk_index == 0


def test_chunker_splits_long_sections():
    from chat.rag.chunker import chunk_markdown
    body = ("Phrase longue. " * 200).strip()
    text = f"## Section\n\n{body}\n"
    chunks = chunk_markdown(text)
    assert len(chunks) > 1
    # All chunks share the same heading
    assert all("Section" in c.heading for c in chunks)
    # Token estimates non-zero
    assert all(c.token_count > 0 for c in chunks)


def test_chunker_heading_path():
    from chat.rag.chunker import chunk_markdown
    text = "# Top\n\n## Sous-titre\n\ncontenu pertinent ici.\n"
    chunks = chunk_markdown(text)
    assert chunks
    # Should have heading hierarchy
    assert "Top" in chunks[0].heading
    assert "Sous-titre" in chunks[0].heading


def test_chunker_handles_no_headings():
    from chat.rag.chunker import chunk_markdown
    text = "Juste du texte sans aucun titre du tout."
    chunks = chunk_markdown(text)
    assert len(chunks) == 1
    assert chunks[0].content


# ---------------------------------------------------------------------------
# chat.types
# ---------------------------------------------------------------------------

def test_tool_spec_to_openai_schema():
    from chat.types import ToolParam, ToolSpec
    spec = ToolSpec(
        name="get_x",
        description="récupère X",
        params=[
            ToolParam("a", "string", "param a", required=True),
            ToolParam("b", "integer", "param b", required=False),
        ],
    )
    schema = spec.to_openai_schema()
    assert schema["type"] == "function"
    fn = schema["function"]
    assert fn["name"] == "get_x"
    assert "a" in fn["parameters"]["properties"]
    assert fn["parameters"]["required"] == ["a"]


def test_message_to_dict_roundtrip():
    from chat.types import Message, ToolCall
    m = Message(
        role="assistant",
        content="ok",
        tool_calls=[ToolCall(id="c1", name="get_x", arguments={"a": 1})],
    )
    d = m.to_dict()
    assert d["role"] == "assistant"
    assert d["tool_calls"][0]["function"]["name"] == "get_x"
    # Args should be JSON-encoded inside the tool_calls payload (OpenAI shape).
    args_str = d["tool_calls"][0]["function"]["arguments"]
    assert json.loads(args_str) == {"a": 1}


# ---------------------------------------------------------------------------
# chat.tools.registry — catalog shape
# ---------------------------------------------------------------------------

def test_registry_has_expected_tools():
    from chat.tools import catalog
    names = {t.name for t in catalog()}
    expected = {
        "get_global_kpis",
        "get_top_products",
        "get_product_detail",
        "get_alerts",
        "get_low_stock",
        "get_supplier_score",
        "get_forecast",
        "get_classification",
        "search_docs",
        "trigger_ai_run",
    }
    assert expected.issubset(names), f"missing: {expected - names}"


def test_registry_separates_read_and_write_tools():
    from chat.tools import tool_specs
    read_only = tool_specs(read_only=True)
    all_tools = tool_specs(read_only=False)
    assert len(read_only) < len(all_tools)
    assert all(not t.requires_confirmation for t in read_only)


# ---------------------------------------------------------------------------
# chat.agent.system_prompt
# ---------------------------------------------------------------------------

def test_system_prompt_includes_persona():
    from chat.agent.system_prompt import build_system_prompt
    p = build_system_prompt()
    assert "StockS" in p
    assert "français" in p.lower()


def test_system_prompt_appends_rag_context():
    from chat.agent.system_prompt import build_system_prompt
    p = build_system_prompt(rag_context="-EXTRACT-")
    assert "-EXTRACT-" in p


def test_system_prompt_english_user():
    from chat.agent.system_prompt import build_system_prompt
    p = build_system_prompt(user_lang="en")
    assert "English" in p


# ---------------------------------------------------------------------------
# chat.llm.factory — provider listing without keys
# ---------------------------------------------------------------------------

def test_factory_lists_all_providers(monkeypatch):
    monkeypatch.delenv("MISTRAL_API_KEY", raising=False)
    monkeypatch.delenv("GROQ_API_KEY", raising=False)
    # Reset module-level singletons so env changes take effect.
    import chat.llm.factory as f
    f._INSTANCES.clear()
    out = f.list_providers()
    names = [p["name"] for p in out]
    assert names == ["groq", "mistral"]
    # With no keys, both providers are unavailable.
    by_name = {p["name"]: p for p in out}
    assert by_name["groq"]["available"] is False
    assert by_name["mistral"]["available"] is False


# ---------------------------------------------------------------------------
# 2026-05-09 improvements: shortcuts, new tools, citations
# ---------------------------------------------------------------------------

def test_shortcut_low_stock():
    from chat.agent.shortcuts import match
    sc = match("Quels produits sont en stock bas ?")
    assert sc is not None
    assert sc.tool == "get_low_stock"
    assert sc.args == {}


def test_shortcut_alerts_critical():
    from chat.agent.shortcuts import match
    sc = match("Liste les alertes critiques")
    assert sc is not None
    assert sc.tool == "get_alerts"
    assert sc.args.get("severity") == "CRITICAL"


def test_shortcut_top_products_with_n():
    from chat.agent.shortcuts import match
    sc = match("Top 5 produits par chiffre d'affaires")
    assert sc is not None
    assert sc.tool == "get_top_products"
    assert sc.args.get("metric") == "revenue"
    assert sc.args.get("limit") == 5


def test_shortcut_product_detail_extracts_id():
    from chat.agent.shortcuts import match
    sc = match("Donne-moi les détails du produit 42")
    assert sc is not None
    assert sc.tool == "get_product_detail"
    assert sc.args == {"product_id": 42}


def test_shortcut_returns_none_for_open_question():
    from chat.agent.shortcuts import match
    assert match("Que penses-tu de mes ventes globalement ?") is None
    assert match("Salut !") is None


def test_new_tools_registered():
    from chat.tools import catalog
    names = {t.name for t in catalog()}
    new_tools = {"compare_products", "get_sales_anomalies",
                 "get_price_anomalies", "get_urgent_restocks",
                 "get_price_suggestions"}
    assert new_tools.issubset(names), f"missing: {new_tools - names}"


def test_compare_products_tool_takes_two_ids():
    from chat.tools import get_tool
    t = get_tool("compare_products")
    assert t is not None
    param_names = {p.name for p in t.spec.params}
    assert {"product_id_a", "product_id_b"}.issubset(param_names)


def test_intent_keeps_doc_for_concept_questions():
    # Sanity check that improvements didn't break intent classification
    from chat.agent.intent import classify
    assert classify("Que signifie ABC-XYZ ?") == "doc"
    assert classify("Liste les alertes critiques") == "data"


def test_factory_raises_when_nothing_configured(monkeypatch):
    monkeypatch.delenv("MISTRAL_API_KEY", raising=False)
    monkeypatch.delenv("GROQ_API_KEY", raising=False)
    import chat.llm.factory as f
    f._INSTANCES.clear()
    import pytest
    with pytest.raises(f.LLMUnavailableError):
        f.get_provider("auto")


# ---------------------------------------------------------------------------
# chat.agent.entity_memory
# ---------------------------------------------------------------------------

def test_entity_needs_context():
    from chat.agent.entity_memory import needs_context
    assert needs_context("et son prix ?") is True
    assert needs_context("compare-le au produit 42") is False  # id explicite
    assert needs_context("top 5 produits") is False
    assert needs_context("quel est son stock") is True


def test_entity_extract_from_history():
    from chat.agent.entity_memory import extract_entities
    from chat.types import Message
    hist = [
        Message(role="user", content="prix de Terreau ?"),
        Message(role="assistant", content='Le prix de "Terreau universel 20L" est 44 €'),
    ]
    ctx = extract_entities(hist)
    assert ctx.last_product_name == "Terreau universel 20L"


def test_entity_build_context_note():
    from chat.agent.entity_memory import build_context_note
    from chat.types import Message
    hist = [
        Message(role="assistant", content='Détails de "Chargeur Rapide 20W" : ...'),
    ]
    note = build_context_note("et son stock ?", hist)
    assert note is not None
    assert "Chargeur Rapide 20W" in note
    # Pas de note si la question ne fait pas référence à une entité
    assert build_context_note("top 5 produits", hist) is None


def test_entity_id_extraction():
    from chat.agent.entity_memory import extract_entities
    from chat.types import Message
    hist = [Message(role="user", content="détails du produit 42")]
    ctx = extract_entities(hist)
    assert ctx.last_product_id == 42


# ---------------------------------------------------------------------------
# chat.tools.registry — compression des gros payloads
# ---------------------------------------------------------------------------

def test_compress_truncates_long_lists():
    from chat.tools.registry import _compress, MAX_LIST_ITEMS
    big = list(range(100))
    out = _compress(big)
    # MAX_LIST_ITEMS éléments + 1 marqueur _tronque
    assert len(out) == MAX_LIST_ITEMS + 1
    assert isinstance(out[-1], dict) and "_tronque" in out[-1]


def test_compress_keeps_short_lists():
    from chat.tools.registry import _compress
    small = [1, 2, 3]
    assert _compress(small) == [1, 2, 3]


def test_compress_nested():
    from chat.tools.registry import _compress, MAX_LIST_ITEMS
    data = {"produits": list(range(50)), "total": 50}
    out = _compress(data)
    assert out["total"] == 50
    assert len(out["produits"]) == MAX_LIST_ITEMS + 1


# ---------------------------------------------------------------------------
# chat.agent.shortcuts — extraction de période
# ---------------------------------------------------------------------------

def test_extract_period_days():
    from chat.agent.shortcuts import extract_period_days
    assert extract_period_days("ces 7 derniers jours") == 7
    assert extract_period_days("cette semaine") == 7
    assert extract_period_days("ce mois") == 30
    assert extract_period_days("3 mois") == 90
    assert extract_period_days("cette année") == 365
    assert extract_period_days("top 5 produits") is None


def test_shortcut_injects_period():
    from chat.agent.shortcuts import match
    # Singulier → get_top_product_full (fiche du #1), période injectée via _PERIOD_AWARE_TOOLS
    sc = match("quel est le produit le plus vendu ces 7 derniers jours ?")
    assert sc is not None
    assert sc.tool == "get_top_product_full"
    assert sc.args.get("metric") == "volume"
    assert sc.args.get("period_days") == 7
    # Pluriel → get_top_products
    sc_pl = match("quels sont les produits les plus vendus ces 7 derniers jours ?")
    assert sc_pl is not None
    assert sc_pl.tool == "get_top_products"
    assert sc_pl.args.get("period_days") == 7


def test_shortcut_period_not_confused_with_limit():
    from chat.agent.shortcuts import match
    sc = match("top 5 produits par CA cette semaine")
    assert sc.tool == "get_top_products"
    assert sc.args.get("limit") == 5      # le 5 = limite
    assert sc.args.get("period_days") == 7  # cette semaine = période


def test_shortcut_top_products_wins_over_total_sales():
    from chat.agent.shortcuts import match
    # Singulier "quel est le produit le plus vendu" → fiche complète (#1 seulement)
    sc = match("quel est le produit le plus vendu ces 7 derniers jours ?")
    assert sc.tool == "get_top_product_full"
    # Pluriel "les produits les plus vendus" → top-10
    sc_pl = match("quels sont les produits les plus vendus ces 7 derniers jours ?")
    assert sc_pl.tool == "get_top_products"
    # Question CA globale reste sur get_total_sales
    sc2 = match("combien on a vendu ces 30 derniers jours")
    assert sc2.tool == "get_total_sales"


# ---------------------------------------------------------------------------
# chat.agent.entity_memory — match_followup (suivi anaphorique → outil)
# ---------------------------------------------------------------------------

def _followup_history():
    from chat.types import Message
    return [
        Message(role="user", content="produit le plus vendu ces 7 derniers jours"),
        Message(role="assistant",
                content="C'est le marqueur permanent noir (product_id: 199), 350 unités."),
    ]


def test_followup_revenue_uses_product_sales():
    from chat.agent.entity_memory import match_followup
    sc = match_followup("quel est le revenu qu'on a fait avec ces unités ?",
                        _followup_history())
    assert sc is not None
    assert sc.tool == "get_product_sales"
    assert sc.args.get("product_id") == 199
    assert sc.args.get("period_days") == 7  # période héritée du tour précédent


def test_followup_stock_uses_product_detail():
    from chat.agent.entity_memory import match_followup
    sc = match_followup("et son stock ?", _followup_history())
    assert sc is not None
    assert sc.tool == "get_product_detail"
    assert sc.args.get("product_id") == 199


def test_followup_no_entity_returns_none():
    from chat.agent.entity_memory import match_followup
    # Pas d'historique → rien à résoudre
    assert match_followup("et son prix ?", []) is None
    # Question non-anaphorique → pas un suivi
    assert match_followup("top 5 produits", _followup_history()) is None


# ---------------------------------------------------------------------------
# chat.agent.verify — garde-fou anti-hallucination
# ---------------------------------------------------------------------------

def test_verify_flags_hallucinated_number():
    from chat.agent.verify import verify_numbers
    data = [{"revenu_eur": 4167.47, "quantite": 350}]
    ok, unexplained = verify_numbers("Le revenu est de 560 196 EUR.", data)
    assert ok is False
    assert 560196.0 in unexplained


def test_verify_accepts_number_from_data():
    from chat.agent.verify import verify_numbers
    data = [{"revenu_eur": 4167.47, "quantite": 350}]
    ok, _ = verify_numbers("Revenu : 4 167,47 EUR pour 350 unités.", data)
    assert ok is True


def test_verify_accepts_rounding():
    from chat.agent.verify import verify_numbers
    data = [{"v": 4167.47}]
    ok, _ = verify_numbers("Environ 4167 EUR.", data)
    assert ok is True


def test_verify_no_data_no_false_positive():
    from chat.agent.verify import verify_numbers
    ok, _ = verify_numbers("Un grand nombre : 999999.", [])
    assert ok is True


def test_verify_ignores_small_numbers_and_years():
    from chat.agent.verify import verify_numbers
    data = [{"v": 5000.0}]
    # 12 (petit) et 2026 (année) ne doivent pas être flaggés
    ok, _ = verify_numbers("Sur 12 mois en 2026, total 5000 EUR.", data)
    assert ok is True


def test_verify_normalize_fr_en():
    from chat.agent.verify import _normalize
    assert _normalize("4 167,47") == 4167.47
    assert _normalize("4167.47") == 4167.47
    assert _normalize("39 355") == 39355.0


# ---------------------------------------------------------------------------
# Nouveaux outils 2026-05-22 — catalogue + shortcuts + intent
# ---------------------------------------------------------------------------

def test_new_tools_registered():
    from chat.tools import catalog
    names = {t.name for t in catalog()}
    for expected in ("get_category_analysis", "get_supplier_ranking",
                     "get_daily_action_list", "get_dormant_stock",
                     "get_negative_margin_products", "create_restock",
                     "resolve_alert", "update_product"):
        assert expected in names, f"manque {expected}"


def test_write_tools_require_confirmation():
    from chat.tools import catalog
    by_name = {t.name: t for t in catalog()}
    for w in ("create_restock", "resolve_alert", "update_product", "trigger_ai_run"):
        assert by_name[w].requires_confirmation is True


def test_shortcut_daily_action_list():
    from chat.agent.shortcuts import match
    sc = match("qu'est-ce que je dois faire aujourd'hui ?")
    assert sc is not None and sc.tool == "get_daily_action_list"


def test_shortcut_category_and_supplier():
    from chat.agent.shortcuts import match
    assert match("quelle catégorie marche le mieux").tool == "get_category_analysis"
    assert match("quel est le meilleur fournisseur").tool == "get_supplier_ranking"


def test_shortcut_dormant_and_negative_margin():
    from chat.agent.shortcuts import match
    assert match("quels produits dorment en stock").tool == "get_dormant_stock"
    sc = match("qu'est-ce qui me fait perdre de l'argent")
    assert sc.tool == "get_negative_margin_products"


def test_intent_write_actions():
    from chat.agent.intent import classify
    assert classify("crée un réappro de 100 unités du produit 8") == "action"
    assert classify("baisse le prix du produit 8 à 12 €") == "action"
    assert classify("marque l'alerte 5 comme résolue") == "action"


def test_shortcut_dormant_dort_en_stock():
    """Le shortcut doit matcher 'dort en stock' (3e personne du singulier)."""
    from chat.agent.shortcuts import match
    assert match("qu'est-ce qui dort en stock").tool == "get_dormant_stock"
    assert match("quels produits ne tournent pas").tool == "get_dormant_stock"


def test_shortcut_dormant_extrait_les_mois():
    """'depuis 6 mois' doit fixer months=6 (pas le défaut 3)."""
    from chat.agent.shortcuts import match
    sc = match("qu'est-ce qui dort en stock depuis 6 mois")
    assert sc.tool == "get_dormant_stock"
    assert sc.args.get("months") == 6


def test_render_dormant_stock_noms_fideles():
    """Le rendu déterministe reprend les vrais noms, pas 'Produit 1'."""
    from chat.agent.render import render
    data = {"fenetre_mois": 6, "produits_dormants": [
        {"category": "Animaux", "product_id": 187,
         "product_name": "Aquarium 20L kit complet", "value": 3.0},
        {"category": "Jardin", "product_id": 72,
         "product_name": "Gants de jardinage", "value": 4.0},
    ]}
    out = render("get_dormant_stock", data)
    assert out is not None
    assert "Aquarium 20L kit complet" in out
    assert "Gants de jardinage" in out
    assert "Produit 1" not in out
    assert "6 derniers mois" in out


def test_render_dormant_stock_vide():
    from chat.agent.render import render
    out = render("get_dormant_stock", {"fenetre_mois": 3, "produits_dormants": []})
    assert out is not None and "Aucun produit" in out


def test_render_negative_margin_aucun_a_perte():
    from chat.agent.render import render
    data = {"produits_a_perte": [], "produits_les_moins_profitables": [
        {"category": "Beaute", "product_id": 110,
         "product_name": "Eau de toilette 50ml", "value": 0.0}]}
    out = render("get_negative_margin_products", data)
    assert out is not None
    assert "aucun produit n'est strictement à perte" in out.lower()
    assert "Eau de toilette 50ml" in out


def test_render_top_products():
    from chat.agent.render import render
    data = {"metric": "revenue", "periode_jours": 30, "unit": "€", "products": [
        {"product_id": 67, "product_name": "Chargeur rapide 20W",
         "category": "Electronique", "value": 1061299.6}]}
    out = render("get_top_products", data)
    assert out is not None and "Chargeur rapide 20W" in out


def test_render_inconnu_retourne_none():
    """Un outil sans renderer déterministe → None (fallback LLM)."""
    from chat.agent.render import render
    assert render("get_forecast", {"x": 1}) is None


def test_render_urgent_restocks_trie_et_labels():
    """Urgent restocks : trié par urgence, quantité = reorder_quantity."""
    from chat.agent.render import render
    data = [
        {"product_name": "Puzzle 1000 pieces", "current_stock": 0,
         "days_until_stockout": 4, "reorder_quantity": 107,
         "recommended_stock": 107, "urgency": "URGENT"},
        {"product_name": "Pastilles lave-vaisselle x30", "current_stock": 0,
         "days_until_stockout": 0, "reorder_quantity": 696,
         "recommended_stock": 696, "urgency": "URGENT"},
    ]
    out = render("get_urgent_restocks", data)
    assert out is not None
    assert "Pastilles lave-vaisselle x30" in out
    assert "696" in out and "107" in out
    # le plus urgent (rupture dans 0 j) doit apparaître avant l'autre
    assert out.index("Pastilles") < out.index("Puzzle")
    assert "déjà en rupture" in out


def test_render_urgent_restocks_vide():
    from chat.agent.render import render
    out = render("get_urgent_restocks", [])
    assert out is not None and "Aucun produit" in out


def test_shortcut_singulier_top_product_full():
    """Singulier → get_top_product_full, pluriel → get_top_products."""
    from chat.agent.shortcuts import match
    # singulier
    assert match("quel est le produit le plus vendu cette semaine").tool == "get_top_product_full"
    assert match("quel est l'article le plus vendu").tool == "get_top_product_full"
    assert match("le produit le plus vendu ce mois").tool == "get_top_product_full"
    assert match("le produit le moins vendu").tool == "get_top_product_full"
    # pluriel — doit rester get_top_products
    assert match("quels sont les produits les plus vendus").tool == "get_top_products"
    assert match("les produits les plus vendus cette semaine").tool == "get_top_products"


def test_render_top_product_full_fiche():
    from chat.agent.render import render
    data = {
        "metric": "volume", "periode_jours": 7,
        "ranking_value": 340,
        "ranking_unit": "quantité totale vendue (unités)",
        "product": {
            "product_id": 8, "product_name": "Terreau universel 20L",
            "category": "Jardin", "reference": "REF0008",
            "buying_price": 5.0,
            "stock_quantity": 150, "status": "in_stock",
        },
        "kpis": {},
    }
    out = render("get_top_product_full", data)
    assert out is not None
    assert "Terreau universel 20L" in out
    assert "340" in out
    assert "5,00" in out or "5" in out  # buying_price
    assert "Jardin" in out
    assert "7 derniers jours" in out
