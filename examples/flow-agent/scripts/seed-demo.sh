#!/usr/bin/env bash
# Seed a demo flow with three connected nodes:
#   Trigger.fire ──> AI Agent.in ──> Transform.in
# Usage: BACKEND_URL=http://127.0.0.1:9741 ./scripts/seed-demo.sh
set -euo pipefail

BACKEND_URL="${BACKEND_URL:-http://127.0.0.1:9741}"
FRONTEND_URL="${FRONTEND_URL:-http://127.0.0.1:9742}"

# Wait for backend up to 10s.
for i in $(seq 1 20); do
  if curl -fsS "${BACKEND_URL}/health" >/dev/null 2>&1; then break; fi
  sleep 0.5
done

graph=$(cat <<'JSON'
{
  "nodes": [
    {
      "id": "trigger-1",
      "kind": "trigger",
      "position": { "x": 80, "y": 200 },
      "label": "Start"
    },
    {
      "id": "agent-1",
      "kind": "ai-agent",
      "position": { "x": 460, "y": 180 },
      "label": "Reasoner",
      "data": { "provider": "anthropic.claude", "model": "claude-sonnet-4-5" }
    }
  ],
  "edges": [
    {
      "id": "e-trigger-agent",
      "source": "trigger-1", "sourceSlot": "fire",
      "target": "agent-1",   "targetSlot": "in"
    }
  ]
}
JSON
)

body=$(jq -n --arg name "Demo: trigger → agent" \
              --arg desc "Seeded by scripts/seed-demo.sh — two nodes wired end to end." \
              --argjson graph "$graph" \
              '{name:$name, description:$desc, graph:$graph}')

echo "POST ${BACKEND_URL}/api/flows"
resp=$(curl -fsS -X POST "${BACKEND_URL}/api/flows" \
  -H 'content-type: application/json' \
  -d "$body")

flow_id=$(echo "$resp" | jq -r .id)
echo "Created flow: ${flow_id}"
echo
echo "Open: ${FRONTEND_URL}/flows/${flow_id}"
echo
echo "Fire it:"
echo "  curl -X POST ${BACKEND_URL}/api/flows/${flow_id}/fire \\"
echo "       -H 'content-type: application/json' \\"
echo "       -d '{\"payload\":\"Reply with the single word: pong\"}'"
