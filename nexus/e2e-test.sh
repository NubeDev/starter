#!/usr/bin/env bash
# Full end-to-end smoke test of the nexus stack against the seeded dev DB.
# Starts its own nexus-api on :4781 (so it never collides with `make dev` on
# :4780), drives auth + the product surface, and asserts on the seeded sim data.
set -uo pipefail

cd "$(dirname "$0")"

DB_URL="postgres://nexus:nexus@127.0.0.1:4770/nexus"
PORT=4781
BASE="http://127.0.0.1:$PORT"
JAR="$(mktemp)"
PASS=0; FAIL=0
say() { printf '\n=== %s ===\n' "$1"; }
ok()   { printf '  ✓ %s\n' "$1"; PASS=$((PASS+1)); }
bad()  { printf '  ✗ %s\n' "$1"; FAIL=$((FAIL+1)); }

# --- boot a dedicated backend ------------------------------------------------
say "starting nexus-api on :$PORT"
NEXUS_METADATA_URL="$DB_URL" \
NEXUS_DATASOURCE_URL="$DB_URL" \
NEXUS_MASTER_KEY="0123456789abcdef0123456789abcdef" \
NEXUS_STREAM_TOKEN_KEY="0123456789abcdef0123456789abcdef" \
NEXUS_BIND="127.0.0.1:$PORT" \
  cargo run --quiet --manifest-path backend/Cargo.toml --bin nexus-api >/tmp/e2e-api.log 2>&1 &
API_PID=$!
cleanup() { kill "$API_PID" 2>/dev/null; rm -f "$JAR"; }
trap cleanup EXIT

printf "  waiting for health "
for i in $(seq 1 60); do
  code=$(curl -s -o /dev/null -w '%{http_code}' "$BASE/health" 2>/dev/null)
  if [ "$code" = "200" ]; then echo "up"; break; fi
  if ! kill -0 "$API_PID" 2>/dev/null; then echo "API died — see /tmp/e2e-api.log"; tail -20 /tmp/e2e-api.log; exit 1; fi
  printf "."; sleep 1
done

# --- 1. health / unauthenticated guard --------------------------------------
say "1. health + auth guard"
[ "$(curl -s -o /dev/null -w '%{http_code}' "$BASE/health")" = "200" ] && ok "GET /health = 200" || bad "/health not 200"
code=$(curl -s -o /dev/null -w '%{http_code}' "$BASE/api/v1/me")
[ "$code" = "401" ] && ok "GET /api/v1/me unauthenticated = 401" || bad "/api/v1/me without auth gave $code (want 401)"

# --- 2. login ----------------------------------------------------------------
say "2. login"
login=$(curl -s -c "$JAR" -X POST "$BASE/auth/login" \
  -H 'content-type: application/json' \
  -d '{"email":"admin@nexus.local","password":"change-me-admin"}')
csrf=$(printf '%s' "$login" | sed -n 's/.*"csrf_token":"\([^"]*\)".*/\1/p')
[ -n "$csrf" ] && ok "POST /auth/login issued csrf token" || { bad "login failed: $login"; }

# --- 3. authenticated identity ----------------------------------------------
say "3. authenticated /api/v1/me"
me=$(curl -s -b "$JAR" "$BASE/api/v1/me")
echo "$me" | grep -q '"subject"' && ok "GET /api/v1/me returns identity" || bad "me: $me"
echo "$me" | grep -q '"role":"admin"' && ok "principal role is admin" || bad "wrong role: $me"
echo "  → $me"
# /auth/me carries the email (the human identity); /api/v1/me carries the UUID subject.
authme=$(curl -s -b "$JAR" "$BASE/auth/me")
echo "$authme" | grep -q 'admin@nexus.local' && ok "/auth/me is admin@nexus.local" || bad "auth/me identity: $authme"

# --- 4. query the seeded sim data -------------------------------------------
say "4. query seeded sim data"
q() { curl -s -b "$JAR" -X POST "$BASE/api/v1/query" -H 'content-type: application/json' -H "X-CSRF-Token: $csrf" -d "{\"sql\":\"$1\"}"; }

for tbl in sim_hvac sim_energy sim_door; do
  r=$(q "SELECT count(*) AS n FROM $tbl")
  n=$(printf '%s' "$r" | sed -n 's/.*"n":\([0-9]*\).*/\1/p')
  if [ -n "$n" ] && [ "$n" -ge 200 ]; then ok "$tbl has $n rows (>=200)"; else bad "$tbl count = '$n' resp=$r"; fi
done

# data-shape assertions per profile
r=$(q "SELECT min(temp_c) AS lo, max(temp_c) AS hi FROM sim_hvac")
echo "  hvac temp range → $r"
echo "$r" | grep -q '"lo"' && ok "sim_hvac numeric temp_c present" || bad "hvac shape: $r"

r=$(q "SELECT bool_or(kwh_total = lag) AS any_drop FROM (SELECT kwh_total, lag(kwh_total) OVER (ORDER BY ts) AS lag FROM sim_energy) s WHERE lag IS NOT NULL AND kwh_total < lag")
# monotonic check: count rows where the counter went backwards (should be 0)
r=$(q "SELECT count(*) AS drops FROM (SELECT kwh_total, lag(kwh_total) OVER (ORDER BY ts) AS prev FROM sim_energy) s WHERE prev IS NOT NULL AND kwh_total < prev")
drops=$(printf '%s' "$r" | sed -n 's/.*"drops":\([0-9]*\).*/\1/p')
[ "$drops" = "0" ] && ok "sim_energy kwh_total is monotonic (0 backward steps)" || bad "energy not monotonic: $drops drops"

r=$(q "SELECT count(DISTINCT zone) AS zones, count(*) FILTER (WHERE open) AS opens FROM sim_door")
echo "  door → $r"
echo "$r" | grep -q '"zones"' && ok "sim_door bool+str columns queryable" || bad "door shape: $r"

# --- 5. product surface lists -----------------------------------------------
say "5. product surface (authenticated lists)"
for ep in datasources flows dashboards; do
  code=$(curl -s -o /dev/null -w '%{http_code}' -b "$JAR" "$BASE/api/v1/$ep")
  [ "$code" = "200" ] && ok "GET /api/v1/$ep = 200" || bad "/api/v1/$ep gave $code"
done

# --- 6. read-only guard: writes must be rejected ----------------------------
say "6. query guard rejects writes"
r=$(q "DROP TABLE sim_door")
code=$(curl -s -o /dev/null -w '%{http_code}' -b "$JAR" -X POST "$BASE/api/v1/query" -H 'content-type: application/json' -H "X-CSRF-Token: $csrf" -d '{"sql":"DROP TABLE sim_door"}')
[ "$code" = "400" ] && ok "destructive SQL rejected (400)" || bad "DROP not rejected, got $code: $r"

# --- summary -----------------------------------------------------------------
printf '\n========================================\n'
printf 'E2E RESULT: %d passed, %d failed\n' "$PASS" "$FAIL"
printf '========================================\n'
[ "$FAIL" -eq 0 ]
