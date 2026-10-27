#!/usr/bin/env bash
# End-to-end smoke test: boots `newera serve`, talks MCP over Streamable HTTP
# like an AI client would, and checks the change through the REST API.
#
# Usage: scripts/mcp-smoke.sh [path/to/newera]
set -euo pipefail

BIN="${1:-target/debug/newera}"
PORT="${SMOKE_PORT:-7989}"
BASE="http://127.0.0.1:${PORT}"
PROTOCOL="2025-06-18"
LOG="$(mktemp)"

"$BIN" serve --demo --addr "127.0.0.1:${PORT}" >"$LOG" 2>&1 &
PID=$!
trap 'kill "$PID" 2>/dev/null || true; rm -f "$LOG"' EXIT

for _ in $(seq 1 50); do
  curl -sf "$BASE/health" >/dev/null && break
  sleep 0.2
done
curl -sf "$BASE/health" >/dev/null || { echo "server did not start:"; cat "$LOG"; exit 1; }

SESSION=""
# Sends a JSON-RPC message and prints the `data:` payload of the SSE reply.
rpc() {
  local headers=(-H "Content-Type: application/json" -H "Accept: application/json, text/event-stream")
  [[ -n "$SESSION" ]] && headers+=(-H "Mcp-Session-Id: $SESSION" -H "MCP-Protocol-Version: $PROTOCOL")
  curl -s "${headers[@]}" -D "$LOG.headers" "$BASE/mcp" -d "$1" | sed -n 's/^data: //p' | grep -v '^$' || true
}

check() {
  local label="$1" haystack="$2" needle="$3"
  if [[ "$haystack" == *"$needle"* ]]; then
    echo "  ✓ $label"
  else
    echo "  ✗ $label — expected to find: $needle"
    echo "    got: $haystack"
    exit 1
  fi
}

echo "MCP smoke test against $BASE"

reply=$(rpc '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"'"$PROTOCOL"'","capabilities":{},"clientInfo":{"name":"smoke","version":"1"}}}')
check "initialize" "$reply" '"name":"3d-new-era-ai"'
SESSION=$(grep -i '^mcp-session-id:' "$LOG.headers" | awk '{print $2}' | tr -d '\r')
rpc '{"jsonrpc":"2.0","method":"notifications/initialized"}' >/dev/null

reply=$(rpc '{"jsonrpc":"2.0","id":2,"method":"tools/list"}')
for tool in home create update delete move edit_walls edit_home edit_background render_plan export file edit_history history catalog place layout variants levels materials cameras video plugins sessions arrange background joinery cabinet_run embed fit_roof cut_list ergonomics disciplines annotations render_3d render_photo rules model edit_model library edit_library; do
  check "tool $tool listed" "$reply" "\"name\":\"$tool\""
done

call() { rpc '{"jsonrpc":"2.0","id":9,"method":"tools/call","params":{"name":"'"$1"'","arguments":'"$2"'}}'; }
count() { curl -s "$BASE/api/home" | grep -o "\"id\":\"$1[0-9]*\"" | wc -l | tr -d ' '; }

reply=$(call create '{"walls":[{"pts":[[900,0],[1300,0],[1300,600],[900,600]],"closed":true}],"rooms":[{"name":"Cozinha","at":[1100,300]}],"dims":[{"wall":"w1"}],"labels":[{"text":"Entrada","at":[1100,-60]}]}')
check "create answers in one line" "$reply" 'ok rev=1 ids=w22,w23,w24,w25,r26,d27,t28'
check "REST sees 9 walls (5 demo + 4 new)" "$(count w)" "9"
check "room detected from walls" "$(count r)" "3"

reply=$(call home '{"detail":"summary"}')
check "summary lists the kitchen" "$reply" 'Cozinha'

reply=$(call update '{"items":[{"id":"w22","t":25},{"id":"t28","text":"Porta"}]}')
check "update several kinds" "$reply" 'ok rev=2'
reply=$(call update '{"items":[{"id":"w22","text":"x"}]}')
check "update rejects fields of other kinds" "$reply" 'does not apply'

reply=$(call edit_walls '{"action":"split","id":"w23"}')
check "edit_walls split returns the new wall" "$reply" 'ids=w29'

reply=$(call render_plan '{"w":320,"h":240}')
check "render_plan returns a PNG image" "$reply" '"mimeType":"image/png"'

reply=$(call edit_history '{"action":"undo"}')
check "undo" "$reply" 'ok rev=4'
check "undo reverted the split" "$(count w)" "9"

reply=$(call delete '{"ids":["w1","nope"]}')
check "unknown id is rejected" "$reply" 'invalid id'
check "failed delete changes nothing" "$(count w)" "9"
check "a refusal is a tool result, not a protocol error" "$reply" '"isError":true'
reply=$(call edit_history '{"action":"undo","zz":1}')
check "an unknown argument is refused by name" "$reply" 'unknown argument `zz`'
check "and inside the result" "$reply" '"isError":true'
check "the refused undo changes nothing" "$(count w)" "9"

reply=$(call catalog '{"q":"cama casal"}')
check "catalog search finds the double bed" "$reply" 'bed-double'
reply=$(call place '{"items":[{"cat":"window","wall":"w25"},{"cat":"bed-single","at":[1100,300]}]}')
check "place answers with furniture ids" "$reply" 'ids=f30,f31'
reply=$(call home '{}')
check "doors and windows report their wall" "$reply" '\"wall\":\"w25\"'
reply=$(call layout '{}')
check "layout reports issues as JSON" "$reply" '{'
reply=$(call render_plan '{"w":320,"h":240}')
check "render_plan with furniture still works" "$reply" '"mimeType":"image/png"'

reply=$(call render_3d '{"w":160,"h":120}')
check "render_3d returns a PNG image" "$reply" '"mimeType":"image/png"'
check "REST serves the 3D view" "$(curl -s -o /dev/null -w '%{content_type}' "$BASE/api/view.png?w=120&h=90")" "image/png"

check "SSE reports the revision" "$(curl -sN --max-time 1 "$BASE/api/events" | head -2 | tr '\n' ' ')" 'event: revision'
reply=$(curl -s -X POST "$BASE/api/commands" -H 'content-type: application/json' -d '{"commands":[{"op":"set_compass","compass":{"center":[0,0],"diameter":100,"north_degrees":30,"visible":true}}]}')
check "REST commands apply atomically" "$reply" '"revision"'

reply=$(call materials '{}')
check "materials lists drywall" "$reply" 'drywall-95'
reply=$(call update '{"items":[{"id":"w22","type":"drywall-95","sides":"tiles #ffffff 30"},{"id":"r26","floor_mat":"wood"}]}')
check "wall type and finishes apply" "$reply" 'ok rev='
reply=$(call home '{}')
check "wall reports its finish" "$reply" 'tiles #ffffff 30'

TMP_PROJECT="$(mktemp -d)/casa"
reply=$(call file '{"action":"save","path":"'"$TMP_PROJECT"'"}')
check "file save adds the extension" "$reply" 'casa.newera'
call file '{"action":"new"}' >/dev/null
check "file new clears" "$(count w)" "0"
reply=$(call file '{"action":"open","path":"'"$TMP_PROJECT"'.newera"}')
check "file open restores" "$(count w)" "9"
rm -rf "$(dirname "$TMP_PROJECT")"

reply=$(rpc '{"jsonrpc":"2.0","id":3,"method":"tools/list"}')
check "tools carry read/write hints" "$reply" '"readOnlyHint":true'
check "show_plan opens the viewer" "$reply" '"resourceUri":"ui://newera/plan-viewer.html"'
reply=$(rpc '{"jsonrpc":"2.0","id":4,"method":"resources/read","params":{"uri":"ui://newera/plan-viewer.html"}}')
check "the viewer page is served" "$reply" '"mimeType":"text/html;profile=mcp-app"'
# This client declared no viewer: it gets the summary, not the SVG.
reply=$(call show_plan '{}')
check "show_plan without a viewer says so" "$reply" 'no viewer in this client'
[[ "$reply" != *'"svg":'* ]] || { echo "  ✗ show_plan sent the SVG to a client without a viewer"; exit 1; }
echo "  ✓ and sends no SVG"
# A client that names the apps extension gets the drawing for its viewer.
CLI_SESSION="$SESSION"
SESSION=""
rpc '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"'"$PROTOCOL"'","capabilities":{"extensions":{"io.modelcontextprotocol/ui":{"mimeTypes":["text/html;profile=mcp-app"]}}},"clientInfo":{"name":"smoke-app","version":"1"}}}' >/dev/null
SESSION=$(grep -i '^mcp-session-id:' "$LOG.headers" | awk '{print $2}' | tr -d '\r')
rpc '{"jsonrpc":"2.0","method":"notifications/initialized"}' >/dev/null
reply=$(call show_plan '{}')
check "show_plan hands the drawing to the viewer" "$reply" '"svg":"<svg'
SESSION="$CLI_SESSION"

# `newera mcp` with the window open edits that window, not a project of its own.
reply=$({
  echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"'"$PROTOCOL"'","capabilities":{},"clientInfo":{"name":"smoke","version":"1"}}}'
  sleep 0.5
  echo '{"jsonrpc":"2.0","method":"notifications/initialized"}'
  echo '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"create","arguments":{"walls":[{"pts":[[0,900],[400,900]]}]}}}'
  sleep 1.5
} | "$BIN" mcp --addr "127.0.0.1:${PORT}" 2>/dev/null)
check "stdio attaches to the open window" "$reply" 'ok rev='
check "the window has the stdio wall" "$(count w)" "10"

rm -f "$LOG.headers"
echo "All good."
