"""Lightweight intent classifier.

We use a rules-first approach (fast, deterministic, free) and only fall back
to an LLM classification if the rules are inconclusive. Keeps token usage low
and gives us predictable routing for common French/English phrasing.

Intents:
- doc      → conceptual / help question (what does X mean, how does Y work)
- data     → user wants data from the DB / KPI endpoints
- action   → user explicitly asks to trigger something
- chitchat → greetings, off-topic, social
"""

from __future__ import annotations

import re
from typing import Literal


Intent = Literal["doc", "data", "action", "chitchat"]


# Order matters — first match wins.
_RULES: list[tuple[Intent, list[str]]] = [
    (
        "action",
        [
            r"\b(lance|déclenche|relance|exécute|run)\b.*\b(forecast|prévision|modèle|ia|cycle|jobs?)\b",
            r"\b(crée|créer|créé|create|ajoute|ajouter)\b.*\b(alerte|alert)\b",
            r"\b(rafraîch[ie]r|refresh|recalcul[er]+)\b.*\b(prévisions?|forecasts?|modèles?)\b",
            # Actions d'écriture : réappro, ajustement de prix/stock, traiter alerte.
            # Accents optionnels — les utilisateurs FR les omettent souvent.
            r"\b(cr[ée]e?[rz]?|cr[ée][ée]|create|passe|lance|fais)\b.*\b(commande|r[ée]appro\w*|restock)\b",
            r"\bcommande[rz]?\b.*\b(unit[ée]s?|produit|articles?|chez)\b",
            r"\b(ajuste|change|modifie|corrige|mets?|met\s+[àa]\s+jour|baisse|augmente|passe)\b.*\b(prix|tarif|stock|quantit[ée])\b",
            r"\b(marque|passe|mets?)\b.*\balertes?\b.*\b(r[ée]solu\w*|trait[ée]?\w*|acquitt\w*|ignor\w*)",
            r"\b(marque|mets?|passe)\b.*\bproduit\b.*\b(arr[êe]t\w*|discontinu\w*|rupture)\b",
        ],
    ),
    (
        "doc",
        [
            r"\b(qu['e]?[\s-]?est[\s-]?ce[\s-]?(que|qu['e]))\b",
            # "que signifie", "que veut dire", "que veux dire" (faute orale courante)
            r"\bque (signifie|veu[txt] dire|veux dire)\b",
            # "ça veut dire quoi", "ça signifie quoi"
            r"\b(ça|ca|c['e]?st) (veut dire|signifie)\b",
            r"\bcomment (fonctionne|marche|ça se passe)\b",
            r"\bhow (does|do) .* work\b",
            r"\bwhat (is|are|does|do)\b",
            r"\b(explique|explain|définition|definition|exactement)\b",
            r"\bABC[ -]?XYZ\b",
            r"\b(clustering|kpi|kpis)\b",
        ],
    ),
    (
        "data",
        [
            r"\b(combien|how many|how much)\b",
            r"\b(top|best|meilleur[se]?|pires?|worst|flop)\b",
            r"\b(stock|rupture|surstock|alerte)\b",
            r"\b(fournisseur|supplier|category|catégorie)\b",
            r"\b(ventes?|chiffre d'affaires|ca|revenue|profit|marge)\b",
            r"\b(produit|product)\s+(\d+|n[°o]\s*\d+|#\d+)\b",
            r"\bid\s*\d+\b",
            r"\bpr[ée]vision",
        ],
    ),
    (
        "chitchat",
        [
            r"^(salut|bonjour|hello|hi|hey|coucou|bonsoir)\b",
            r"\b(merci|thanks|thank you)\b",
            r"\bcomment (vas-tu|tu vas|ça va)\b",
        ],
    ),
]


def classify(message: str) -> Intent:
    text = message.strip().lower()
    if not text:
        return "chitchat"
    for intent, patterns in _RULES:
        for p in patterns:
            if re.search(p, text):
                return intent
    # Default to data (the most useful answer for stock-management questions
    # about specific products/categories that don't match heuristics above).
    return "data"
