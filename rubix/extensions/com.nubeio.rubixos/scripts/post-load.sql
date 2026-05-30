-- scripts/post-load.sql — install Timescale continuous aggregates that
-- back the rollup warehouse_templates contributed by this extension.
--
-- See rubix/docs/scope/extensions/extension-data-to-dashboard.md
-- ("Layer 1 — Storage"). Idempotent: every CREATE / policy call
-- uses `IF NOT EXISTS` (or `if_not_exists => true`) so running the
-- load pipeline twice in a row is a no-op.
--
-- We ship `_1m` only for now (the chart workhorse the
-- `com.nubeio.rubixos.history_bucketed_1m` template binds against).
-- Wider cadences (`_5m`, `_1h`, `_1d`) are added the day a panel
-- asks for them, per the spec's three operational notes: storage
-- cost is `unique(group_by_keys) × buckets_in_window`, not
-- `raw_rows ÷ bucket_size`, so unused cadences are not free.
--
-- Tenant isolation lives on the query side (templates filter by
-- `$caller_tenant_id`), not the CAGG. The `tenant_id` column is in
-- the materialized view because it is in `GROUP BY`, but the CAGG
-- itself has no RLS — adding RLS would break the refresh.

\set ON_ERROR_STOP on

-- 1m rollup. Backfill is automatic on the first refresh.
CREATE MATERIALIZED VIEW IF NOT EXISTS public.com_nubeio_rubixos__histories_1m
WITH (timescaledb.continuous) AS
SELECT tenant_id,
       point_uuid,
       host_uuid,
       time_bucket('1 minute'::interval, "timestamp") AS bucket,
       avg(value)::float8 AS avg_value,
       min(value)::float8 AS min_value,
       max(value)::float8 AS max_value,
       count(*)           AS sample_count
FROM   public.com_nubeio_rubixos__histories
GROUP  BY tenant_id, point_uuid, host_uuid, bucket
WITH NO DATA;

-- Refresh policy. `end_offset INTERVAL '1 minute'` means a "now"
-- panel sees a 1-minute hole — accept the lag and document it on
-- the panel, or layer a `history_live` UNION on top later. CAGG
-- refresh is global (not per-tenant): in a one-big-tenant + many-
-- small-tenants deployment the small tenants inherit the busiest
-- tenant's bucket size. Acceptable for v1; revisit if it bites.
SELECT add_continuous_aggregate_policy(
    'public.com_nubeio_rubixos__histories_1m',
    start_offset      => INTERVAL '7 days',
    end_offset        => INTERVAL '1 minute',
    schedule_interval => INTERVAL '1 minute',
    if_not_exists     => true
);
