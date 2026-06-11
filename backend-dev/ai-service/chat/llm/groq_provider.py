"""Groq adapter (free tier, OpenAI-compatible API)."""

from __future__ import annotations

import json
import os
import time
from typing import Any

from ..types import ChatResponse, ChatUsage, Message, ToolCall, ToolSpec
from .base import LLMProvider


DEFAULT_MODEL = os.getenv("GROQ_MODEL", "llama-3.3-70b-versatile")


class GroqProvider(LLMProvider):
    name = "groq"
    supports_tools = True

    def __init__(self) -> None:
        self._client = None
        self._api_key = os.getenv("GROQ_API_KEY", "").strip()
        self._model = DEFAULT_MODEL

    def is_available(self) -> bool:
        return bool(self._api_key)

    def ensure_loaded(self) -> None:
        if self._client is not None:
            return
        if not self._api_key:
            raise RuntimeError("GROQ_API_KEY not set")
        from groq import Groq  # type: ignore
        self._client = Groq(api_key=self._api_key)

    def chat(
        self,
        messages: list[Message],
        tools: list[ToolSpec] | None = None,
        max_tokens: int = 1024,
        temperature: float = 0.3,
    ) -> ChatResponse:
        self.ensure_loaded()
        assert self._client is not None

        api_messages = [_to_groq_msg(m) for m in messages]
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

        resp = self._client.chat.completions.create(**kwargs)
        latency_ms = int((time.time() - start) * 1000)

        choice = resp.choices[0]
        content = choice.message.content or ""

        tool_calls: list[ToolCall] | None = None
        raw_tcs = getattr(choice.message, "tool_calls", None)
        if raw_tcs:
            tool_calls = []
            for tc in raw_tcs:
                args_raw = tc.function.arguments
                try:
                    args = json.loads(args_raw) if args_raw else {}
                except (json.JSONDecodeError, TypeError):
                    args = {}
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


def _to_groq_msg(m: Message) -> dict[str, Any]:
    if m.role == "tool":
        return {
            "role": "tool",
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
