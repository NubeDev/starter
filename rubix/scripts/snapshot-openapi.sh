#!/usr/bin/env bash
# Snapshot rubix-agent's `/openapi.json` to `rubix/openapi.json`.
#
# Why this exists
# ---------------
# `@nube/rubix-client-ts` is codegen'd from the OpenAPI doc the
# agent serves at `GET /openapi.json`. Codegen must not depend on
# a live agent process at build time, so we commit a snapshot at
# `rubix/openapi.json` and regenerate it from this script.
# See `rubix/HOW-TO-CODE.md` (§OpenAPI snapshot regen) and
# `rubix/docs/design/client-ts/README.md` for the contract.
#
# How it works
# ------------
# 1. Build `rubix-agent` in release mode (no migrations, no DB).
# 2. Boot it in the background with `RUBIX_BIND=127.0.0.1:0` and
#    no `RUBIX_DATABASE_URL`/`RUBIX_CH_URL` so it stays single-
#    process and dependency-free.
# 3. Parse the `local_addr=` field from the boot log to learn the
#    ephemeral port (see `rubix/crates/rubix-agent/src/health.rs`).
# 4. `curl` the document, pretty-print it through `jq` (sorted
#    keys, stable indent) into `rubix/openapi.json`.
# 5. Tear the child down via a `trap` on EXIT/INT/TERM.
#
# Determinism
# -----------
# utoipa serialises the document from compile-time attributes
# only — no timestamps, no host-derived identifiers — so re-running
# this script on a clean tree must produce a byte-identical file.
# CI's `openapi-drift` job (added in phase B.4) enforces that.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT="${REPO_ROOT}/rubix/openapi.json"
LOG_FILE="$(mktemp -t rubix-openapi-XXXXXX.log)"
CHILD_PID=""

cleanup() {
  local rc=$?
  if [[ -n "${CHILD_PID}" ]] && kill -0 "${CHILD_PID}" 2>/dev/null; then
    kill -TERM "${CHILD_PID}" 2>/dev/null || true
    # Give the runtime ~2s to drop its tasks, then SIGKILL.
    for _ in 1 2 3 4 5 6 7 8 9 10; do
      kill -0 "${CHILD_PID}" 2>/dev/null || break
      sleep 0.2
    done
    if kill -0 "${CHILD_PID}" 2>/dev/null; then
      kill -KILL "${CHILD_PID}" 2>/dev/null || true
    fi
  fi
  if [[ ${rc} -ne 0 && -f "${LOG_FILE}" ]]; then
    echo "--- rubix-agent log (tail) ---" >&2
    tail -n 80 "${LOG_FILE}" >&2 || true
  fi
  rm -f "${LOG_FILE}"
  exit "${rc}"
}
trap cleanup EXIT INT TERM

echo ">>> building rubix-agent (release)" >&2
( cd "${REPO_ROOT}" && cargo build --release -p rubix-agent --bin rubix-agent )

BIN="${REPO_ROOT}/target/release/rubix-agent"
if [[ ! -x "${BIN}" ]]; then
  echo "snapshot-openapi: rubix-agent binary missing at ${BIN}" >&2
  exit 1
fi

echo ">>> booting rubix-agent on 127.0.0.1:0" >&2
# Unset every DSN so the binary takes the laptop path (no PG, no
# CH, no migrations). The OpenAPI doc is the same in either mode.
env -u RUBIX_DATABASE_URL -u RUBIX_CH_URL -u DATABASE_URL \
  RUBIX_BIND=127.0.0.1:0 \
  RUST_LOG="${RUST_LOG:-info}" \
  NO_COLOR=1 \
  "${BIN}" >"${LOG_FILE}" 2>&1 &
CHILD_PID=$!

# Wait up to 20s for the `local_addr=` log line.
ADDR=""
for _ in $(seq 1 100); do
  if ! kill -0 "${CHILD_PID}" 2>/dev/null; then
    echo "snapshot-openapi: rubix-agent exited before binding" >&2
    exit 1
  fi
  # Strip ANSI escapes defensively (NO_COLOR is set above, but
  # belt-and-braces keeps this resilient to subscriber changes).
  ADDR="$(sed -E 's/\x1b\[[0-9;]*[a-zA-Z]//g' "${LOG_FILE}" \
    | grep -oE 'local_addr=[0-9.]+:[0-9]+' \
    | head -n 1 \
    | cut -d= -f2 || true)"
  if [[ -n "${ADDR}" ]]; then break; fi
  sleep 0.2
done
if [[ -z "${ADDR}" ]]; then
  echo "snapshot-openapi: never saw local_addr= in boot log" >&2
  exit 1
fi

URL="http://${ADDR}/openapi.json"
echo ">>> GET ${URL}" >&2
RAW="$(curl -fsS --max-time 10 "${URL}")"

# Pretty-print with sorted keys for a deterministic, diff-friendly
# snapshot. `jq -S` sorts object keys recursively; `--indent 2`
# matches the repo's existing `openapi.json` style.
printf '%s' "${RAW}" | jq -S --indent 2 . >"${OUT}"
# Validate JSON well-formedness (jq already errored above on bad
# JSON, but re-parse for an explicit, scriptable signal).
jq -e . "${OUT}" >/dev/null

echo ">>> wrote ${OUT}" >&2
