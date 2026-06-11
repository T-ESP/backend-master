#!/usr/bin/env bash
# scripts/chat_smoke_test.sh
#
# Quick end-to-end check of the chatbot stack. Assumes `docker compose up -d`
# is already running. Logs in as the seed admin, hits each chat endpoint, and
# prints a tiny report.
#
# Requires: jq, curl.

set -euo pipefail

API="${API:-http://localhost:8090}"
AI="${AI:-http://localhost:8001}"
EMAIL="${EMAIL:-admin@example.com}"
PASSWORD="${PASSWORD:-adminpass}"

bold() { printf '\n\033[1m== %s ==\033[0m\n' "$*"; }
ok()   { printf '\033[32m  OK   %s\033[0m\n' "$*"; }
fail() { printf '\033[31m  FAIL %s\033[0m\n' "$*"; }

bold "1. Ping ai-service health"
if curl -fsS "$AI/ai/health" >/dev/null; then ok "ai-service alive"; else fail "ai-service down"; exit 1; fi

bold "2. Ping Rust API health"
if curl -fsS "$API/health" >/dev/null; then ok "stocks_api alive"; else fail "stocks_api down"; exit 1; fi

bold "3. Login"
TOKEN=$(curl -fsS -X POST "$API/auth/login" \
  -H 'content-type: application/json' \
  -d "{\"email\":\"$EMAIL\",\"password\":\"$PASSWORD\"}" | jq -r '.data.token')
[[ -n "$TOKEN" && "$TOKEN" != "null" ]] && ok "got JWT" || { fail "no JWT"; exit 1; }
H="authorization: Bearer $TOKEN"

bold "4. RAG stats (direct ai-service)"
curl -fsS "$AI/rag/stats" | jq

bold "5. LLM provider health (via Rust admin)"
curl -fsS "$API/admin/chat/providers" -H "$H" | jq

bold "6. Create chat session"
SID=$(curl -fsS -X POST "$API/chat/sessions" -H "$H" \
  -H 'content-type: application/json' \
  -d '{"title":"Smoke test"}' | jq -r '.data.session_id')
[[ -n "$SID" && "$SID" != "null" ]] && ok "session $SID" || { fail "could not create session"; exit 1; }

bold "7. Send a doc-style question (RAG path) — first call"
curl -fsS -X POST "$API/chat/sessions/$SID/messages" -H "$H" \
  -H 'content-type: application/json' \
  -d '{"content":"Que signifie la classification ABC-XYZ ?"}' \
  | jq '{provider: .data.provider_used, intent: .data.intent, cached: .data.cached, citations: (.data.citations | length), content: (.data.assistant_message.content[:200])}'

bold "7b. Same question again — should be CACHED (cached:true, latency_ms<100)"
curl -fsS -X POST "$API/chat/sessions/$SID/messages" -H "$H" \
  -H 'content-type: application/json' \
  -d '{"content":"Que signifie la classification ABC-XYZ ?"}' \
  | jq '{cached: .data.cached, latency_ms: .data.usage.latency_ms, content: (.data.assistant_message.content[:80])}'

bold "8. Send a data-style question (shortcut path — should fire get_low_stock immediately)"
curl -fsS -X POST "$API/chat/sessions/$SID/messages" -H "$H" \
  -H 'content-type: application/json' \
  -d '{"content":"Quels produits sont en stock bas ?"}' \
  | jq '{provider: .data.provider_used, intent: .data.intent, shortcut: .data.shortcut_used, content: (.data.assistant_message.content[:200])}'

bold "8b. Top products query (shortcut path)"
curl -fsS -X POST "$API/chat/sessions/$SID/messages" -H "$H" \
  -H 'content-type: application/json' \
  -d '{"content":"Top 5 produits par chiffre d'\''affaires ?"}' \
  | jq '{shortcut: .data.shortcut_used, content: (.data.assistant_message.content[:200])}'

bold "9. Provider-specific request (force local)"
curl -fsS -X POST "$API/chat/sessions/$SID/messages" -H "$H" \
  -H 'content-type: application/json' \
  -d '{"content":"Salut !","provider":"local"}' \
  | jq '{provider: .data.provider_used, content: .data.assistant_message.content}'

bold "10. List sessions"
curl -fsS "$API/chat/sessions" -H "$H" | jq '.data | length' | xargs -I{} echo "  sessions visible: {}"

bold "11. Export session as markdown"
curl -fsS "$API/chat/sessions/$SID/export?format=markdown" -H "$H" -o "/tmp/chat-$SID.md"
test -s "/tmp/chat-$SID.md" && ok "exported to /tmp/chat-$SID.md ($(wc -c < /tmp/chat-$SID.md) bytes)" || fail "export empty"

ok "All checks ran. Inspect output above; if every block has content, the bot is alive."
