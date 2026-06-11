"""Shared dataclasses used across the chat module.

Kept provider-agnostic on purpose: every LLM adapter converts to/from these
shapes so the agent never depends on a specific SDK.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Iterator, Literal, Optional


Role = Literal["system", "user", "assistant", "tool"]


@dataclass
class Message:
    role: Role
    content: str
    # Set on assistant messages that requested tool calls.
    tool_calls: list[ToolCall] | None = None
    # Set on role="tool" messages: which call this answers, and the tool name.
    tool_call_id: Optional[str] = None
    name: Optional[str] = None

    def to_dict(self) -> dict[str, Any]:
        d: dict[str, Any] = {"role": self.role, "content": self.content}
        if self.tool_calls:
            d["tool_calls"] = [tc.to_dict() for tc in self.tool_calls]
        if self.tool_call_id:
            d["tool_call_id"] = self.tool_call_id
        if self.name:
            d["name"] = self.name
        return d


@dataclass
class ToolCall:
    id: str
    name: str
    arguments: dict[str, Any]

    def to_dict(self) -> dict[str, Any]:
        import json
        return {
            "id": self.id,
            "type": "function",
            "function": {"name": self.name, "arguments": json.dumps(self.arguments)},
        }


@dataclass
class ToolParam:
    """Single parameter on a tool. Kept simple — just the JSON-schema bits we use."""
    name: str
    type: str  # "string" | "integer" | "number" | "boolean"
    description: str
    required: bool = False
    enum: Optional[list[Any]] = None


@dataclass
class ToolSpec:
    """Description of a tool the LLM may invoke."""
    name: str
    description: str
    params: list[ToolParam] = field(default_factory=list)
    # Write-actions require a user confirmation before being executed.
    requires_confirmation: bool = False

    def to_openai_schema(self) -> dict[str, Any]:
        """Convert to the OpenAI/Mistral/Groq function-calling schema."""
        properties: dict[str, Any] = {}
        required: list[str] = []
        for p in self.params:
            prop: dict[str, Any] = {"type": p.type, "description": p.description}
            if p.enum is not None:
                prop["enum"] = p.enum
            properties[p.name] = prop
            if p.required:
                required.append(p.name)

        return {
            "type": "function",
            "function": {
                "name": self.name,
                "description": self.description,
                "parameters": {
                    "type": "object",
                    "properties": properties,
                    "required": required,
                },
            },
        }


@dataclass
class ChatUsage:
    tokens_in: int = 0
    tokens_out: int = 0
    latency_ms: int = 0


@dataclass
class ChatResponse:
    """A single (non-streaming) LLM response."""
    content: str
    tool_calls: list[ToolCall] | None
    usage: ChatUsage
    provider: str
    finish_reason: str = "stop"


@dataclass
class StreamChunk:
    """One chunk of a streamed response."""
    kind: Literal["delta", "tool_call", "done", "error"]
    content: Optional[str] = None
    tool_call: Optional[ToolCall] = None
    error: Optional[str] = None
    usage: Optional[ChatUsage] = None


# Forward refs cleanup
Message.__annotations__["tool_calls"] = "list[ToolCall] | None"
