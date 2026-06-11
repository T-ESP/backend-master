#!/usr/bin/env python3
"""REPL interactif pour discuter avec le chatbot StockS.

Usage :
    python scripts/chat_repl.py
    python scripts/chat_repl.py --provider groq
    python scripts/chat_repl.py --api http://localhost:8090 --email admin@example.com

Commandes spéciales (dans le REPL) :
    /quit         — quitter
    /new          — nouvelle session
    /provider X   — changer le provider (auto | mistral | groq | local)
    /history      — voir l'historique de la session courante
    /export       — exporter la conversation en markdown
"""

from __future__ import annotations

import argparse
import getpass
import json
import sys
import time
from pathlib import Path

import urllib.request
import urllib.error

# Force UTF-8 stdout/stderr on Windows so accented French + emojis render.
if hasattr(sys.stdout, "reconfigure"):
    try:
        sys.stdout.reconfigure(encoding="utf-8")
        sys.stderr.reconfigure(encoding="utf-8")
    except Exception:
        pass


def http(method: str, url: str, *, token: str | None = None,
         body: dict | None = None) -> dict:
    headers = {"Content-Type": "application/json"}
    if token:
        headers["Authorization"] = f"Bearer {token}"
    data = json.dumps(body).encode() if body is not None else None
    req = urllib.request.Request(url, data=data, headers=headers, method=method)
    try:
        with urllib.request.urlopen(req, timeout=600) as resp:
            return json.loads(resp.read())
    except urllib.error.HTTPError as e:
        print(f"\n  ⚠ HTTP {e.code}: {e.read().decode()[:300]}")
        return {}
    except Exception as e:
        print(f"\n  ⚠ Erreur réseau : {e}")
        return {}


def login(api: str, email: str, password: str) -> str | None:
    out = http("POST", f"{api}/auth/login", body={"email": email, "password": password})
    token = out.get("data", {}).get("token") if out else None
    if not token:
        print("  ⚠ Login impossible. Vérifiez identifiants + que la stack tourne.")
    return token


def create_session(api: str, token: str, provider: str) -> str | None:
    out = http("POST", f"{api}/chat/sessions", token=token,
               body={"title": f"REPL {time.strftime('%H:%M')}", "provider": provider})
    return out.get("data", {}).get("session_id") if out else None


def send_message(api: str, token: str, session_id: str, content: str,
                 provider: str | None) -> dict:
    body = {"content": content}
    if provider:
        body["provider"] = provider
    return http("POST", f"{api}/chat/sessions/{session_id}/messages",
                token=token, body=body)


def show_history(api: str, token: str, session_id: str) -> None:
    out = http("GET", f"{api}/chat/sessions/{session_id}", token=token)
    msgs = out.get("data", {}).get("messages", []) if out else []
    for m in msgs:
        role = m.get("role", "?")
        content = (m.get("content") or "")[:200]
        prefix = {"user": "🧑 ", "assistant": "🤖 ", "tool": "🔧 ",
                  "system": "⚙️  "}.get(role, "? ")
        print(f"  {prefix}{content}")


def export_session(api: str, token: str, session_id: str) -> None:
    req = urllib.request.Request(
        f"{api}/chat/sessions/{session_id}/export?format=markdown",
        headers={"Authorization": f"Bearer {token}"},
    )
    try:
        with urllib.request.urlopen(req, timeout=30) as resp:
            md = resp.read().decode()
        out_path = Path(f"chat-{session_id}.md")
        out_path.write_text(md, encoding="utf-8")
        print(f"  ✓ Conversation exportée vers {out_path} ({len(md)} octets)")
    except Exception as e:
        print(f"  ⚠ Export impossible : {e}")


def repl(api: str, email: str, password: str, default_provider: str) -> None:
    print("=" * 60)
    print(f"  Chat StockS — REPL")
    print(f"  API: {api}")
    print(f"  Provider par défaut: {default_provider}")
    print(f"  Commandes : /quit  /new  /provider X  /history  /export")
    print("=" * 60)

    token = login(api, email, password)
    if not token:
        sys.exit(1)
    print(f"  ✓ Connecté en tant que {email}")

    provider = default_provider
    sid = create_session(api, token, provider)
    if not sid:
        sys.exit(1)
    print(f"  ✓ Session créée: {sid[:8]}…\n")

    while True:
        try:
            line = input("\n🧑 vous : ").strip()
        except (EOFError, KeyboardInterrupt):
            print("\n  Bye 👋")
            break

        if not line:
            continue

        if line == "/quit":
            print("  Bye 👋")
            break
        if line == "/new":
            sid = create_session(api, token, provider)
            print(f"  ✓ Nouvelle session: {sid[:8]}…")
            continue
        if line.startswith("/provider "):
            provider = line.split(" ", 1)[1].strip()
            print(f"  ✓ Provider: {provider}")
            continue
        if line == "/history":
            show_history(api, token, sid)
            continue
        if line == "/export":
            export_session(api, token, sid)
            continue

        t0 = time.time()
        print("  🤖 (génération…)", end="", flush=True)
        out = send_message(api, token, sid, line, provider)
        elapsed = time.time() - t0
        print("\r" + " " * 30 + "\r", end="")

        data = out.get("data") if out else None
        if not data:
            continue

        msg = data.get("assistant_message", {})
        content = msg.get("content", "")

        # Métadonnées
        meta_bits = []
        meta_bits.append(f"provider={data.get('provider_used','?')}")
        meta_bits.append(f"intent={data.get('intent','?')}")
        if data.get("shortcut_used"):
            meta_bits.append(f"shortcut={data['shortcut_used']}")
        if data.get("cached"):
            meta_bits.append("cached=YES")
        usage = data.get("usage", {})
        if usage.get("latency_ms"):
            meta_bits.append(f"{usage['latency_ms']/1000:.1f}s")

        print(f"🤖 ({', '.join(meta_bits)}) :")
        print(content)

        # Citations RAG
        cits = data.get("citations") or []
        if cits:
            print("\n  📚 Sources :")
            for c in cits[:3]:
                print(f"    - {c.get('source_path')} (similarity={c.get('similarity',0):.2f})")

        # Action en attente
        pa = data.get("pending_action")
        if pa:
            print(f"\n  ⚠ Action en attente : {pa.get('tool_name')} {pa.get('tool_args')}")
            print("  Pour confirmer/annuler, utilise Swagger UI ou /quit.")


if __name__ == "__main__":
    p = argparse.ArgumentParser()
    p.add_argument("--api", default="http://localhost:8090")
    p.add_argument("--email", default="admin@example.com")
    p.add_argument("--password", default=None,
                   help="Mot de passe (sinon admin par défaut, ou prompt si --prompt)")
    p.add_argument("--prompt", action="store_true",
                   help="Demander le mot de passe interactivement")
    p.add_argument("--provider", default="auto",
                   choices=["auto", "mistral", "groq", "local"])
    args = p.parse_args()

    if args.prompt:
        args.password = getpass.getpass("Mot de passe : ")
    elif args.password is None:
        args.password = "adminpass"

    repl(args.api, args.email, args.password, args.provider)
