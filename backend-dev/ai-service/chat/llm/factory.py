"""Provider factory + auto-fallback chain.

Singletons are kept so the local model is loaded only once.
"""

from __future__ import annotations

import os
import threading
from typing import Iterator, Literal

from utils.logger import get_logger

from ..types import ChatResponse, Message, StreamChunk, ToolSpec
from .base import LLMProvider
from .groq_provider import GroqProvider
from .local_provider import LocalLLMProvider
from .mistral_provider import MistralProvider


ProviderName = Literal["mistral", "groq", "local", "auto"]

logger = get_logger("chat.llm")

_INSTANCES: dict[str, LLMProvider] = {}
_LOCK = threading.Lock()


# Order in which `auto` mode tries providers (cheapest / most reliable first).
_AUTO_CHAIN: list[str] = ["mistral", "groq", "local"]


class LLMUnavailableError(RuntimeError):
    """Raised when no configured provider can serve a request."""


def _instance(name: str) -> LLMProvider:
    with _LOCK:
        if name in _INSTANCES:
            return _INSTANCES[name]
        if name == "mistral":
            inst: LLMProvider = MistralProvider()
        elif name == "groq":
            inst = GroqProvider()
        elif name == "local":
            inst = LocalLLMProvider()
        else:
            raise ValueError(f"Unknown provider: {name}")
        _INSTANCES[name] = inst
        return inst


def list_providers() -> list[dict]:
    """Snapshot of provider availability — used by /llm/health."""
    out = []
    for name in ("mistral", "groq", "local"):
        try:
            avail = _instance(name).is_available()
        except Exception as e:
            avail = False
            err = str(e)
        else:
            err = None
        out.append({"name": name, "available": avail, "error": err})
    return out


def provider_health() -> dict:
    """Detailed health: which providers respond, default selection, model names."""
    return {
        "default": _resolve_default(),
        "providers": list_providers(),
    }


def _resolve_default() -> str:
    requested = (os.getenv("LLM_PROVIDER") or "auto").lower()
    if requested in ("mistral", "groq", "local"):
        return requested
    return "auto"


def get_provider(preference: ProviderName | None = None) -> LLMProvider:
    """Return a usable provider, honoring preference but falling back if needed.

    - If preference is a concrete name and available → return it.
    - If preference is `auto` (or None) → walk the chain.
    - Raises LLMUnavailableError if nothing usable is configured.
    """
    pref = (preference or _resolve_default()).lower()

    if pref in ("mistral", "groq", "local"):
        inst = _instance(pref)
        if inst.is_available():
            return inst
        logger.warning("Requested provider %s not available, falling back to auto", pref)

    # Auto chain
    last_err: str | None = None
    for name in _AUTO_CHAIN:
        inst = _instance(name)
        if inst.is_available():
            return inst
        last_err = f"{name} unavailable"
    raise LLMUnavailableError(
        f"No LLM provider configured. Set MISTRAL_API_KEY, GROQ_API_KEY, "
        f"or provide the local model at {os.getenv('LOCAL_LLM_MODEL_PATH', '')}. "
        f"({last_err})"
    )


def chat_with_fallback(
    preference: ProviderName | None,
    messages: list[Message],
    tools: list[ToolSpec] | None = None,
    max_tokens: int = 1024,
    temperature: float = 0.3,
) -> ChatResponse:
    """Try the preferred provider; on failure walk the auto chain.

    Use this when the caller wants resilience against rate-limits / network errors.
    """
    pref = (preference or _resolve_default()).lower()
    chain: list[str]
    if pref in ("mistral", "groq", "local"):
        # Preferred first, then everything else as fallback.
        chain = [pref] + [n for n in _AUTO_CHAIN if n != pref]
    else:
        chain = list(_AUTO_CHAIN)

    last_err: Exception | None = None
    for name in chain:
        inst = _instance(name)
        if not inst.is_available():
            continue
        try:
            return inst.chat(messages, tools=tools, max_tokens=max_tokens, temperature=temperature)
        except Exception as e:  # SDK errors, rate limits, network — try next
            logger.warning("Provider %s failed: %s — falling back", name, e)
            last_err = e
    raise LLMUnavailableError(
        f"All configured providers failed. Last error: {last_err}"
    )
