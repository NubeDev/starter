-- L1 raw meter-readings table for the rubix data-flow scenario.
--
-- One row per emitted reading; written from
-- `rubix-tools::warehouse::ingest` via a multi-row INSERT after the
-- producer flow's synth node fires. Single DDL statement per file
-- (CH DDL is non-transactional, so the starter-store-clickhouse
-- migration runner sends one statement per HTTP round trip and
-- `IF NOT EXISTS` keeps re-apply safe).
--
-- Multi-tenant isolation is the per-row `tenant_id` column — the
-- cheapest of the three options enumerated in
-- docs/design/warehouse/README.md. The mandatory authz filter at
-- the resolver layer keeps cross-tenant reads from leaking.
--
-- Wire shape (locked across stages 01 → 05; see
-- `rubix/docs/sessions/data-flow/01-producer.md` "Wire shape"):
-- tenant_id String, meter_id String, kind ('electricity'|'water'),
-- unit ('kWh'|'L'), epoch_ms Int64, value Float64 (NaN allowed for
-- electricity.hvac mess), quality ('ok'|'suspect'|'missing').
--
-- Schema is **owned by warehouse-bundled migration**, not by the
-- operator-authored `rubix.clickhouse.rule.write` verb. Same
-- precedent as `0002_history/up.sql` (system_disk_history). The
-- rule.write verb is reserved for operator-authored derived-state
-- rules, not for the hard-coded core schema. See
-- `rubix/docs/sessions/data-flow/02-ingest-l1-blockers-2026-05-26.md`
-- for the rationale.
--
-- `PARTITION BY toYYYYMM(toDateTime(epoch_ms/1000))` — month-grain
-- partitions so dropping aged data is cheap.
-- `ORDER BY (tenant_id, meter_id, epoch_ms)` — most queries filter
-- by tenant + meter then scan time; this is the obvious primary
-- sort. Matches the canonical layout in design/warehouse/README.md.
CREATE TABLE IF NOT EXISTS meter_readings_raw (
    tenant_id String,
    meter_id  String,
    kind      LowCardinality(String),
    unit      LowCardinality(String),
    epoch_ms  Int64,
    value     Float64,
    quality   LowCardinality(String)
)
ENGINE = MergeTree
PARTITION BY toYYYYMM(toDateTime(epoch_ms / 1000))
ORDER BY (tenant_id, meter_id, epoch_ms);
