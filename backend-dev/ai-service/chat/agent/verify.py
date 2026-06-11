"""Garde-fou anti-hallucination.

Vérifie que les nombres « significatifs » cités dans une réponse du bot
proviennent réellement des données renvoyées par les outils. On a observé
des cas où le LLM local inventait des chiffres plausibles (« 560 196 € »,
« 1400 € ») totalement absents du résultat outil.

Principe :
1. Extraire les nombres de la réponse et du payload outil.
2. Normaliser (FR « 4 167,47 » == EN « 4167.47 »).
3. Pour chaque nombre significatif de la réponse, vérifier qu'il correspond
   à un nombre des données — exact, arrondi, ou partie entière.
4. Si des nombres restent inexpliqués → la réponse est suspecte.

On reste tolérant : arrondis et reformatages sont acceptés. Le but est de
détecter l'invention pure, pas de chasser chaque décimale.
"""

from __future__ import annotations

import re
from typing import Iterable


# Capture un nombre : "4167.47", "4 167,47", "39 355", "55,15", "1413".
_NUM_RE = re.compile(r"\d[\d  .,]*\d|\d")

# En dessous de ce seuil, on ignore (compteurs, petits indices, années).
_SIGNIFICANCE_THRESHOLD = 100.0
# Tolérance relative pour considérer deux nombres « égaux ».
_REL_TOL = 0.02


def _normalize(token: str) -> float | None:
    """Convertit un token numérique FR/EN en float. None si impossible."""
    s = token.strip().replace(" ", " ").replace("\xa0", " ")
    s = s.replace(" ", "")
    if not s:
        return None
    # Déterminer le séparateur décimal : le dernier ',' ou '.' suivi de 1-2
    # chiffres est décimal ; le reste sont des séparateurs de milliers.
    last_comma = s.rfind(",")
    last_dot = s.rfind(".")
    dec_pos = max(last_comma, last_dot)
    if dec_pos != -1 and len(s) - dec_pos - 1 in (1, 2):
        intpart = re.sub(r"[.,]", "", s[:dec_pos])
        decpart = s[dec_pos + 1:]
        s = f"{intpart}.{decpart}"
    else:
        s = re.sub(r"[.,]", "", s)
    try:
        return float(s)
    except ValueError:
        return None


def _extract_numbers(text: str) -> list[float]:
    out: list[float] = []
    for m in _NUM_RE.finditer(text or ""):
        v = _normalize(m.group(0))
        if v is not None:
            out.append(v)
    return out


def _collect_data_numbers(payloads: Iterable) -> set[float]:
    """Tous les nombres présents dans les payloads outil (récursif)."""
    nums: set[float] = set()

    def walk(v) -> None:
        if isinstance(v, bool):
            return
        if isinstance(v, (int, float)):
            nums.add(float(v))
        elif isinstance(v, str):
            for n in _extract_numbers(v):
                nums.add(n)
        elif isinstance(v, dict):
            for x in v.values():
                walk(x)
        elif isinstance(v, (list, tuple)):
            for x in v:
                walk(x)

    for p in payloads:
        walk(p)
    return nums


def _is_explained(n: float, data: set[float]) -> bool:
    """Vrai si n correspond à une donnée (exact, arrondi, ou échelle)."""
    for d in data:
        if d == 0:
            if abs(n) < 1e-9:
                return True
            continue
        # Égalité à tolérance relative.
        if abs(n - d) / abs(d) <= _REL_TOL:
            return True
        # n est un arrondi de d (entier, ou 1 décimale).
        if abs(n - round(d)) < 0.5 or abs(n - round(d, 1)) < 0.05:
            return True
        # d est un arrondi de n.
        if abs(d - round(n)) < 0.5:
            return True
    return False


def verify_numbers(response: str, tool_payloads: list) -> tuple[bool, list[float]]:
    """Vérifie les nombres de `response` contre les données des outils.

    Retourne (ok, nombres_non_expliqués). `ok` est True s'il n'y a aucun
    nombre significatif inexpliqué.

    Si aucun outil n'a fourni de données (tool_payloads vide), on ne peut
    rien vérifier → on renvoie ok=True (pas de faux positif).
    """
    if not tool_payloads:
        return True, []
    data = _collect_data_numbers(tool_payloads)
    if not data:
        return True, []

    unexplained: list[float] = []
    for n in _extract_numbers(response):
        # On ignore les petits nombres (compteurs, rangs) et les années.
        if abs(n) < _SIGNIFICANCE_THRESHOLD:
            continue
        if 1900 <= n <= 2100 and n == int(n):
            continue
        if not _is_explained(n, data):
            unexplained.append(n)

    return (not unexplained), unexplained
