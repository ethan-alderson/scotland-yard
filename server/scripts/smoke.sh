#!/usr/bin/env bash
#
# Phase 1 smoke test: create a game, then auto-drive it to a terminal state by
# always taking the first legal move (or passing when forced). Verifies the
# REST lifecycle and that legal_moves stays in lockstep with the engine.
#
# Requires: a running server (`cargo run -p server`), plus `curl` and `jq`.
# Usage: BASE=http://127.0.0.1:3000 server/scripts/smoke.sh

set -euo pipefail
BASE=${BASE:-http://127.0.0.1:3000}

echo "== Creating game (3 detectives, seeded) =="
GAME=$(curl -fsS -X POST "$BASE/api/games" \
  -H 'content-type: application/json' \
  -d '{"detectives":3,"seed":42}')
echo "$GAME" | jq '{game_id, current_player, players: [.players[] | {id, station}]}'
ID=$(echo "$GAME" | jq -r .game_id)

echo
echo "== Driving the game =="
for _ in $(seq 1 300); do
  STATE=$(curl -fsS "$BASE/api/games/$ID")
  if [ "$(echo "$STATE" | jq -r .is_terminal)" = "true" ]; then
    break
  fi

  LM=$(curl -fsS "$BASE/api/games/$ID/legal_moves")
  if [ "$(echo "$LM" | jq '.singles | length')" -gt 0 ]; then
    TO=$(echo "$LM" | jq '.singles[0].to')
    TK=$(echo "$LM" | jq -r '.singles[0].tickets[0]')
    BODY=$(jq -nc --argjson to "$TO" --arg tk "$TK" '{kind:"single", to:$to, ticket:$tk}')
  elif [ "$(echo "$LM" | jq -r .can_pass)" = "true" ]; then
    BODY='{"kind":"pass"}'
  else
    echo "BUG: non-terminal state with no legal moves"; echo "$LM" | jq .; exit 1
  fi

  curl -fsS -X POST "$BASE/api/games/$ID/moves" \
    -H 'content-type: application/json' -d "$BODY" > /dev/null
done

echo "== Final state =="
curl -fsS "$BASE/api/games/$ID" \
  | jq '{turn_number, current_player, is_terminal, winner, mr_x_log_len: (.mr_x_log | length)}'

echo
echo "== Negative checks =="
echo -n "unknown game -> "; curl -s -o /dev/null -w "%{http_code}\n" "$BASE/api/games/does-not-exist"
echo -n "illegal move -> "; curl -s -o /dev/null -w "%{http_code}\n" \
  -X POST "$BASE/api/games/$ID/moves" -H 'content-type: application/json' \
  -d '{"kind":"single","to":1,"ticket":"taxi"}'
