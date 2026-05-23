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
-- `PARTITION BY toYYYYMMDD(received_at)` + `TTL received_at +
--  INTERVAL 14 DAY` — daily partitions, two-week retention. L1 is
-- a buffer, not a museum.
CREATE TABLE IF NOT EXISTS raw_events (
    id           UInt64 DEFAULT generateSnowflakeID(),
    source       LowCardinality(String),
    received_at  DateTime64(3) DEFAULT now64(3),
    payload      String CODEC(ZSTD(3)),
    tags         Map(String, String) DEFAULT map(),
    INDEX tags_bloom tags TYPE bloom_filter GRANULARITY 1
)
ENGINE = MergeTree
PARTITION BY toYYYYMMDD(received_at)
ORDER BY (source, received_at)
TTL received_at + INTERVAL 14 DAY;
