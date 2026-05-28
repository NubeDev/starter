#!/usr/bin/env bash
# scripts/load-dump.sh — restore a Rubix-OS pg_dump into the rubix
# warehouse and bulk-INSERT the rows into this extension's
# host-owned warehouse tables (`com_nubeio_rubixos__*`).
#
# Steps:
#
#   1. Sanity-check: the rubix agent must have booted at least once
#      so `boot::extension_tables` has CREATEd
#      `com_nubeio_rubixos__*` tables (empty, with the host-prepended
#      `tenant_id TEXT NOT NULL` column).
#   2. Drop + recreate `com_nubeio_rubixos__histories` as a Timescale
#      hypertable on `"timestamp"`. The host's plain CREATE TABLE
#      can't add the hypertable conversion itself, and `histories`
#      is the only table that benefits from Timescale chunking. We
#      preserve the host's `(tenant_id, ...)` index by recreating it.
#   3. `pg_restore --schema=public --data-only`-style — actually we
#      use `--no-owner --no-acl --schema-only` first into a staging
#      schema `rubixos_import`, then `--data-only` into the same
#      schema (`pre-data → data → post-data` ordering pg_restore
#      handles natively). The staging schema means we never touch
#      `public` (where rubix-native tables live).
#   4. Bulk `INSERT … SELECT` from `rubixos_import.<table>` into
#      `com_nubeio_rubixos__<table>`, stamping `tenant_id`
#      (defaults to '*' — matches a fresh dev session whose
#      login is bound to the wildcard tenant; pass
#      `--tenant-id <id>` to load under a specific tenant).
#   5. (Optional) `--drop-staging` removes `rubixos_import` after
#      ingest so it doesn't sit around eating disk.
#
# The script is idempotent at the table level: re-running it
# truncates each `com_nubeio_rubixos__*` table for the chosen
# tenant first, then re-ingests. Running it against an empty
# staging schema is a no-op.
#
# Usage:
#
#   ./load-dump.sh \
#       --dump /home/user/Documents/db.dump \
#       --container <pg-container-id>                       \
#       [--tenant-id *]                                    \
#       [--db rubix] [--user rubix]                         \
#       [--drop-staging]
#
# Requires `docker exec` access to the rubix-postgres container.
# Pass `--no-docker` to run psql/pg_restore directly against
# whatever cluster `$PGHOST`/`$PGPORT`/`$PGUSER` point at instead.

set -euo pipefail

DUMP=""
CONTAINER=""
TENANT_ID="*"
DB="rubix"
USER="rubix"
NO_DOCKER=0
DROP_STAGING=0
STAGING_SCHEMA="rubixos_import"
EXT_TABLE_PREFIX="com_nubeio_rubixos__"

usage() {
    sed -n '2,/^$/p' "$0" | sed 's/^# \{0,1\}//'
    exit "${1:-1}"
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --dump)         DUMP="$2"; shift 2 ;;
        --container)    CONTAINER="$2"; shift 2 ;;
        --tenant-id)    TENANT_ID="$2"; shift 2 ;;
        --db)           DB="$2"; shift 2 ;;
        --user)         USER="$2"; shift 2 ;;
        --no-docker)    NO_DOCKER=1; shift ;;
        --drop-staging) DROP_STAGING=1; shift ;;
        -h|--help)      usage 0 ;;
        *)              echo "unknown flag: $1" >&2; usage 1 ;;
    esac
done

[[ -n "$DUMP" ]]      || { echo "--dump is required" >&2; usage 1; }
[[ -f "$DUMP" ]]      || { echo "dump not found: $DUMP" >&2; exit 1; }
if [[ "$NO_DOCKER" -eq 0 ]]; then
    [[ -n "$CONTAINER" ]] || { echo "--container <id> or --no-docker required" >&2; usage 1; }
fi

# psql / pg_restore wrappers — route through docker exec or use the
# host binaries directly, depending on --no-docker.
if [[ "$NO_DOCKER" -eq 1 ]]; then
    PSQL=(psql -U "$USER" -d "$DB" -v ON_ERROR_STOP=1)
    PG_RESTORE=(pg_restore -U "$USER" -d "$DB")
    DUMP_PATH_IN_PG="$DUMP"
else
    # Copy the dump into the container's /tmp once; pg_restore needs
    # to be able to seek the custom-format file.
    DUMP_PATH_IN_PG="/tmp/$(basename "$DUMP")"
    echo "==> docker cp $DUMP -> $CONTAINER:$DUMP_PATH_IN_PG"
    docker cp "$DUMP" "$CONTAINER:$DUMP_PATH_IN_PG"
    PSQL=(docker exec -i "$CONTAINER" psql -U "$USER" -d "$DB" -v ON_ERROR_STOP=1)
    PG_RESTORE=(docker exec -i "$CONTAINER" pg_restore -U "$USER" -d "$DB")
fi

psql_one() { "${PSQL[@]}" -t -A -c "$1"; }
psql_run() { "${PSQL[@]}" -c "$1" >/dev/null; }
psql_run_fail_ok() { "${PSQL[@]}" -c "$1" >/dev/null 2>&1 || true; }

echo "==> sanity: extension tables exist (host has booted at least once)"
for tbl in histories points device_tags device_meta_tags \
           network_tags network_meta_tags point_tags point_meta_tags; do
    n=$(psql_one "SELECT 1 FROM pg_tables WHERE schemaname='public' AND tablename='${EXT_TABLE_PREFIX}${tbl}'")
    if [[ "$n" != "1" ]]; then
        echo "missing table ${EXT_TABLE_PREFIX}${tbl}. Start the rubix agent so" >&2
        echo "boot::extension_tables creates them, then re-run."                  >&2
        exit 1
    fi
done

echo "==> ensure timescaledb extension"
psql_run "CREATE EXTENSION IF NOT EXISTS timescaledb"

# Recreate the histories table as a hypertable. The host's plain
# CREATE TABLE doesn't know about Timescale, so we drop + recreate
# with the same columns the host would have produced, then
# `create_hypertable` it. Safe to do BEFORE the bulk INSERT (cheap
# on an empty table); doing it after-the-fact would require
# create_hypertable(..., migrate_data => true) and is much slower.
echo "==> recreate com_nubeio_rubixos__histories as a Timescale hypertable"
psql_run "DROP TABLE IF EXISTS public.${EXT_TABLE_PREFIX}histories"
psql_run "CREATE TABLE public.${EXT_TABLE_PREFIX}histories (
    tenant_id   TEXT NOT NULL,
    point_uuid  TEXT NOT NULL,
    host_uuid   TEXT NOT NULL,
    value       NUMERIC,
    \"timestamp\" TIMESTAMPTZ NOT NULL
)"
psql_run "SELECT create_hypertable(
    'public.${EXT_TABLE_PREFIX}histories',
    'timestamp',
    chunk_time_interval => INTERVAL '1 day',
    if_not_exists => TRUE
)"
psql_run "CREATE INDEX IF NOT EXISTS ${EXT_TABLE_PREFIX}histories_tenant_point_ts
    ON public.${EXT_TABLE_PREFIX}histories (tenant_id, point_uuid, \"timestamp\" DESC)"

echo "==> drop + recreate staging schema $STAGING_SCHEMA"
psql_run "DROP SCHEMA IF EXISTS $STAGING_SCHEMA CASCADE"
psql_run "CREATE SCHEMA $STAGING_SCHEMA"

echo "==> pg_restore source dump into $STAGING_SCHEMA (this can take a few minutes)"
# Strategy: emit pg_restore's table-of-contents (`-l`), keep ONLY
# the `TABLE` and `TABLE DATA` entries from schema `public`, then
# restore with `-L filtered.list`. This skips:
#
#   * EXTENSION / SCHEMA / ACL / COMMENT entries (no perms to recreate)
#   * INDEX / CONSTRAINT / TRIGGER entries (we don't need them in staging)
#   * Anything in `_timescaledb_internal` / `_timescaledb_catalog`
#     (chunk tables, the `insert_blocker` trigger whose function
#     name differs across Timescale versions — the source of the
#     "_timescaledb_functions.insert_blocker() does not exist" error)
#   * FUNCTION / SEQUENCE OWNED BY entries
#
# `--no-owner --no-acl` drops any residual ALTER OWNER / GRANT lines.
# pg_restore has no native schema-rename flag, so we render to plain
# SQL and `sed` `public.` → `${STAGING_SCHEMA}.` before piping into
# psql.
RESTORE_LIST_HOST="/tmp/com.nubeio.rubixos.restore.list"
# pg_restore -l output (custom format) looks like:
#   272; 1259 17951 TABLE public histories postgres
#   7978; 0 17951 TABLE DATA public histories postgres
# Two shapes. For TABLE entries the schema is in field 5; for
# TABLE DATA entries field 5 is the literal "DATA" and the schema
# is in field 6.
#
# We keep:
#   - everything in schema `public` (regular tables + their data)
#   - all `_hyper_1_*_chunk` tables + data in `_timescaledb_internal`
#     — these are the per-time-range partitions of the source's
#     `histories` hypertable. The parent `public.histories` table
#     is empty in the dump; the rows live in these chunks.
#
# Both schemas are sed-rewritten to ${STAGING_SCHEMA} so the
# restore lands in one flat staging schema.
filter_toc() {
    awk '
        $4 == "TABLE" && $5 == "public"                       { print; next }
        $4 == "TABLE" && $5 == "DATA" && $6 == "public"       { print; next }
        $4 == "TABLE" && $5 == "_timescaledb_internal" && $6 ~ /^_hyper_1_[0-9]+_chunk$/  { print; next }
        $4 == "TABLE" && $5 == "DATA" && $6 == "_timescaledb_internal" && $7 ~ /^_hyper_1_[0-9]+_chunk$/ { print; next }
    '
}
if [[ "$NO_DOCKER" -eq 1 ]]; then
    pg_restore -l "$DUMP" | filter_toc > "$RESTORE_LIST_HOST"
    echo "    TOC entries kept: $(wc -l < "$RESTORE_LIST_HOST")"
    pg_restore --no-owner --no-acl -L "$RESTORE_LIST_HOST" -f - "$DUMP" \
      | sed -E -e "s/public\\./${STAGING_SCHEMA}./g" \
              -e "s/_timescaledb_internal\\./${STAGING_SCHEMA}./g" \
              -e "/^SET (default_table_access_method|search_path)/d" \
      | "${PSQL[@]}" >/dev/null
else
    RESTORE_LIST_PG="/tmp/com.nubeio.rubixos.restore.list"
    docker exec -i "$CONTAINER" pg_restore -l "$DUMP_PATH_IN_PG" \
      | filter_toc > "$RESTORE_LIST_HOST"
    echo "    TOC entries kept: $(wc -l < "$RESTORE_LIST_HOST")"
    docker cp "$RESTORE_LIST_HOST" "$CONTAINER:$RESTORE_LIST_PG"
    docker exec -i "$CONTAINER" pg_restore --no-owner --no-acl \
        -L "$RESTORE_LIST_PG" -f - "$DUMP_PATH_IN_PG" \
      | sed -E -e "s/public\\./${STAGING_SCHEMA}./g" \
              -e "s/_timescaledb_internal\\./${STAGING_SCHEMA}./g" \
              -e "/^SET (default_table_access_method|search_path)/d" \
      | "${PSQL[@]}" >/dev/null
fi

echo "==> ingest into extension-owned tables (tenant_id='$TENANT_ID')"

ingest() {
    local src="$1"; local dst="${EXT_TABLE_PREFIX}$2"; local cols="$3"; local src_cols="$4"
    local has
    has=$(psql_one "SELECT 1 FROM pg_tables WHERE schemaname='$STAGING_SCHEMA' AND tablename='$src'")
    if [[ "$has" != "1" ]]; then
        echo "    skip: $STAGING_SCHEMA.$src not in dump"
        return
    fi
    echo "    $src -> $dst"
    psql_run "DELETE FROM public.$dst WHERE tenant_id = '$TENANT_ID'"
    psql_run "INSERT INTO public.$dst (tenant_id, $cols)
              SELECT '$TENANT_ID', $src_cols FROM $STAGING_SCHEMA.$src"
}

# Small lookup tables first.
ingest points              points              \
    "uuid, name, description, device_uuid, device_name, device_description, network_uuid, network_name, network_description, global_uuid, host_uuid, host_name, host_description, group_uuid, group_name, group_description, location_uuid, location_name, location_description" \
    "uuid, name, description, device_uuid, device_name, device_description, network_uuid, network_name, network_description, global_uuid, host_uuid, host_name, host_description, group_uuid, group_name, group_description, location_uuid, location_name, location_description"

ingest device_tags         device_tags         "host_uuid, device_uuid, tag"          "host_uuid, device_uuid, tag"
ingest device_meta_tags    device_meta_tags    "host_uuid, device_uuid, key, value"   "host_uuid, device_uuid, key, value"
ingest network_tags        network_tags        "host_uuid, network_uuid, tag"         "host_uuid, network_uuid, tag"
ingest network_meta_tags   network_meta_tags   "host_uuid, network_uuid, key, value"  "host_uuid, network_uuid, key, value"
ingest point_tags          point_tags          "host_uuid, point_uuid, tag"           "host_uuid, point_uuid, tag"
ingest point_meta_tags     point_meta_tags     "host_uuid, point_uuid, key, value"    "host_uuid, point_uuid, key, value"

# Histories last — by far the largest table. In the source dump
# `public.histories` is the empty hypertable parent; the rows live
# in `_timescaledb_internal._hyper_1_*_chunk` partitions which the
# pg_restore step landed in the staging schema under the same
# `_hyper_1_*_chunk` names. Truncate the destination once, then
# stream each chunk's rows into the hypertable in a single
# `INSERT … SELECT`. Per-chunk loop avoids one giant transaction
# and gives readable progress.
echo "    histories -> com_nubeio_rubixos__histories  (chunked)"
psql_run "DELETE FROM public.${EXT_TABLE_PREFIX}histories WHERE tenant_id = '$TENANT_ID'"
CHUNKS=$(psql_one "SELECT tablename FROM pg_tables
                   WHERE schemaname='$STAGING_SCHEMA'
                     AND tablename ~ '^_hyper_1_[0-9]+_chunk$'
                   ORDER BY tablename")
CHUNK_COUNT=$(echo "$CHUNKS" | grep -c .) || true
echo "      $CHUNK_COUNT chunks to stream"
i=0
for chunk in $CHUNKS; do
    i=$((i+1))
    rows=$(psql_one "
        WITH ins AS (
            INSERT INTO public.${EXT_TABLE_PREFIX}histories
                (tenant_id, point_uuid, host_uuid, value, \"timestamp\")
            SELECT '$TENANT_ID', point_uuid, host_uuid, value, \"timestamp\"
            FROM $STAGING_SCHEMA.\"$chunk\"
            RETURNING 1
        ) SELECT count(*) FROM ins")
    printf "      [%3d/%3d] %-32s  %10s rows\n" "$i" "$CHUNK_COUNT" "$chunk" "$rows"
done

echo "==> ANALYZE for fresh planner stats"
psql_run "ANALYZE public.${EXT_TABLE_PREFIX}histories"
psql_run "ANALYZE public.${EXT_TABLE_PREFIX}points"

if [[ "$DROP_STAGING" -eq 1 ]]; then
    echo "==> drop staging schema $STAGING_SCHEMA"
    psql_run "DROP SCHEMA $STAGING_SCHEMA CASCADE"
fi

echo "==> summary"
"${PSQL[@]}" -c "
SELECT 'histories'         AS tbl, count(*) FROM public.${EXT_TABLE_PREFIX}histories         WHERE tenant_id='$TENANT_ID'
UNION ALL SELECT 'points',         count(*) FROM public.${EXT_TABLE_PREFIX}points            WHERE tenant_id='$TENANT_ID'
UNION ALL SELECT 'device_tags',    count(*) FROM public.${EXT_TABLE_PREFIX}device_tags       WHERE tenant_id='$TENANT_ID'
UNION ALL SELECT 'device_meta_tags', count(*) FROM public.${EXT_TABLE_PREFIX}device_meta_tags WHERE tenant_id='$TENANT_ID'
UNION ALL SELECT 'network_tags',   count(*) FROM public.${EXT_TABLE_PREFIX}network_tags      WHERE tenant_id='$TENANT_ID'
UNION ALL SELECT 'network_meta_tags', count(*) FROM public.${EXT_TABLE_PREFIX}network_meta_tags WHERE tenant_id='$TENANT_ID'
UNION ALL SELECT 'point_tags',     count(*) FROM public.${EXT_TABLE_PREFIX}point_tags        WHERE tenant_id='$TENANT_ID'
UNION ALL SELECT 'point_meta_tags', count(*) FROM public.${EXT_TABLE_PREFIX}point_meta_tags  WHERE tenant_id='$TENANT_ID';
"

echo "done."
