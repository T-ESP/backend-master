"""Markdown-aware chunker.

Strategy:
1. Split by `##` and `###` headings to keep semantic boundaries.
2. Inside a section, if it exceeds ~400 tokens (≈1600 chars), break into
   overlapping windows.
3. Each chunk carries its heading path so retrieved snippets are self-explanatory.

We use a simple char-based proxy for tokens (1 token ≈ 4 chars for FR/EN);
exact token counts don't matter — the goal is reasonable chunk sizes.
"""

from __future__ import annotations

import re
from dataclasses import dataclass
from typing import List


_HEADING_RE = re.compile(r"^(#{1,6})\s+(.+?)\s*$", re.MULTILINE)
TARGET_CHARS = 1600   # ≈400 tokens
OVERLAP_CHARS = 200


@dataclass
class Chunk:
    heading: str
    content: str
    chunk_index: int
    token_count: int  # rough estimate


def chunk_markdown(text: str) -> List[Chunk]:
    """Split a markdown document into context-preserving chunks."""
    sections = _split_by_heading(text)
    chunks: list[Chunk] = []
    idx = 0
    for heading, body in sections:
        body = body.strip()
        if not body:
            continue
        if len(body) <= TARGET_CHARS:
            chunks.append(Chunk(
                heading=heading,
                content=body,
                chunk_index=idx,
                token_count=_estimate_tokens(body),
            ))
            idx += 1
            continue
        # Split long sections into overlapping windows.
        for piece in _windowed(body, TARGET_CHARS, OVERLAP_CHARS):
            chunks.append(Chunk(
                heading=heading,
                content=piece,
                chunk_index=idx,
                token_count=_estimate_tokens(piece),
            ))
            idx += 1
    return chunks


def _split_by_heading(text: str) -> List[tuple[str, str]]:
    """Return list of (heading_path, body_text) tuples.

    heading_path looks like 'Top H1 > Subheading' so retrieved chunks have
    locator context.
    """
    lines = text.split("\n")
    sections: list[tuple[str, str]] = []
    stack: list[str] = []
    current_lines: list[str] = []

    def flush() -> None:
        if current_lines:
            sections.append((" > ".join(stack) if stack else "", "\n".join(current_lines)))
            current_lines.clear()

    for line in lines:
        m = _HEADING_RE.match(line)
        if m:
            level = len(m.group(1))
            title = m.group(2).strip()
            flush()
            # Pop stack to current level - 1
            while len(stack) >= level:
                stack.pop()
            stack.append(title)
        else:
            current_lines.append(line)
    flush()

    if not sections:
        # No headings — treat the whole doc as one section.
        return [("", text)]
    return sections


def _windowed(text: str, size: int, overlap: int) -> List[str]:
    """Yield overlapping char windows. Tries to break on paragraph boundaries."""
    if len(text) <= size:
        return [text]

    chunks: list[str] = []
    start = 0
    n = len(text)
    while start < n:
        end = min(start + size, n)
        if end < n:
            # Look back for a clean break: paragraph > sentence > word.
            break_at = text.rfind("\n\n", start + size // 2, end)
            if break_at == -1:
                break_at = text.rfind(". ", start + size // 2, end)
            if break_at == -1:
                break_at = text.rfind(" ", start + size // 2, end)
            if break_at != -1 and break_at > start:
                end = break_at
        chunks.append(text[start:end].strip())
        if end >= n:
            break
        start = max(end - overlap, start + 1)
    return [c for c in chunks if c]


def _estimate_tokens(text: str) -> int:
    """Rough token count: ≈4 chars/token for FR+EN."""
    return max(1, len(text) // 4)
