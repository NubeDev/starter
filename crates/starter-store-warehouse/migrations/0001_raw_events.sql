-- Warehouse SCOPE L1 — raw landing zone.
--
-- One DDL statement per file (CH DDL is non-transactional, and the
-- in-crate migration runner sends one statement per HTTP round trip
-- so a failure mid-file is observable). IF NOT EXISTS everywhere so
-- the runner is idempotent.
--
-- `id UInt64 DEFAULT generateSnowflakeID()` — per M-1, server-side
-- IDs so writers do not have to mint and round-trip a UUID just to
-- get a primary-key-ish column.
--
-- `payload String CODEC(ZSTD(3))` — per SCOPE L1 ("ZSTD on parts
-- older than 3 days"); column-level CODEC is the simplest knob, the
-- server keeps a generic recompression schedule on top of the part
-- merge cycle. `tags Map(String, String)` uses the default codec
-- (LowCardinality dictionaries already compress well).
--
-- `PARTITION BY toYYYYMMDD(received_at)` + `TTL toDateTime(received_at)
--  + INTERVAL 14 DAY` — daily partitions, two-week retention. L1 is
-- a buffer, not a museum. The `toDateTime` cast is required: from
-- ClickHouse 24+, TTL expressions must resolve to `DateTime` or
-- `Date`, never `DateTime64` (BAD_TTL_EXPRESSION at CREATE time).
CREATE TABLE IF NOT EXISTS raw_events (
    id           UInt64 DEFAULT generateSnowflakeID(),
    source       LowCardinality(String),
    received_at  DateTime64(3) DEFAULT now64(3),
    payload      String CODEC(ZSTD(3)),
    tags         Map(String, String) DEFAULT map(),
    -- `mapKeys(tags)` cast required from ClickHouse 24+: a
    -- `bloom_filter` index cannot be declared directly on a
    -- `Map(String, String)` column (ILLEGAL_COLUMN at CREATE
    -- time). Indexing the keys covers the dominant
    -- tag-exists lookup; value-level skip needs a separate
    -- `tokenbf_v1` index on `mapValues(tags)` if it ever
    -- becomes a hot path. Note that the migration runner
    -- forwards this text through the `clickhouse` crate, which
    -- treats a bare U+003F character as a query placeholder
    -- and would reject the file with "unbound query argument".
    INDEX tags_bloom mapKeys(tags) TYPE bloom_filter GRANULARITY 1
)
ENGINE = MergeTree
PARTITION BY toYYYYMMDD(received_at)
ORDER BY (source, received_at)
TTL toDateTime(received_at) + INTERVAL 14 DAY;
