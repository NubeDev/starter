-- Warehouse SCOPE L2 — curated samples.
--
-- `ORDER BY (entity_id, ts)` lets per-entity lookups skip without
-- depending on the tag bloom filter (SCOPE note on high-cardinality
-- tag values).
--
-- TTL: SCOPE says `ts + INTERVAL 90 DAY TO VOLUME 's3_cold'` and
-- `ts + INTERVAL 2 YEAR DELETE`. The `TO VOLUME 's3_cold'` tier is
-- a deployment-time concern: it requires the storage policy that
-- owns this table to declare an `s3_cold` volume, which is set in
-- ClickHouse server config, not DDL. Including the move tier here
-- would break CREATE TABLE on any deployment that has not yet
-- configured the volume (and on the bare testcontainer). We
-- therefore ship the DELETE tier in this migration and leave the
-- move tier to a deployment-specific
-- `ALTER TABLE samples MODIFY TTL ts + INTERVAL 90 DAY TO VOLUME
-- 's3_cold', ts + INTERVAL 2 YEAR DELETE;` once the storage policy
-- exists. The DELETE bound is the load-bearing retention contract;
-- the move tier is a cost-optimisation tier.
--
-- Bloom-filter skip index on tags per SCOPE; same caveat about
-- high-cardinality tag values applies — see the SCOPE note.
CREATE TABLE IF NOT EXISTS samples (
    entity_id    String CODEC(ZSTD(3)),
    ts           DateTime64(3),
    value_num    Nullable(Float64),
    value_str    Nullable(String) CODEC(ZSTD(3)),
    value_bool   Nullable(UInt8),
    quality      UInt8 DEFAULT 0,
    tags         Map(String, String) DEFAULT map(),
    INDEX tags_bloom tags TYPE bloom_filter GRANULARITY 1,
    INDEX entity_bloom entity_id TYPE bloom_filter GRANULARITY 1
)
ENGINE = MergeTree
PARTITION BY toYYYYMM(ts)
ORDER BY (entity_id, ts)
TTL ts + INTERVAL 2 YEAR DELETE;
