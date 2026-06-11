"""GBNF grammars for llama.cpp.

Tiny models (~1-3B) often fumble structured tool-call JSON. llama.cpp lets us
constrain the *decoder* with a grammar, so output that violates the format
becomes literally impossible. The model can either emit free-form text OR a
valid `<tool_call>{...}</tool_call>` block — never garbage in between.

We keep grammars conservative (string values only, no nested arrays) because
small models drift on complex schemas and our tools accept simple primitives.
"""

from __future__ import annotations

from typing import Iterable

from ..types import ToolSpec


# Base GBNF skeleton: prose OR a single tool call.
# `root ::= prose | call` — model picks one of two paths at start.
# `prose ::= [^<]*` — any characters except `<` (avoids accidental tool tag).
# `call ::= "<tool_call>" obj "</tool_call>"` — strict envelope.

_BASE_GRAMMAR = r"""
root      ::= prose | call
prose     ::= [^<]+
call      ::= "<tool_call>" ws? "{" ws? "\"name\"" ws? ":" ws? string ws? "," ws? "\"arguments\"" ws? ":" ws? args ws? "}" ws? "</tool_call>"
args      ::= "{" ws? (kv (ws? "," ws? kv)*)? ws? "}"
kv        ::= string ws? ":" ws? value
value     ::= string | number | "true" | "false" | "null"
string    ::= "\"" char* "\""
char      ::= [^"\\] | "\\" ["\\/bfnrt]
number    ::= "-"? ("0" | [1-9] [0-9]*) ("." [0-9]+)? ([eE] [-+]? [0-9]+)?
ws        ::= [ \t\n\r]+
"""


def build_grammar(tools: Iterable[ToolSpec] | None = None) -> str:
    """Return a GBNF source string suitable for llama.cpp's `grammar=` argument.

    `tools` is unused in the current schema (we accept any object as `arguments`),
    but the param is kept so we can later restrict to known tool names if needed.
    """
    _ = tools
    return _BASE_GRAMMAR


def build_strict_grammar(tools: list[ToolSpec]) -> str:
    """Stricter variant: limits the `name` field to actual registered tools.

    Use when you want the model to physically be unable to invent a tool name.
    Slightly slower decode (more constraints to check) but bulletproof.
    """
    if not tools:
        return _BASE_GRAMMAR

    names_alt = " | ".join(f'"\\"{t.name}\\""' for t in tools)
    return r"""
root        ::= prose | call
prose       ::= [^<]+
call        ::= "<tool_call>" ws? "{" ws? "\"name\"" ws? ":" ws? tool_name ws? "," ws? "\"arguments\"" ws? ":" ws? args ws? "}" ws? "</tool_call>"
tool_name   ::= """ + names_alt + r"""
args        ::= "{" ws? (kv (ws? "," ws? kv)*)? ws? "}"
kv          ::= string ws? ":" ws? value
value       ::= string | number | "true" | "false" | "null"
string      ::= "\"" char* "\""
char        ::= [^"\\] | "\\" ["\\/bfnrt]
number      ::= "-"? ("0" | [1-9] [0-9]*) ("." [0-9]+)? ([eE] [-+]? [0-9]+)?
ws          ::= [ \t\n\r]+
"""
