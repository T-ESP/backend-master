"""Per-turn orchestrator: shortcuts → cache → intent → RAG/tool loop → final.

Pipeline (skips short-circuit cleanly when an earlier stage hits):

    user_message
        │
        ├─ Deterministic shortcut?  ─── yes ──▶ Direct tool call → format → done
        │
        ├─ intent = doc + cache hit?  ── yes ──▶ Return cached answer → done
        │
        ▼
    intent classification (rules)
        │
        ├─ doc      → hybrid RAG (vector + keyword + reranker) → 1 LLM call
        ├─ data     → tool-loop (max N iterations) → answer
        ├─ action   → tool-loop + confirmation gate → answer
        └─ chitchat → 1 LLM call
"""

from __future__ import annotations

import os
import re
from dataclasses import dataclass, field
from typing import Any, Optional

from utils.logger import get_logger

from ..llm import LLMUnavailableError, get_provider
from ..llm.factory import chat_with_fallback
from ..rag import format_context, retrieve_hybrid
from ..tools import ToolContext, execute_tool, get_tool, tool_specs
from ..types import ChatResponse, ChatUsage, Message, ToolCall, ToolSpec
from .cache import lookup as cache_lookup, store as cache_store
from .compression import maybe_compress
from .entity_memory import (
    build_context_note,
    match_followup,
    needs_context as entity_needs_context,
)
from .intent import Intent, classify
from .render import render as render_result
from .semantic_shortcuts import match_semantic
from .shortcuts import match as match_shortcut
from .suggest import generate as generate_suggestions
from .system_prompt import build_system_prompt


MAX_TOOL_ITERATIONS = int(os.getenv("CHAT_MAX_TOOL_ITERATIONS", "3"))
DEFAULT_RAG_TOP_K = int(os.getenv("CHAT_RAG_TOP_K", "4"))
DEFAULT_RAG_FETCH_K = int(os.getenv("CHAT_RAG_FETCH_K", "20"))

logger = get_logger("chat.agent")


@dataclass
class Citation:
    source_path: str
    heading: str
    similarity: float


@dataclass
class TurnResult:
    content: str
    intent: Intent
    provider_used: str
    tool_calls: list[dict] = field(default_factory=list)
    pending_action: Optional[dict] = None
    citations: list[Citation] = field(default_factory=list)
    suggestions: list[dict] = field(default_factory=list)
    usage: ChatUsage = field(default_factory=ChatUsage)
    cached: bool = False
    shortcut_used: Optional[str] = None
    # Garde-fou anti-hallucination : False si la réponse cite des nombres
    # significatifs absents des données outil.
    numbers_verified: bool = True
    # Données brutes des outils appelés ce tour (pour la vérification).
    tool_data: list = field(default_factory=list)


def _detect_lang(text: str) -> str:
    if not text:
        return "fr"
    if any(c in text for c in "àâäéèêëîïôöùûüÿçÀÂÄÉÈÊËÎÏÔÖÙÛÜŸÇ"):
        return "fr"
    low = text.lower()
    fr_words = ("le ", "la ", " et ", " des ", " est ", "comment", "quel", "quelle",
                "produit", "stock", "fournisseur", "rupture", "alerte")
    if any(w in low for w in fr_words):
        return "fr"
    en_words = ("the ", " and ", " is ", " what ", " how ", " supplier", " stock", " alert")
    if any(w in low for w in en_words):
        return "en"
    return "fr"


def run_turn(
    *,
    user_message: str,
    history: list[Message],
    ctx: ToolContext,
    provider_pref: Optional[str] = None,
    proactive_summary: Optional[str] = None,
) -> TurnResult:
    intent = classify(user_message)
    user_lang = _detect_lang(user_message)
    logger.info("turn intent=%s lang=%s pref=%s", intent, user_lang, provider_pref)

    # Les questions de suivi anaphoriques ("et son prix ?", "le revenu de
    # ces unités") ne sont pas court-circuitées par un shortcut générique.
    is_followup = entity_needs_context(user_message)

    # ---- Stage 0-followup: shortcut de suivi résolu via le contexte d'entité ----
    # Les petits LLM locaux échouent à enchaîner "comprendre la référence →
    # appeler le bon outil". On le fait nous-mêmes : si on a l'entité en
    # mémoire et qu'on reconnaît l'intention, on appelle l'outil directement.
    if is_followup:
        fsc = match_followup(user_message, history)
        if fsc is not None:
            logger.info("shortcut (followup) matched: %s(%s)", fsc.tool, fsc.args)
            return _shortcut_turn(fsc.tool, fsc.args, user_message, ctx, provider_pref)

    # Les actions d'écriture (créer un réappro, ajuster un prix…) ne doivent
    # PAS être court-circuitées : un shortcut de lecture attraperait à tort
    # "produit 8" dans "crée un réappro du produit 8". Elles passent par le
    # LLM qui extrait les arguments structurés, puis le garde-fou de
    # confirmation s'applique.
    skip_shortcuts = is_followup or intent == "action"

    # ---- Stage 0a: deterministic regex shortcut (exact, 0 ms) ----
    if not skip_shortcuts:
        sc = match_shortcut(user_message)
        if sc is not None:
            logger.info("shortcut (regex) matched: %s(%s)", sc.tool, sc.args)
            return _shortcut_turn(sc.tool, sc.args, user_message, ctx, provider_pref)

    # ---- Stage 0b: semantic shortcut (robust to paraphrases) ----
    # Lecture seule : on ne tente le sémantique que pour les questions data.
    if intent == "data" and not skip_shortcuts:
        sem = match_semantic(user_message)
        if sem is not None:
            shortcut, sim = sem
            logger.info("shortcut (semantic) matched: %s(%s) sim=%.3f",
                        shortcut.tool, shortcut.args, sim)
            return _shortcut_turn(shortcut.tool, shortcut.args, user_message,
                                  ctx, provider_pref)

    # ---- Stage 1: response cache for doc/concept questions ----
    if intent == "doc":
        hit = cache_lookup(user_message)
        if hit:
            content, provider = hit
            logger.info("doc cache hit (provider=%s)", provider)
            return TurnResult(
                content=content,
                intent="doc",
                provider_used=provider,
                cached=True,
            )

    # ---- Stage 2: build system prompt ----
    rag_block: Optional[str] = None
    citations: list[Citation] = []
    if intent == "doc":
        try:
            hits = retrieve_hybrid(
                user_message,
                fetch_k=DEFAULT_RAG_FETCH_K,
                top_k=DEFAULT_RAG_TOP_K,
                use_reranker=True,
            )
            rag_block = format_context(hits)
            citations = [
                Citation(source_path=h.source_path, heading=h.heading,
                         similarity=round(h.similarity, 4))
                for h in hits
            ]
        except Exception as e:
            logger.warning("RAG retrieve failed: %s", e)

    # ---- Stage 2b: entity-memory context note for follow-up questions ----
    # "et son prix ?", "compare-le au 42" → injecte le dernier produit cité.
    entity_note: Optional[str] = None
    try:
        entity_note = build_context_note(user_message, history)
        if entity_note:
            logger.info("entity-memory: contexte injecté")
    except Exception as e:
        logger.debug("entity-memory failed: %s", e)

    system_prompt = build_system_prompt(
        proactive_summary=proactive_summary,
        rag_context=rag_block,
        user_lang=user_lang,
    )
    if entity_note:
        system_prompt = system_prompt + "\n\n" + entity_note

    # ---- Stage 3: history compression for long sessions ----
    compressed_history, _ = maybe_compress(
        ctx.session_id, history, provider_pref=provider_pref
    )

    messages: list[Message] = [Message(role="system", content=system_prompt)]
    messages.extend(compressed_history)
    messages.append(Message(role="user", content=user_message))

    # ---- Stage 4: tool selection per intent ----
    if intent == "chitchat":
        tools: list[ToolSpec] = []
    else:
        tools = tool_specs(read_only=(intent != "action"))

    # ---- Stage 5: tool loop (or 1-shot for chitchat) ----
    result = _tool_loop(
        messages=messages,
        tools=tools,
        ctx=ctx,
        provider_pref=provider_pref,
        intent=intent,
    )
    result.citations = citations

    # ---- Stage 5b: self-recovery for failed tool calls ----
    # Sur les petits LLM locaux, il arrive que le modèle annonce "je vais
    # appeler get_X" mais émette du texte au lieu d'un <tool_call>. On
    # détecte ce pattern et on rejoue le tool extrait.
    if intent in ("data", "action") and not result.tool_calls and result.content:
        recovered = _recover_tool_from_text(result.content, user_message, ctx,
                                             provider_pref)
        if recovered is not None:
            result = recovered

    # ---- Stage 5c: anti-hallucination — vérifie les chiffres cités ----
    if result.content and result.tool_data:
        try:
            from .verify import verify_numbers
            ok, unexplained = verify_numbers(result.content, result.tool_data)
            result.numbers_verified = ok
            if not ok:
                logger.warning(
                    "anti-hallucination: chiffres non vérifiés dans la réponse "
                    "(%s) — question=%r",
                    ", ".join(f"{n:.2f}" for n in unexplained), user_message[:80],
                )
        except Exception as e:
            logger.debug("verify_numbers failed: %s", e)

    # ---- Stage 6: cache successful doc answers ----
    # On ne cache PAS une réponse dont les chiffres n'ont pas été vérifiés.
    if (intent == "doc" and result.content and not result.tool_calls
            and result.numbers_verified):
        cache_store(user_message, result.content, result.provider_used)

    return result


_TOOL_MENTION_RE = re.compile(
    # Verbe d'intention + jusqu'à ~50 caractères + nom d'outil entouré
    # éventuellement de backticks/guillemets.
    r"\b(?:appel(?:er|le|er)?|ex[ée]cute(?:r)?|utilise(?:r)?|lance(?:r)?|"
    r"d[ée]clenche(?:r)?|invoque(?:r)?|fais(?:e|ons)?|rappro[cs]he(?:r)?)\b"
    r".{0,60}?"
    r"`?(get_[a-z_]+|find_[a-z_]+|trigger_[a-z_]+|search_[a-z_]+|compare_[a-z_]+)`?",
    re.IGNORECASE | re.DOTALL,
)


def _recover_tool_from_text(
    content: str,
    user_message: str,
    ctx: ToolContext,
    provider_pref: Optional[str],
) -> Optional[TurnResult]:
    """Quand le LLM dit 'je vais appeler get_X' mais ne le fait pas, le faire
    nous-mêmes. Renvoie un TurnResult complet ou None si pas de recouvrement
    possible."""
    m = _TOOL_MENTION_RE.search(content)
    if not m:
        return None
    tool_name = m.group(1)
    tool = get_tool(tool_name)
    if tool is None:
        return None
    if tool.spec.requires_confirmation:
        # On ne déclenche jamais une action sans confirmation explicite.
        return None
    logger.info("Self-recovery: LLM mentionned %s without calling it, doing it now", tool_name)
    # On utilise le shortcut path qui formate proprement le résultat avec un
    # 2e appel LLM.
    return _shortcut_turn(tool_name, {}, user_message, ctx, provider_pref)


def _shortcut_turn(
    tool_name: str,
    args: dict,
    user_message: str,
    ctx: ToolContext,
    provider_pref: Optional[str],
) -> TurnResult:
    """Direct-tool-call path: bypass intent + tool loop entirely."""
    tool = get_tool(tool_name)
    if tool is None:
        # Fall through to normal flow if shortcut points to unknown tool.
        logger.warning("Shortcut targeted unknown tool %s", tool_name)
        return TurnResult(content="", intent="data", provider_used="none",
                          shortcut_used=tool_name)

    if tool.spec.requires_confirmation:
        # Treat as a pending action — never auto-execute writes from a shortcut.
        return TurnResult(
            content=f"Vous avez demandé `{tool_name}`. Voulez-vous confirmer ?",
            intent="action",
            provider_used="none",
            pending_action={"tool_name": tool_name, "tool_args": args,
                            "description": tool.spec.description},
            shortcut_used=tool_name,
        )

    result = execute_tool(tool_name, args, ctx)

    # Deterministic rendering — for list-shaped results, build the answer
    # ourselves instead of letting the (unreliable) local model transcribe a
    # JSON table. Guarantees faithful names/numbers and skips the LLM call.
    if result.ok:
        rendered = render_result(tool_name, result.data)
        if rendered:
            logger.info("shortcut %s rendered deterministically", tool_name)
            sug = generate_suggestions(tool_name, result.data)
            return TurnResult(
                content=rendered,
                intent="data",
                provider_used="deterministic",
                tool_calls=[{"tool": tool_name, "args": args, "ok": True,
                             "error": None}],
                usage=ChatUsage(),
                shortcut_used=tool_name,
                tool_data=[result.data],
                numbers_verified=True,
                suggestions=sug,
            )

    # Truncate the tool payload before feeding to the LLM so a huge list
    # (e.g. 200 low-stock products) doesn't blow the context window.
    payload = result.to_payload()
    MAX_PAYLOAD_CHARS = int(os.getenv("CHAT_TOOL_PAYLOAD_MAX_CHARS", "8000"))
    if len(payload) > MAX_PAYLOAD_CHARS:
        payload = payload[:MAX_PAYLOAD_CHARS] + (
            f"\n\n[Résultat tronqué — {len(result.to_payload()) - MAX_PAYLOAD_CHARS} "
            f"caractères supplémentaires omis. Synthétise sur l'extrait visible.]"
        )

    # Hand the data + a one-line instruction to the LLM so prose is in style.
    summary_msgs = [
        Message(role="system", content=(
            "Tu es l'assistant StockS. L'utilisateur a posé une question pour laquelle "
            "j'ai déjà appelé l'outil approprié — synthétise simplement le résultat en "
            "français de manière concise et utile (chiffres en gras, listes à puces si "
            "pertinent). Pas d'appel d'outil nécessaire."
        )),
        Message(role="user", content=user_message),
        Message(role="tool", name=tool_name, tool_call_id=f"shortcut_{tool_name}",
                content=payload),
    ]
    try:
        resp = chat_with_fallback(provider_pref or "auto", summary_msgs, tools=None)
        content = resp.content or ""
        provider_used = resp.provider
        usage = resp.usage
    except LLMUnavailableError as e:
        # Even without an LLM we can render something useful.
        content = f"Résultat brut de `{tool_name}` :\n```json\n{result.to_payload()}\n```"
        provider_used = "none"
        usage = ChatUsage()
        logger.warning("Shortcut LLM call failed: %s — returning raw result", e)

    return TurnResult(
        content=content,
        intent="data",
        provider_used=provider_used,
        tool_calls=[{"tool": tool_name, "args": args, "ok": result.ok,
                     "error": result.error}],
        usage=usage,
        shortcut_used=tool_name,
        tool_data=[result.data] if result.ok else [],
    )


def _tool_loop(
    *,
    messages: list[Message],
    tools: list[ToolSpec],
    ctx: ToolContext,
    provider_pref: Optional[str],
    intent: Intent,
) -> TurnResult:
    total_usage = ChatUsage()
    tool_calls_log: list[dict] = []
    tool_data_collected: list = []
    pending_action: Optional[dict] = None
    provider_used = ""
    iteration = 0
    final_content = ""

    try:
        provider = get_provider(provider_pref or "auto")
        provider_used = provider.name
    except LLMUnavailableError as e:
        return TurnResult(
            content=(
                "Désolé, aucun fournisseur LLM n'est configuré. "
                "Vérifiez MISTRAL_API_KEY, GROQ_API_KEY ou la présence du modèle local. "
                f"Détail : {e}"
            ),
            intent=intent,
            provider_used="none",
        )

    while iteration < MAX_TOOL_ITERATIONS + 1:
        try:
            resp: ChatResponse = chat_with_fallback(
                provider_pref or provider.name,
                messages,
                tools=tools if tools else None,
            )
        except LLMUnavailableError as e:
            return TurnResult(
                content=f"Erreur LLM : {e}", intent=intent,
                provider_used=provider_used or "none",
                usage=total_usage, tool_calls=tool_calls_log,
            )

        provider_used = resp.provider
        total_usage.tokens_in += resp.usage.tokens_in
        total_usage.tokens_out += resp.usage.tokens_out
        total_usage.latency_ms += resp.usage.latency_ms

        if not resp.tool_calls:
            final_content = resp.content
            break

        messages.append(Message(role="assistant", content=resp.content or "",
                                tool_calls=resp.tool_calls))

        if iteration >= MAX_TOOL_ITERATIONS:
            final_content = (
                resp.content
                or "J'ai atteint la limite d'appels d'outils pour ce tour. "
                   "Reformule ta question si besoin."
            )
            break

        for tc in resp.tool_calls:
            tool = get_tool(tc.name)

            if tool and tool.spec.requires_confirmation:
                pending_action = {
                    "tool_name": tc.name, "tool_args": tc.arguments,
                    "description": tool.spec.description,
                }
                messages.append(Message(
                    role="tool", name=tc.name, tool_call_id=tc.id,
                    content='{"status":"pending_confirmation","note":"Action en attente"}',
                ))
                tool_calls_log.append({"tool": tc.name, "args": tc.arguments,
                                       "result": "pending_confirmation"})
                continue

            result = execute_tool(tc.name, tc.arguments, ctx)
            messages.append(Message(role="tool", name=tc.name,
                                    tool_call_id=tc.id, content=result.to_payload()))
            tool_calls_log.append({"tool": tc.name, "args": tc.arguments,
                                   "ok": result.ok, "error": result.error})
            if result.ok:
                tool_data_collected.append(result.data)

        if pending_action is not None:
            messages.append(Message(role="system", content=(
                "Une action nécessite une confirmation. Demande clairement à l'utilisateur "
                "de confirmer ou d'annuler en français, sans appeler d'autre outil."
            )))
            try:
                final_resp = chat_with_fallback(provider_pref or provider_used,
                                                messages, tools=None)
                final_content = final_resp.content or "Voulez-vous confirmer cette action ? (oui / non)"
                provider_used = final_resp.provider
                total_usage.tokens_in += final_resp.usage.tokens_in
                total_usage.tokens_out += final_resp.usage.tokens_out
                total_usage.latency_ms += final_resp.usage.latency_ms
            except LLMUnavailableError:
                final_content = "Voulez-vous confirmer cette action ? (oui / non)"
            break

        iteration += 1

    return TurnResult(
        content=final_content, intent=intent, provider_used=provider_used,
        tool_calls=tool_calls_log, pending_action=pending_action, usage=total_usage,
        tool_data=tool_data_collected,
    )
