#!/usr/bin/env bash
# Poll the rubix-dev Postgres + warehouse (TimescaleDB) endpoints
# until both accept TCP connections, with a 30s ceiling.
#
# Used by `mani run demo` between `dev-deps` and `bootstrap` so a
# fresh `docker compose up -d` does not race the binary's first
# connection. Intentionally does NOT shell out to `docker compose`
# — it polls the host-bound ports directly, so it works the same
# whether the stores are containerised, bare-metal, or remote.
#
# Stage 1 of `rubix/docs/proposal/warehouse-engine-swap.md` ships
# this script pointing at the TimescaleDB container (port 5434
# per `rubix/docker/docker-compose.dev.yaml`); Stage 2 wires the
# warehouse crate at it.

set -euo pipefail

PG_HOST="${RUBIX_DEV_PG_HOST:-127.0.0.1}"
PG_PORT="${RUBIX_DEV_PG_PORT:-5433}"
WH_HOST="${RUBIX_DEV_WAREHOUSE_HOST:-127.0.0.1}"
WH_PORT="${RUBIX_DEV_WAREHOUSE_PORT:-5434}"
TIMEOUT="${RUBIX_DEV_WAIT_TIMEOUT:-30}"

probe() {
  # `bash`'s /dev/tcp pseudo-device opens a TCP socket; we discard
  # output and rely on the exit code. Works without nc/curl/psql.
  (exec 3<>/dev/tcp/"$1"/"$2") >/dev/null 2>&1
}

deadline=$(( $(date +%s) + TIMEOUT ))
while :; do
  if probe "$PG_HOST" "$PG_PORT" && probe "$WH_HOST" "$WH_PORT"; then
    echo "rubix dev deps ready (pg=${PG_HOST}:${PG_PORT}, warehouse=${WH_HOST}:${WH_PORT})"
    exit 0
  fi
  if [ "$(date +%s)" -ge "$deadline" ]; then
    echo "wait-for-deps: timed out after ${TIMEOUT}s waiting for pg=${PG_HOST}:${PG_PORT} warehouse=${WH_HOST}:${WH_PORT}" >&2
    exit 1
  fi
  sleep 1
done
