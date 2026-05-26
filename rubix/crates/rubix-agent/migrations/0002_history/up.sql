-- L1 system-disk history table for rubix.
--
-- One row per `rubix.system.disk` probe; written from
-- `rubix-tools::system::disk` after a successful probe. Single
-- DDL statement per file (CH DDL is non-transactional, so the
-- starter-store-warehouse migration runner sends one statement
-- per HTTP round trip and `IF NOT EXISTS` keeps re-apply safe).
--
-- Multi-tenant isolation is the per-row `tenant_id` column — the
-- cheapest of the three options enumerated in
-- docs/design/warehouse/README.md. The mandatory authz filter at
-- the resolver layer keeps cross-tenant reads from leaking.
--
-- `tenant_id UUID` — populated from the principal on the request
-- context; the in-process insights test path uses the zero UUID.
-- `epoch_ms Int64` — UTC milliseconds (matches the DTO timestamp
-- contract; transports render against the caller's timezone).
-- `PARTITION BY toYYYYMM(toDateTime(epoch_ms/1000))` — month-grain
-- partitions so dropping aged data is cheap.
-- `ORDER BY (tenant_id, host, epoch_ms)` — most queries filter by
-- tenant + host then scan time; this is the obvious primary sort.
CREATE TABLE IF NOT EXISTS system_disk_history (
    tenant_id    UUID,
    host         String,
    percent_used UInt8,
    free_bytes   UInt64,
    epoch_ms     Int64
)
ENGINE = MergeTree
PARTITION BY toYYYYMM(toDateTime(epoch_ms / 1000))
ORDER BY (tenant_id, host, epoch_ms);
