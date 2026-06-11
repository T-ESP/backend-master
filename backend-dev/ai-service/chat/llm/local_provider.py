"""Local LLM adapter using llama-cpp-python.

Default model: Qwen2.5-1.5B-Instruct Q4_K_M (multilingual, ~1.0GB on disk,
~1.8GB RAM at load, runs comfortably on a 4-core 8GB VPS).

Tool-call strategy:
- Small models (~1-3B) struggle with native function-calling formats.
- We give the model a `<tool_call>{...}</tool_call>` convention via the system
  prompt and parse the tags out of plain text. This is reliable and works with
  any model llama.cpp can run.
"""

from __future__ import annotations

import json
import os
import re
import time
import threading
from pathlib import Path
from typing import Any, Optional

from ..types import ChatResponse, ChatUsage, Message, ToolCall, ToolSpec
from .base import LLMProvider


def _model_path() -> str:
    return os.getenv(
        "LOCAL_LLM_MODEL_PATH",
        "/app/llm_models/qwen2.5-1.5b-instruct-q4_k_m.gguf",
    )


def _model_url() -> str:
    return os.getenv(
        "LOCAL_LLM_MODEL_URL",
        "https://huggingface.co/Qwen/Qwen2.5-1.5B-Instruct-GGUF/resolve/main/qwen2.5-1.5b-instruct-q4_k_m.gguf",
    )


def _auto_download() -> bool:
    return os.getenv("LOCAL_LLM_AUTO_DOWNLOAD", "true").lower() == "true"


def _n_threads() -> int:
    return int(os.getenv("LOCAL_LLM_THREADS", "3"))


def _n_ctx() -> int:
    return int(os.getenv("LOCAL_LLM_CTX", "4096"))


def _use_grammar() -> bool:
    return os.getenv("LOCAL_LLM_USE_GRAMMAR", "true").lower() == "true"


def _repair_mojibake(s: str) -> str:
    """Reverse UTF-8 text that was decoded as Latin-1 (``é`` -> ``Ã©``).

    llama.cpp output can surface this double-encoding for accented characters.
    The repair is safe: a string that is already correct UTF-8 (e.g. a real
    ``é`` = U+00E9) raises on the utf-8 decode and is returned unchanged, and we
    also reject any repair that introduces replacement characters.
    """
    if not s:
        return s
    try:
        repaired = s.encode("latin-1").decode("utf-8")
    except (UnicodeEncodeError, UnicodeDecodeError):
        return s
    if "�" in repaired:
        return s
    return repaired


# Single shared lock — llama.cpp generation is single-threaded per model.
_LLAMA_LOCK = threading.Lock()


TOOL_INSTRUCTION_TEMPLATE = """Tu peux appeler les outils suivants pour obtenir des données fraîches du système de gestion de stock.

Pour appeler un outil, écris EXACTEMENT et UNIQUEMENT ce format (rien avant, rien après) :
<tool_call>{{"name": "nom_outil", "arguments": {{...}}}}</tool_call>

Outils disponibles :
{tool_descriptions}

EXEMPLES (à imiter) :

Q : Combien de produits sont en rupture critique ?
R : <tool_call>{{"name": "get_alerts", "arguments": {{"severity": "CRITICAL"}}}}</tool_call>

Q : Quels sont mes top 5 produits par chiffre d'affaires ?
R : <tool_call>{{"name": "get_top_products", "arguments": {{"metric": "revenue", "limit": 5}}}}</tool_call>

Q : Donne-moi les détails du produit 42.
R : <tool_call>{{"name": "get_product_detail", "arguments": {{"product_id": 42}}}}</tool_call>

Q : Tous les produits en stock bas ?
R : <tool_call>{{"name": "get_low_stock", "arguments": {{}}}}</tool_call>

Q : Bonjour !
R : Bonjour ! Comment puis-je vous aider ?

Règles :
- Un seul appel d'outil à la fois.
- Si la question peut être répondue sans outil (chitchat, concept), réponds directement en français.
- Après réception du résultat (message [Résultat de l'outil ...]), formule une réponse claire en français qui synthétise les chiffres pour l'utilisateur.
"""


class LocalLLMProvider(LLMProvider):
    name = "local"
    # We support tools via prompt convention, not native function calling.
    supports_tools = True

    def __init__(self) -> None:
        self._llm = None

    # ------------------------------------------------------------------
    # Lifecycle
    # ------------------------------------------------------------------

    def is_available(self) -> bool:
        """Available if model file exists OR auto-download is on (and we have a URL)."""
        if Path(_model_path()).is_file():
            return True
        return bool(_auto_download() and _model_url())

    def ensure_loaded(self) -> None:
        if self._llm is not None:
            return
        from utils.logger import get_logger
        log = get_logger("chat.llm.local")

        path = Path(_model_path())
        if not path.is_file():
            if not _auto_download():
                raise RuntimeError(
                    f"Local LLM model not found at {path} and LOCAL_LLM_AUTO_DOWNLOAD=false"
                )
            url = _model_url()
            log.info("Local LLM model not found, downloading from %s", url)
            _download_model(url, path, logger=log)

        n_threads = _n_threads()
        n_ctx = _n_ctx()
        log.info("Loading local LLM from %s (threads=%d, ctx=%d)", path, n_threads, n_ctx)
        from llama_cpp import Llama  # type: ignore

        # NB : pas de KV cache quantifié (type_k/type_v + flash_attn). Testé et
        # abandonné : flash attention sur CPU fait spinner llama.cpp (300 % CPU,
        # blocage). Le KV cache reste donc en f16, c'est le choix sûr sans GPU.
        self._llm = Llama(
            model_path=str(path),
            n_ctx=n_ctx,
            n_threads=n_threads,
            n_threads_batch=n_threads,
            n_batch=256,
            use_mmap=True,
            use_mlock=False,
            verbose=False,
        )

        # Prompt caching : réutilise l'état KV du préfixe commun (prompt système
        # + few-shot, ~800 tokens identiques à chaque tour). Économise le
        # re-traitement de ce préfixe → 5-15 s gagnées par tour sur CPU.
        if os.getenv("LOCAL_LLM_PROMPT_CACHE", "true").lower() == "true":
            try:
                from llama_cpp import LlamaRAMCache  # type: ignore
                cache_mb = int(os.getenv("LOCAL_LLM_CACHE_MB", "256"))
                self._llm.set_cache(LlamaRAMCache(capacity_bytes=cache_mb * 1024 * 1024))
                log.info("Prompt cache activé (%d MB)", cache_mb)
            except Exception as e:
                log.warning("Prompt cache indisponible (%s)", e)

        log.info("Local LLM ready.")

    def unload(self) -> None:
        """Free the model from RAM (used by scheduler before heavy cron runs)."""
        if self._llm is not None:
            self._llm = None
            import gc
            gc.collect()

    # ------------------------------------------------------------------
    # Inference
    # ------------------------------------------------------------------

    def chat(
        self,
        messages: list[Message],
        tools: list[ToolSpec] | None = None,
        max_tokens: int = 1024,
        temperature: float = 0.3,
    ) -> ChatResponse:
        self.ensure_loaded()
        assert self._llm is not None

        # If tools are available, append a tool-instruction system message
        # describing the format. Small models work much better when told what to do.
        prompt_messages = list(messages)
        if tools:
            tool_block = _build_tool_instruction(tools)
            # Splice the tool instruction *after* the existing system messages so
            # business context comes first.
            insert_idx = 0
            for i, m in enumerate(prompt_messages):
                if m.role == "system":
                    insert_idx = i + 1
            prompt_messages.insert(
                insert_idx,
                Message(role="system", content=tool_block),
            )

        api_messages = [_to_oai_msg(m) for m in prompt_messages]

        # Compile a GBNF grammar that constrains tool-call output. This makes
        # small models (~1.5-3B) reliably emit valid JSON even when their
        # natural tendency is to drift.
        gbnf = None
        if tools and _use_grammar():
            try:
                from llama_cpp import LlamaGrammar  # type: ignore
                from .grammar import build_grammar
                gbnf = LlamaGrammar.from_string(build_grammar(tools))
            except Exception as e:
                from utils.logger import get_logger
                get_logger("chat.llm.local").warning("GBNF grammar load failed (%s); proceeding without", e)
                gbnf = None

        start = time.time()
        with _LLAMA_LOCK:
            kwargs = dict(
                messages=api_messages,
                max_tokens=max_tokens,
                temperature=temperature,
                stop=["</tool_call>"],  # we let model emit our marker, then we close it
            )
            if gbnf is not None:
                kwargs["grammar"] = gbnf
            resp = self._llm.create_chat_completion(**kwargs)
        latency_ms = int((time.time() - start) * 1000)

        choice = resp["choices"][0]
        raw_content = _repair_mojibake(choice["message"].get("content") or "")
        finish = choice.get("finish_reason") or "stop"

        # If we stopped on the tool-call closer, the closing tag was eaten — restore.
        if finish == "stop" and "<tool_call>" in raw_content and "</tool_call>" not in raw_content:
            raw_content = raw_content + "</tool_call>"

        content, tool_calls = _split_tool_calls(raw_content)

        usage_block = resp.get("usage", {}) or {}
        usage = ChatUsage(
            tokens_in=int(usage_block.get("prompt_tokens", 0) or 0),
            tokens_out=int(usage_block.get("completion_tokens", 0) or 0),
            latency_ms=latency_ms,
        )

        return ChatResponse(
            content=content,
            tool_calls=tool_calls if tool_calls else None,
            usage=usage,
            provider=self.name,
            finish_reason=finish,
        )


# ----------------------------------------------------------------------
# Helpers
# ----------------------------------------------------------------------

_TOOL_CALL_RE = re.compile(r"<tool_call>\s*(\{.*?\})\s*</tool_call>", re.DOTALL)


def _split_tool_calls(text: str) -> tuple[str, list[ToolCall]]:
    """Extract <tool_call>{...}</tool_call> blocks from plain text.

    Returns (visible_content_without_tags, parsed_calls).
    """
    calls: list[ToolCall] = []
    matches = list(_TOOL_CALL_RE.finditer(text))
    if not matches:
        return text.strip(), []

    for i, m in enumerate(matches):
        try:
            payload = json.loads(m.group(1))
        except json.JSONDecodeError:
            continue
        name = payload.get("name") or payload.get("tool") or ""
        args = payload.get("arguments") or payload.get("args") or {}
        if not isinstance(args, dict):
            args = {}
        if name:
            calls.append(ToolCall(id=f"local_{i}", name=name, arguments=args))

    visible = _TOOL_CALL_RE.sub("", text).strip()
    return visible, calls


def _build_tool_instruction(tools: list[ToolSpec]) -> str:
    descs = []
    for t in tools:
        params_desc = ", ".join(
            f"{p.name} ({p.type}{', requis' if p.required else ''}): {p.description}"
            for p in t.params
        ) or "aucun paramètre"
        descs.append(f"- {t.name}: {t.description}\n  Paramètres: {params_desc}")
    return TOOL_INSTRUCTION_TEMPLATE.format(tool_descriptions="\n".join(descs))


def _to_oai_msg(m: Message) -> dict[str, Any]:
    """Convert our Message into the OpenAI-style dict that llama.cpp expects."""
    if m.role == "tool":
        # llama.cpp accepts tool messages as plain user messages tagged as such.
        # We render them as system context for maximum compatibility.
        return {
            "role": "user",
            "content": f"[Résultat de l'outil {m.name or 'tool'}]\n{m.content}",
        }
    if m.role == "assistant" and m.tool_calls:
        # Render previous assistant tool calls as text so the model sees its own history.
        rendered = m.content or ""
        for tc in m.tool_calls:
            rendered += (
                f"\n<tool_call>{json.dumps({'name': tc.name, 'arguments': tc.arguments}, ensure_ascii=False)}</tool_call>"
            )
        return {"role": "assistant", "content": rendered}
    return {"role": m.role, "content": m.content}


def _download_model(url: str, dest: Path, *, logger=None) -> None:
    """Stream a GGUF file from a URL to disk."""
    import requests
    dest.parent.mkdir(parents=True, exist_ok=True)
    tmp = dest.with_suffix(dest.suffix + ".part")
    if logger:
        logger.info("Downloading %s → %s", url, dest)
    with requests.get(url, stream=True, timeout=600) as r:
        r.raise_for_status()
        total = int(r.headers.get("Content-Length", 0))
        downloaded = 0
        last_log = 0
        with open(tmp, "wb") as f:
            for chunk in r.iter_content(chunk_size=1 << 20):  # 1 MiB
                if chunk:
                    f.write(chunk)
                    downloaded += len(chunk)
                    if logger and total and downloaded - last_log > (50 << 20):
                        last_log = downloaded
                        pct = 100.0 * downloaded / total
                        logger.info("  ... %.1f%% (%d / %d MiB)", pct,
                                    downloaded >> 20, total >> 20)
    tmp.replace(dest)
    if logger:
        logger.info("Download complete (%d MiB)", dest.stat().st_size >> 20)
