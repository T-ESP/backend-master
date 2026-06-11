"""Mistral la Plateforme adapter (free tier compatible)."""

from __future__ import annotations

import json
import os
import time
from typing import Any

from ..types import ChatResponse, ChatUsage, Message, ToolCall, ToolSpec
from .base import LLMProvider


DEFAULT_MODEL = os.getenv("MISTRAL_MODEL", "mistral-small-latest")


class MistralProvider(LLMProvider):
    name = "mistral"
    supports_tools = True

    def __init__(self) -> None:
        self._client = None
        self._api_key = os.getenv("MISTRAL_API_KEY", "").strip()
        self._model = DEFAULT_MODEL

    def is_available(self) -> bool:
        return bool(self._api_key)

    def ensure_loaded(self) -> None:
        if self._client is not None:
            return
        if not self._api_key:
            raise RuntimeError("MISTRAL_API_KEY not set")
        # Imported lazily so the service starts fine without the SDK present.
        from mistralai import Mistral  # type: ignore
        self._client = Mistral(api_key=self._api_key)

    def chat(
        self,
        messages: list[Message],
        tools: list[ToolSpec] | None = None,
        max_tokens: int = 1024,
        temperature: float = 0.3,
    ) -> ChatResponse:
        self.ensure_loaded()
        assert self._client is not None

        api_messages = [_to_mistral_msg(m) for m in messages]
        api_tools = [t.to_openai_schema() for t in tools] if tools else None

        start = time.time()
        kwargs: dict[str, Any] = {
            "model": self._model,
            "messages": api_messages,
            "max_tokens": max_tokens,
            "temperature": temperature,
        }
        if api_tools:
            kwargs["tools"] = api_tools
            kwargs["tool_choice"] = "auto"

        resp = self._client.chat.complete(**kwargs)
        latency_ms = int((time.time() - start) * 1000)

        choice = resp.choices[0]
        content = choice.message.content or ""

        tool_calls: list[ToolCall] | None = None
        raw_tcs = getattr(choice.message, "tool_calls", None)
        if raw_tcs:
            tool_calls = []
            for tc in raw_tcs:
                args_raw = tc.function.arguments
                if isinstance(args_raw, str):
                    try:
                        args = json.loads(args_raw) if args_raw else {}
                    except json.JSONDecodeError:
                        args = {}
                else:
                    args = args_raw or {}
                tool_calls.append(ToolCall(id=tc.id, name=tc.function.name, arguments=args))

        usage_obj = getattr(resp, "usage", None)
        usage = ChatUsage(
            tokens_in=getattr(usage_obj, "prompt_tokens", 0) or 0,
            tokens_out=getattr(usage_obj, "completion_tokens", 0) or 0,
            latency_ms=latency_ms,
        )
        return ChatResponse(
            content=content,
            tool_calls=tool_calls,
            usage=usage,
            provider=self.name,
            finish_reason=choice.finish_reason or "stop",
        )


def _to_mistral_msg(m: Message) -> dict[str, Any]:
    """Convert our Message → Mistral SDK message shape."""
    if m.role == "tool":
        return {
            "role": "tool",
            "name": m.name or "tool",
            "tool_call_id": m.tool_call_id or "",
            "content": m.content,
        }
    if m.role == "assistant" and m.tool_calls:
        return {
            "role": "assistant",
            "content": m.content or "",
            "tool_calls": [tc.to_dict() for tc in m.tool_calls],
        }
    return {"role": m.role, "content": m.content}
