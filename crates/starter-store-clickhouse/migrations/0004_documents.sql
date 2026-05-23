-- Warehouse SCOPE L2 — documents (blob index rows; the blob itself
-- lives in BlobStore).
--
-- `id String` — caller-supplied per SCOPE (documents are typically
-- minted with a content-addressed id by the upload pipeline). No
-- `DEFAULT generateSnowflakeID()` here, unlike `raw_events` /
-- `events`.
CREATE TABLE IF NOT EXISTS documents (
    id           String,
    entity_id    String CODEC(ZSTD(3)),
    ts           DateTime64(3) DEFAULT now64(3),
    blob_ref     String CODEC(ZSTD(3)),
    mime         LowCardinality(String),
    tags         Map(String, String) DEFAULT map(),
    INDEX tags_bloom tags TYPE bloom_filter GRANULARITY 1
)
ENGINE = MergeTree
PARTITION BY toYYYYMM(ts)
ORDER BY (entity_id, ts);
