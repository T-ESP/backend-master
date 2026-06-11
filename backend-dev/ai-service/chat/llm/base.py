"""Abstract LLM provider interface."""

from __future__ import annotations

from abc import ABC, abstractmethod
from typing import Iterator

from ..types import ChatResponse, Message, StreamChunk, ToolSpec


class LLMProvider(ABC):
    """All providers must conform to this contract.

    Implementations should be cheap to construct (no network/model load in
    __init__) so the factory can probe availability without paying startup cost.
    Heavy work (model loading) goes in `ensure_loaded()`.
    """

    name: str = ""
    supports_tools: bool = True

    @abstractmethod
    def is_available(self) -> bool:
        """Quick check (env var, file presence) — must not do network I/O."""
        ...

    def ensure_loaded(self) -> None:
        """Lazy resource initialization. Default: no-op."""
        pass

    def unload(self) -> None:
        """Free any heavy resources (used by scheduler before cron runs)."""
        pass

    @abstractmethod
    def chat(
        self,
        messages: list[Message],
        tools: list[ToolSpec] | None = None,
        max_tokens: int = 1024,
        temperature: float = 0.3,
    ) -> ChatResponse:
        """Non-streaming chat completion."""
        ...

    def chat_stream(
        self,
        messages: list[Message],
        tools: list[ToolSpec] | None = None,
        max_tokens: int = 1024,
        temperature: float = 0.3,
    ) -> Iterator[StreamChunk]:
        """Streaming chat completion. Default: synthesize stream from non-streaming chat."""
        try:
            resp = self.chat(messages, tools=tools, max_tokens=max_tokens, temperature=temperature)
            if resp.content:
                yield StreamChunk(kind="delta", content=resp.content)
            if resp.tool_calls:
                for tc in resp.tool_calls:
                    yield StreamChunk(kind="tool_call", tool_call=tc)
            yield StreamChunk(kind="done", usage=resp.usage)
        except Exception as e:  # pragma: no cover — provider-level safety net
            yield StreamChunk(kind="error", error=str(e))
