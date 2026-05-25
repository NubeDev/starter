#!/usr/bin/env bash
# migrate-node-state-to-pg.sh — one-shot migration of legacy
# `~/.rubix/node_state.db` rows into the Postgres `node_state`
# table.
#
# Context: rubix-agent originally backed the flow `NodeStateStore`
# SPI with SQLite at `~/.rubix/node_state.db`. As of
# `rubix/docs/scope/sqlite-to-postgres.md` that state moves into
# the same Postgres database that holds flow definitions, runs,
# sessions, etc. The boot path performs a best-effort auto-copy
# on first start of the new binary (see
# `rubix-agent/src/boot/flow_runtime.rs::migrate_legacy_node_state_db`);
# this script is the manual escape hatch for operators who
#
#   - want to migrate before upgrading the binary, or
#   - need to re-run the copy after the auto-step failed, or
#   - are restoring an old `~/.rubix/node_state.db` backup against
#     an already-upgraded agent.
#
# Semantics: first-writer-wins. Rows already present in PG under
# the same `(flow_id, node_id, key)` are NEVER overwritten — they
# represent state the upgraded agent has already written. If you
# explicitly want the SQLite snapshot to override, truncate
# `node_state` in PG first (a destructive operation requiring
# operator sign-off).
#
# Requirements: `sqlite3` and `psql` on PATH. `PGURL` env var
# pointing at the rubix database (e.g.
# `postgres://rubix:rubix@localhost:5432/rubix`).
#
# Idempotent: safe to re-run. After a successful run the source
# file is renamed to `<file>.migrated` so the next invocation is
# a no-op (mirrors the boot-time auto-copy behaviour).

set -euo pipefail

SOURCE_DB="${SOURCE_DB:-$HOME/.rubix/node_state.db}"
PGURL="${PGURL:-${RUBIX_DATABASE_URL:-}}"

if [[ -z "$PGURL" ]]; then
    echo "error: set PGURL (or RUBIX_DATABASE_URL) to the rubix Postgres DSN" >&2
    exit 2
fi

if [[ ! -f "$SOURCE_DB" ]]; then
    echo "no legacy node_state.db at $SOURCE_DB — nothing to do"
    exit 0
fi

for cmd in sqlite3 psql; do
    if ! command -v "$cmd" >/dev/null 2>&1; then
        echo "error: $cmd not found on PATH" >&2
        exit 2
    fi
done

echo "migrating $SOURCE_DB -> $PGURL"

# Warn if PG already has rows — first-writer-wins means those rows
# survive. Operators who want the snapshot to override should
# truncate `node_state` first; the script does not do that
# automatically.
EXISTING="$(psql "$PGURL" -Atc 'SELECT COUNT(*) FROM node_state')"
if [[ "$EXISTING" -gt 0 ]]; then
    echo "warning: PG node_state already contains $EXISTING rows;" >&2
    echo "         the copy is first-writer-wins (ON CONFLICT DO NOTHING)." >&2
    echo "         To force the SQLite snapshot to override, run" >&2
    echo "         \`psql \$PGURL -c 'TRUNCATE node_state'\` first." >&2
fi

TMP_TSV="$(mktemp -t node_state_migration.XXXXXX.tsv)"
trap 'rm -f "$TMP_TSV"' EXIT

# Dump the SQLite rows as TSV with hex-encoded `value` so binary
# bytes survive the pipeline. `psql` decodes the column with
# `decode(value, 'hex')` on the staging-table insert path below.
sqlite3 -separator $'\t' "$SOURCE_DB" <<SQL > "$TMP_TSV"
SELECT flow_id, node_id, key, hex(value), version FROM node_state;
SQL

ROW_COUNT="$(wc -l < "$TMP_TSV" | tr -d ' ')"
echo "source rows: $ROW_COUNT"

if [[ "$ROW_COUNT" -eq 0 ]]; then
    echo "no rows to copy"
else
    # Staging-table pattern: COPY does NOT honour ON CONFLICT,
    # so we land the rows in a temp table and then do an INSERT
    # ... SELECT ... ON CONFLICT DO NOTHING. Single psql
    # session — temp tables die at session close, and the
    # commit must happen before we leave it.
    psql "$PGURL" -v ON_ERROR_STOP=1 -v tsv_path="$TMP_TSV" <<'SQL'
BEGIN;

CREATE TEMP TABLE node_state_staging (
    flow_id    TEXT NOT NULL,
    node_id    TEXT NOT NULL,
    key        TEXT NOT NULL,
    value_hex  TEXT NOT NULL,
    version    BIGINT NOT NULL
) ON COMMIT DROP;

\copy node_state_staging (flow_id, node_id, key, value_hex, version) FROM :'tsv_path' WITH (FORMAT text, DELIMITER E'\t')

INSERT INTO node_state (flow_id, node_id, key, value, version, updated_at)
SELECT flow_id, node_id, key, decode(value_hex, 'hex'), version, NOW()
FROM   node_state_staging
ON CONFLICT (flow_id, node_id, key) DO NOTHING;

COMMIT;
SQL
fi

# Rename the source file so a second invocation is a no-op and
# the boot-time auto-copy in `flow_runtime.rs` also short-circuits.
MIGRATED="${SOURCE_DB}.migrated"
mv -f "$SOURCE_DB" "$MIGRATED"
echo "renamed source to $MIGRATED"
echo "done"
