"""Shortcuts sémantiques — matching par embedding plutôt que par regex.

Les regex de `shortcuts.py` sont rapides et déterministes mais fragiles : il
faut anticiper chaque formulation. Ici on prend l'approche inverse :

1. On définit une bibliothèque de **questions canoniques**, chacune mappée à
   un (tool, args).
2. Au démarrage, on embed toutes ces questions canoniques (modèle déjà chargé
   pour le RAG → coût RAM nul).
3. À l'exécution, on embed la question de l'utilisateur et on cherche la
   canonique la plus proche (cosine). Si la similarité dépasse un seuil, on
   utilise le shortcut associé.

Pipeline complet : regex shortcuts (exact, 0 ms) → sémantique (robuste aux
paraphrases) → routage LLM classique.
"""

from __future__ import annotations

import os
import threading
from dataclasses import dataclass
from typing import Optional

from utils.logger import get_logger

from .shortcuts import Shortcut


logger = get_logger("chat.agent.semantic")

# Seuil de similarité cosine au-dessus duquel on fait confiance au match.
# 0.62 est volontairement prudent : en dessous on laisse le LLM décider.
SIM_THRESHOLD = float(os.getenv("CHAT_SEMANTIC_SHORTCUT_THRESHOLD", "0.62"))


@dataclass
class _Canonical:
    question: str
    tool: str
    args: dict


# Bibliothèque de questions canoniques. Plusieurs formulations par intention
# pour couvrir le spectre. La similarité fait le reste.
_CANONICALS: list[_Canonical] = [
    # ── stock bas / rupture ──────────────────────────────────────────────
    _Canonical("quels produits sont en stock bas", "get_low_stock", {}),
    _Canonical("liste des articles bientôt épuisés", "get_low_stock", {}),
    _Canonical("qu'est-ce qui manque en stock", "get_low_stock", {}),
    _Canonical("produits en rupture de stock", "get_alerts", {"severity": "CRITICAL"}),
    _Canonical("quels articles sont presque épuisés", "get_soon_out_of_stock", {}),
    _Canonical("produits sur le point d'être en rupture", "get_soon_out_of_stock", {}),

    # ── surstock ─────────────────────────────────────────────────────────
    _Canonical("quels produits ont trop de stock", "get_overstock", {}),
    _Canonical("articles en surstock", "get_overstock", {}),
    _Canonical("qu'est-ce qui dort dans l'entrepôt", "get_overstock", {}),
    _Canonical("produits avec un stock excessif", "get_overstock", {}),

    # ── top / flop produits ──────────────────────────────────────────────
    _Canonical("quels sont les meilleurs produits", "get_top_products", {"metric": "revenue", "limit": 10}),
    _Canonical("produits qui rapportent le plus d'argent", "get_top_products", {"metric": "revenue", "limit": 10}),
    _Canonical("articles les plus rentables", "get_top_products", {"metric": "profit", "limit": 10}),
    _Canonical("produits les plus vendus en quantité", "get_top_products", {"metric": "volume", "limit": 10}),
    _Canonical("quels sont les pires produits", "get_top_products", {"metric": "flop_sales", "limit": 10}),
    _Canonical("articles qui se vendent le moins bien", "get_top_products", {"metric": "flop_sales", "limit": 10}),

    # ── KPI globaux / santé business ─────────────────────────────────────
    _Canonical("comment va mon entreprise", "get_global_kpis", {"period_days": 30}),
    _Canonical("donne-moi un résumé de l'activité", "get_global_kpis", {"period_days": 30}),
    _Canonical("quels sont mes indicateurs de performance", "get_global_kpis", {"period_days": 30}),
    _Canonical("est-ce que le commerce marche bien", "get_global_kpis", {"period_days": 30}),
    _Canonical("vue d'ensemble de mes chiffres", "get_global_kpis", {"period_days": 30}),

    # ── ventes / CA ──────────────────────────────────────────────────────
    _Canonical("combien d'argent on a gagné ce mois", "get_total_sales", {"period_days": 30}),
    _Canonical("quel est le chiffre d'affaires récent", "get_total_sales", {"period_days": 30}),
    _Canonical("combien de ventes sur la dernière période", "get_total_sales", {"period_days": 30}),

    # ── stock global ─────────────────────────────────────────────────────
    _Canonical("combien de stock il reste au total", "get_stock_summary", {}),
    _Canonical("synthèse de l'état du stock", "get_stock_summary", {}),
    _Canonical("est-ce que le stock est bien géré", "get_stock_summary", {}),

    # ── alertes ──────────────────────────────────────────────────────────
    _Canonical("quelles sont les alertes en cours", "get_alerts", {"limit": 20}),
    _Canonical("y a-t-il des problèmes à régler", "get_alerts", {"limit": 20}),
    _Canonical("montre-moi les alertes critiques", "get_alerts", {"severity": "CRITICAL", "limit": 20}),

    # ── réapprovisionnement ──────────────────────────────────────────────
    _Canonical("quelles livraisons sont en attente", "get_pending_restocks", {"limit": 20}),
    _Canonical("réapprovisionnements en cours", "get_pending_restocks", {"limit": 20}),
    _Canonical("qu'est-ce qu'il faut recommander en urgence", "get_urgent_restocks", {}),
    _Canonical("quels produits faut-il réapprovisionner", "get_urgent_restocks", {}),

    # ── anomalies ────────────────────────────────────────────────────────
    _Canonical("y a-t-il des anomalies de ventes", "get_sales_anomalies", {"limit": 10}),
    _Canonical("anomalies de prix détectées", "get_price_anomalies", {"limit": 10}),

    # ── suggestions de prix ──────────────────────────────────────────────
    _Canonical("comment optimiser mes prix", "get_price_suggestions", {"limit": 10}),
    _Canonical("suggestions pour ajuster les tarifs", "get_price_suggestions", {"limit": 10}),
]


_lock = threading.Lock()
_embeddings: Optional[list[list[float]]] = None  # parallèle à _CANONICALS


def _ensure_embeddings() -> Optional[list[list[float]]]:
    """Embed les questions canoniques une seule fois (lazy)."""
    global _embeddings
    if _embeddings is not None:
        return _embeddings
    with _lock:
        if _embeddings is not None:
            return _embeddings
        try:
            from ..rag.embedder import get_embedder
            embedder = get_embedder()
            texts = [c.question for c in _CANONICALS]
            _embeddings = embedder.embed(texts)
            logger.info("Shortcuts sémantiques : %d questions canoniques embeddées",
                        len(_CANONICALS))
        except Exception as e:
            logger.warning("Embeddings shortcuts sémantiques indisponibles (%s)", e)
            _embeddings = None
    return _embeddings


def _cosine(a: list[float], b: list[float]) -> float:
    # Les embeddings du modèle sont déjà normalisés L2 → produit scalaire.
    return sum(x * y for x, y in zip(a, b))


def match_semantic(question: str) -> Optional[tuple[Shortcut, float]]:
    """Retourne (Shortcut, similarité) si une canonique est assez proche,
    sinon None."""
    q = (question or "").strip()
    if len(q) < 4:
        return None
    embs = _ensure_embeddings()
    if not embs:
        return None
    try:
        from ..rag.embedder import get_embedder
        qvec = get_embedder().embed_one(q)
    except Exception as e:
        logger.debug("embed question échoué (%s)", e)
        return None

    best_idx = -1
    best_sim = -1.0
    for i, cvec in enumerate(embs):
        sim = _cosine(qvec, cvec)
        if sim > best_sim:
            best_sim = sim
            best_idx = i

    if best_idx >= 0 and best_sim >= SIM_THRESHOLD:
        canon = _CANONICALS[best_idx]
        logger.info("Shortcut sémantique : %r ~ %r (sim=%.3f) → %s",
                    q[:50], canon.question, best_sim, canon.tool)
        return Shortcut(canon.tool, dict(canon.args)), best_sim
    return None
