-- Warehouse SCOPE L2 — events history.
--
-- `id UInt64 DEFAULT generateSnowflakeID()` per M-1.
--
-- `kind LowCardinality(String)` per SCOPE — see M-5 cardinality cap
-- comment: LowCardinality is only safe when the column has a small
-- finite vocabulary (`alarm` | `state-change` | `note` | …). New
-- kinds need a coordinated cardinality review before being added;
-- once the dictionary spills past ~10k distinct values the
-- LowCardinality compression collapses and read performance
-- regresses faster than the table grows. The SCOPE-listed enum is
-- the load-bearing cap, not just a starter list.
--
-- `ORDER BY (kind, entity_id, ts)` puts the low-cardinality column
-- first so the index can skip whole kinds cheaply; per-entity
-- lookups within a kind still get good locality.
--
-- `TTL toDateTime(ts) + INTERVAL 1 YEAR` per SCOPE. The cast is
-- mandatory: ClickHouse 24+ rejects TTL expressions whose result
-- type is `DateTime64` (BAD_TTL_EXPRESSION at CREATE time).
CREATE TABLE IF NOT EXISTS events (
    id           UInt64 DEFAULT generateSnowflakeID(),
    entity_id    String CODEC(ZSTD(3)),
    ts           DateTime64(3),
    kind         LowCardinality(String),
    payload      String CODEC(ZSTD(3)),
    tags         Map(String, String) DEFAULT map(),
    -- See `0001_raw_events.sql` — ClickHouse 24+ rejects
    -- `bloom_filter` declared directly on a Map column; index
    -- the keys instead.
    INDEX tags_bloom mapKeys(tags) TYPE bloom_filter GRANULARITY 1
)
ENGINE = MergeTree
PARTITION BY toYYYYMM(ts)
ORDER BY (kind, entity_id, ts)
TTL toDateTime(ts) + INTERVAL 1 YEAR;
