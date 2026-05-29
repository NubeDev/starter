#!/usr/bin/env bash
# scripts/install-caggs.sh — install TimescaleDB continuous
# aggregates that back the `usage_bucketed @ '1 day'` template.
#
# Why: at 6m/1y the elec channel aggregates ~200k+ raw rows live on
# every cache miss, taking 4-8s per template. A 1d CAGG refreshed
# hourly turns the same scan into ~9k pre-aggregated rows (<100 ms).
# See `DB.md` §5.1.
#
# Idempotent: re-running drops the policy + view and recreates them.
# The materialised data is preserved by Timescale because the view
# definition is unchanged. Pass `--reset` to drop the view entirely
# and rebuild (slow — re-materialises the whole hypertable).
#
# Requires the rubix-agent to have booted at least once so
# `com_nubeio_rubixos__histories` exists as a hypertable.

set -euo pipefail

PGHOST=${PGHOST:-127.0.0.1}
PGPORT=${PGPORT:-5433}
PGUSER=${PGUSER:-rubix}
PGDATABASE=${PGDATABASE:-rubix}
export PGPASSWORD=${PGPASSWORD:-rubix-dev}

RESET=0
for arg in "$@"; do
    case "$arg" in
        --reset) RESET=1 ;;
        *) echo "unknown arg: $arg" >&2; exit 2 ;;
    esac
done

psql_run() {
    psql -h "$PGHOST" -p "$PGPORT" -U "$PGUSER" -d "$PGDATABASE" \
         -v ON_ERROR_STOP=1 --pset=pager=off "$@"
}

CAGG=com_nubeio_rubixos__usage_daily_cagg

if [[ "$RESET" == "1" ]]; then
    echo "==> DROP MATERIALIZED VIEW $CAGG"
    psql_run -c "DROP MATERIALIZED VIEW IF EXISTS \"$CAGG\" CASCADE;"
fi

echo "==> CREATE MATERIALIZED VIEW $CAGG (if not exists)"
psql_run <<SQL
CREATE MATERIALIZED VIEW IF NOT EXISTS "$CAGG"
WITH (timescaledb.continuous) AS
SELECT tenant_id,
       point_uuid,
       host_uuid,
       time_bucket('1 day'::interval, "timestamp") AS bucket,
       AVG(value)::float8  AS avg_value,
       MIN(value)::float8  AS min_value,
       MAX(value)::float8  AS max_value,
       COUNT(*)            AS sample_count
FROM   com_nubeio_rubixos__histories
GROUP  BY tenant_id, point_uuid, host_uuid, bucket
WITH NO DATA;
SQL

echo "==> add_continuous_aggregate_policy"
# Re-add the policy idempotently. add_* returns an error if a policy
# already exists, so we drop first.
psql_run <<SQL
DO \$\$
DECLARE
    j INTEGER;
BEGIN
    FOR j IN
        SELECT job_id FROM timescaledb_information.jobs
        WHERE proc_name = 'policy_refresh_continuous_aggregate'
          AND hypertable_name = '$CAGG'
    LOOP
        PERFORM remove_continuous_aggregate_policy('$CAGG');
    END LOOP;
END\$\$;

SELECT add_continuous_aggregate_policy(
    '$CAGG',
    start_offset      => INTERVAL '60 days',
    end_offset        => INTERVAL '1 hour',
    schedule_interval => INTERVAL '1 hour'
);
SQL

echo "==> initial refresh (full history) — this can take a minute on first run"
psql_run -c "CALL refresh_continuous_aggregate('$CAGG', NULL, NULL);"

echo "==> $CAGG ready:"
psql_run -c "SELECT count(*) AS rows_cagg FROM \"$CAGG\";"
echo "done."
