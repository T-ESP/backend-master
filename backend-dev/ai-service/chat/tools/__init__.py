"""Tool catalog the agent can call.

Read tools wrap GET endpoints on the Rust API; write tools wrap POST/PUT and
require an explicit user confirmation before being executed.
"""

from .registry import (
    ToolContext,
    ToolError,
    ToolResult,
    catalog,
    execute_tool,
    get_tool,
    tool_specs,
)

__all__ = [
    "ToolContext",
    "ToolError",
    "ToolResult",
    "catalog",
    "execute_tool",
    "get_tool",
    "tool_specs",
]
