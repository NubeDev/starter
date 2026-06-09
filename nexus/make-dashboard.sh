#!/usr/bin/env bash
# Build a demo dashboard (slug: aaa) of the seeded sim data, via the live API
# behind the UI proxy on :4790. Panels match the canvas contract: the opaque
# `layout` carries grid position + field mapping (x / series), so they render.
set -uo pipefail

BASE="http://127.0.0.1:4790"
SLUG="aaa"
JAR="$(mktemp)"; trap 'rm -f "$JAR"' EXIT

# --- auth --------------------------------------------------------------------
curl -s -c "$JAR" -X POST "$BASE/auth/login" -H 'content-type: application/json' \
  -d '{"email":"admin@nexus.local","password":"change-me-admin"}' >/dev/null
CSRF=$(grep starter_csrf "$JAR" | awk '{print $NF}')
hdr=(-b "$JAR" -H "X-CSRF-Token: $CSRF" -H 'content-type: application/json')

# --- resolve the Demo Sim Data datasource id --------------------------------
DS=$(curl -s -b "$JAR" "$BASE/api/v1/datasources" \
  | sed -n 's/.*"id":"\([^"]*\)","name":"Demo Sim Data".*/\1/p')
[ -z "$DS" ] && { echo "no 'Demo Sim Data' datasource — create it first"; exit 1; }
echo "datasource: $DS"

# --- create the dashboard (ignore conflict if it already exists) ------------
curl -s "${hdr[@]}" -X POST "$BASE/api/v1/dashboards" \
  -d "{\"slug\":\"$SLUG\",\"name\":\"Demo — Sim Devices\"}" >/dev/null
echo "dashboard /d/$SLUG ready"

# add_panel POSTs to /api/v1/dashboards/:slug/panels
panel() { # title datasource sql viz layout_json
  curl -s "${hdr[@]}" -X POST "$BASE/api/v1/dashboards/$SLUG/panels" \
    -d "{\"title\":\"$1\",\"datasource_id\":\"$DS\",\"sql\":\"$2\",\"viz\":\"$3\",\"layout\":$4}" \
    -o /dev/null -w "  + panel '$1' (%{http_code})\n"
}

# 1) HVAC temperature — line over time (x=ts as time, series=temp_c/setpoint)
panel "HVAC Temperature" \
  "SELECT ts, temp_c, setpoint FROM sim_hvac ORDER BY ts" \
  "line" \
  '{"x":0,"y":0,"w":6,"h":4,"fields":{"x":"ts","xKind":"time","series":[{"value":"temp_c","label":"Temp","unit":"°C","color":"152 76% 44%"},{"value":"setpoint","label":"Setpoint","unit":"°C","color":"38 92% 55%"}]}}'

# 2) Energy — kWh total counter over time (area)
panel "Energy — kWh Total" \
  "SELECT ts, kwh_total FROM sim_energy ORDER BY ts" \
  "area" \
  '{"x":6,"y":0,"w":6,"h":4,"fields":{"x":"ts","xKind":"time","series":[{"value":"kwh_total","label":"kWh","unit":"kWh","color":"199 89% 55%"}]}}'

# 3) Current power — stat (single latest value)
panel "Current Power" \
  "SELECT power_w FROM sim_energy ORDER BY ts DESC LIMIT 1" \
  "stat" \
  '{"x":0,"y":4,"w":3,"h":2,"fields":{"series":[{"value":"power_w","label":"Power","unit":"W"}]}}'

# 4) Doors open right now — stat (count of latest-state opens)
panel "Doors Open" \
  "SELECT count(*) FILTER (WHERE open) AS open_count FROM (SELECT DISTINCT ON (zone) zone, open FROM sim_door ORDER BY zone, ts DESC) s" \
  "stat" \
  '{"x":3,"y":4,"w":3,"h":2,"fields":{"series":[{"value":"open_count","label":"Open"}]}}'

# 5) Door events — table (recent rows; ts formatted, bool/text as-is)
panel "Door Events" \
  "SELECT ts, zone, open FROM sim_door ORDER BY ts DESC LIMIT 20" \
  "table" \
  '{"x":6,"y":4,"w":6,"h":4,"fields":{"x":"ts","series":[{"value":"ts","label":"Time","kind":"time"},{"value":"zone","label":"Zone","kind":"text"},{"value":"open","label":"Open","kind":"text"}]}}'

echo
echo "done — open $BASE/d/$SLUG"
echo "panels on dashboard:"
curl -s -b "$JAR" "$BASE/api/v1/dashboards/$SLUG" \
  | grep -o '"title":"[^"]*"' | sed 's/^/  /'
