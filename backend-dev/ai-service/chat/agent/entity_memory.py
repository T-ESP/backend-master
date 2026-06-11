"""Mémoire d'entités conversationnelle.

Permet les questions de suivi naturelles :

    User : quel est le prix de Terreau universel 20L ?
    Bot  : 44.08 €
    User : et son stock ?            ← "son" = Terreau universel 20L
    User : compare-le au produit 42  ← "le" = Terreau universel 20L

On scanne l'historique récent pour repérer les produits / identifiants
mentionnés, et on injecte un rappel de contexte dans le prompt système
quand la question courante contient une référence anaphorique ("son", "le",
"celui-ci", "ce produit"…) sans nommer explicitement d'entité.

100 % Python, aucun coût RAM, aucun appel réseau.
"""

from __future__ import annotations

import re
from dataclasses import dataclass
from typing import Optional

from ..types import Message


# Marqueurs anaphoriques. ATTENTION : "le/la/les" comme articles sont
# omniprésents en français ("le mieux", "la boutique") — on ne les compte
# PAS comme anaphores. On ne retient que :
#  - les possessifs (son, sa, ses, leur)
#  - les démonstratifs explicites (celui-ci, ce produit, ce dernier…)
#  - les pronoms objets ACCROCHÉS à un verbe par un tiret (compare-le, montre-la)
_ANAPHORA_RE = re.compile(
    r"\b(son|sa|ses|leurs?)\b"
    r"|\b(celui|celle|ceux|celles)(-ci|-là)?\b"
    r"|\bce\s+(produit|dernier|article|fournisseur)\b"
    r"|\bcet\s+article\b"
    r"|\bces\s+(\d+\s+)?(unit[ée]s?|produits?|articles?|ventes?)\b"  # "ces 350 unités"
    r"|-(le|la|les|en|y)\b",          # pronom objet : compare-le, montre-les
    re.IGNORECASE,
)

# Détecte un identifiant produit explicite déjà présent dans la question.
_EXPLICIT_ID_RE = re.compile(r"\bproduit\s*#?\s*\d+\b|\bid\s*\d+\b", re.IGNORECASE)

# Repère un identifiant produit sous toutes ses formes courantes :
# "produit 199", "produit #199", "product_id: 199", "id_pro 199", "(ID 199)".
_PRODUCT_ID_RE = re.compile(
    r"\b(?:produit|article|product[_ ]?id|id[_ ]?pro)\s*[:#°]?\s*(\d+)\b",
    re.IGNORECASE,
)
# Nom de produit : suite de mots dont au moins un commence par une majuscule
# ou un chiffre + unité (ex : "Terreau universel 20L", "Chargeur Rapide 20W").
_PRODUCT_NAME_RE = re.compile(
    r"\b([A-ZÀ-Ÿ][\wÀ-ÿ-]+(?:\s+[\wÀ-ÿ-]+){0,4}\s*\d*\s*[A-Za-z]{0,3})\b"
)


@dataclass
class EntityContext:
    last_product_name: Optional[str] = None
    last_product_id: Optional[int] = None
    last_supplier_id: Optional[int] = None
    last_period_days: Optional[int] = None

    def is_empty(self) -> bool:
        return not (self.last_product_name or self.last_product_id
                    or self.last_supplier_id or self.last_period_days)


# "prix/stock/... de <Nom>" — capture un nom de produit dans un message user.
_NAME_AFTER_DE_RE = re.compile(
    r"\b(?:de|du|des|de\s+la|de\s+l['’]|pour|sur)\s+"
    r"(?:le\s+|la\s+|les\s+|l['’]\s*)?(?:produit\s+|article\s+)?"
    r"([A-ZÀ-Ÿ0-9][\wÀ-ÿ-]*(?:\s+[\wÀ-ÿ0-9-]+){0,5})",
)


def extract_entities(history: list[Message]) -> EntityContext:
    """Scanne l'historique (du plus récent au plus ancien) pour trouver le
    dernier produit / fournisseur mentionné."""
    ctx = EntityContext()
    # Parcourt à l'envers : le plus récent gagne.
    for msg in reversed(history):
        content = msg.content or ""
        if ctx.last_product_id is None:
            m = _PRODUCT_ID_RE.search(content)
            if m:
                ctx.last_product_id = int(m.group(1))
        if ctx.last_supplier_id is None:
            ms = re.search(r"\bfournisseur\s*#?\s*(\d+)\b", content, re.IGNORECASE)
            if ms:
                ctx.last_supplier_id = int(ms.group(1))
        if ctx.last_product_name is None:
            name = _guess_product_name(content)
            if name:
                ctx.last_product_name = name
        if ctx.last_period_days is None:
            # Réutilise l'extracteur de période des shortcuts.
            try:
                from .shortcuts import extract_period_days
                p = extract_period_days(content)
                if p:
                    ctx.last_period_days = p
            except Exception:
                pass
        if ctx.last_product_name and ctx.last_product_id:
            break
    return ctx


def _guess_product_name(text: str) -> Optional[str]:
    """Extrait un nom de produit plausible d'un message (user ou assistant).

    Trois stratégies, par ordre de fiabilité :
      1. entre guillemets "..." / « ... » (l'assistant cite souvent ainsi)
      2. après "de/du/pour ..." (les questions user : "prix de Terreau ...")
      3. rien → None
    """
    # 1. Entre guillemets
    q = re.search(r'["«»“”]\s*([^"«»“”]{3,60}?)\s*["«»“”]', text)
    if q:
        cand = q.group(1).strip()
        if len(cand) >= 3:
            return cand
    # 2. Après "de/du/pour" : un nom propre (commence par majuscule/chiffre)
    m = _NAME_AFTER_DE_RE.search(text)
    if m:
        cand = m.group(1).strip().rstrip("?.!,;:")
        # Coupe au premier mot-outil. On NE met PAS "de/du/des" ni les
        # articles : ils apparaissent dans de vrais noms ("Sirop de grenadine",
        # "Crème de jour"). On coupe seulement sur les verbes / connecteurs qui
        # ne peuvent pas faire partie d'un nom de produit.
        _STOP = {"est", "était", "coûte", "coute", "vaut", "avec", "dans",
                 "qui", "que", "c'est", "cest", "fait", "a", "ont", "sont",
                 "merci", "stp", "svp"}
        words = cand.split()
        cut = []
        for w in words:
            if w.lower() in _STOP and cut:  # garde le 1er mot même si stop
                break
            cut.append(w)
        cand = " ".join(cut).strip()
        if len(cand) >= 4 and (cand[0].isupper() or any(c.isdigit() for c in cand)):
            return cand
    return None


def needs_context(question: str) -> bool:
    """Vrai si la question contient une référence anaphorique et PAS
    d'entité explicite — donc a besoin du contexte précédent."""
    q = question.strip()
    if _EXPLICIT_ID_RE.search(q):
        return False
    # Question très courte avec un pronom = quasi certainement un suivi.
    has_anaphora = bool(_ANAPHORA_RE.search(q))
    return has_anaphora


def build_context_note(question: str, history: list[Message]) -> Optional[str]:
    """Retourne une note de contexte à injecter dans le prompt système,
    ou None si pas nécessaire."""
    if not needs_context(question) or not history:
        return None
    ctx = extract_entities(history)
    if ctx.is_empty():
        return None
    bits = []
    if ctx.last_product_name:
        bits.append(f"dernier produit mentionné : « {ctx.last_product_name} »")
    if ctx.last_product_id:
        bits.append(f"dernier id produit mentionné : {ctx.last_product_id}")
    if ctx.last_supplier_id:
        bits.append(f"dernier fournisseur mentionné : #{ctx.last_supplier_id}")
    if ctx.last_period_days:
        bits.append(f"dernière période évoquée : {ctx.last_period_days} jours")
    if not bits:
        return None
    return (
        "Contexte de la conversation (pour résoudre les références comme "
        "« son », « le », « ce produit », « ces unités ») — "
        + " ; ".join(bits) + ". "
        "Si la question courante fait référence à un produit ou une quantité "
        "sans le nommer, il s'agit très probablement de cet élément. "
        "Pour le revenu/CA d'UN produit précis, utilise get_product_sales "
        "(pas get_total_sales qui donne le CA global du magasin)."
    )


# ──────────────────────────────────────────────────────────────────────────
# Shortcut de suivi : résout une question anaphorique en appel d'outil direct
# ──────────────────────────────────────────────────────────────────────────
#
# Les petits LLM locaux (~3B) échouent souvent à enchaîner "comprendre la
# référence -> appeler le bon outil avec les bons args". On le fait donc
# nous-mêmes, de façon déterministe, quand on a l'entité en mémoire.

def match_followup(question: str, history: list[Message]):
    """Retourne un Shortcut (tool, args) si la question de suivi peut être
    résolue depuis le contexte d'entité, sinon None.

    Importé paresseusement pour éviter une dépendance circulaire avec
    shortcuts.py.
    """
    from .shortcuts import Shortcut, extract_period_days

    if not needs_context(question):
        return None
    ctx = extract_entities(history)
    if ctx.is_empty():
        return None

    q = question.lower()
    # Période : celle de la question si présente, sinon celle mémorisée.
    period = extract_period_days(question) or ctx.last_period_days

    # Identifiant produit utilisable (id direct, sinon on passe le nom).
    has_product = ctx.last_product_id is not None or ctx.last_product_name

    def _product_args(extra: dict | None = None) -> dict:
        a: dict = {}
        if ctx.last_product_id is not None:
            a["product_id"] = ctx.last_product_id
        elif ctx.last_product_name:
            a["name"] = ctx.last_product_name
        if extra:
            a.update(extra)
        return a

    # 1. Revenu / CA / combien rapporté — d'UN produit
    if has_product and re.search(
        r"\b(revenu|chiffre\s+d['’]affaires|ca\b|rapport[ée]|gagn[ée]|"
        r"combien.*rapport)", q
    ):
        args = _product_args()
        if period:
            args["period_days"] = period
        return Shortcut("get_product_sales", args)

    # 2. Quantité vendue d'UN produit ("combien on en a vendu")
    if has_product and re.search(r"\b(combien).*(vendu|écoul)", q):
        args = _product_args()
        if period:
            args["period_days"] = period
        return Shortcut("get_product_sales", args)

    # 3. Prix / stock / marge / détails / catégorie — d'UN produit
    if has_product and re.search(
        r"\b(prix|tarif|co[ûu]t|stock|marge|d[ée]tails?|cat[ée]gorie|"
        r"fournisseur|caract[ée]ristiques?)\b", q
    ):
        if ctx.last_product_id is not None:
            return Shortcut("get_product_detail",
                            {"product_id": ctx.last_product_id})
        return Shortcut("get_product_by_name", {"name": ctx.last_product_name})

    # 4. Prévision / forecast / durée de stock d'UN produit
    if has_product and re.search(
        r"\b(pr[ée]vision|forecast|pr[ée]voir|"
        r"jusqu['']\s*[àa]\s+quand|combien\s+de\s+temps|"
        r"dur\w+|[ée]puiser?|tenir|couvrir|rupture\s+dans|"
        r"quand\s+(?:va[- ]t[- ]il|sera[- ]t[- ]il|manquer|finir))\b", q
    ):
        if ctx.last_product_id is not None:
            return Shortcut("get_forecast", {"product_id": ctx.last_product_id})
        elif ctx.last_product_name:
            # On a le nom mais pas l'ID : get_product_by_name renvoie les KPIs
            # incluant ai_days_until_stockout et estimated_stockout_date.
            return Shortcut("get_product_by_name",
                            {"name": ctx.last_product_name})

    # 5. Classification ABC-XYZ d'UN produit
    if ctx.last_product_id is not None and re.search(
        r"\b(classification|abc|xyz|class[ée])\b", q
    ):
        return Shortcut("get_classification",
                        {"product_id": ctx.last_product_id})

    return None
