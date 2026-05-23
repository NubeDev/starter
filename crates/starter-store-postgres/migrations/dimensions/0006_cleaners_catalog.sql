-- Warehouse SCOPE `cleaner.define`: catalog of L1 → L2 curation MVs.
--
-- Mirrors the marts catalog (status machine, definition_hash,
-- created_by trust seam) but adds the cleaner-specific knobs:
--
--   * `backfill`        — 'none' | 'sync' | 'async' (CHECK enum).
--   * `validate_entity` — 'strict' | 'best_effort' | 'none', controls
--     whether the cleaner SELECT prefilters on `entities_dict`
--     existence (best_effort) or accepts dangling entity_ids on
--     purpose (none). 'strict' is rejected at define-time because
--     the dictionary lags by up to 10 min (W11) — only `curate.write`
--     can be strict.
--   * `mv_live_at`           — when the MV DDL succeeded; null until live.
--   * `backfill_window_end`  — `now()` snapshot at MV-creation time,
--     used as the upper bound for the explicit INSERT ... SELECT
--     backfill so the MV catches anything that arrives during the
--     backfill without double-counting.
--   * `frozen_at_revision`   — mirrors the sandbox/marts revision-pin
--     idea: the schema revision of the source table (raw_events or
--     a sandbox) at the moment the cleaner went live. Subsequent
--     source-schema bumps invalidate the cleaner and require an
--     explicit redefine.

CREATE TABLE IF NOT EXISTS cleaners (
    name                  TEXT PRIMARY KEY,
    description           TEXT,
    source_table          TEXT NOT NULL,
    target_table          TEXT NOT NULL,
    filter                JSONB NOT NULL,
    projection            JSONB NOT NULL,
    definition_hash       TEXT NOT NULL,
    backfill              TEXT NOT NULL DEFAULT 'none',
    validate_entity       TEXT NOT NULL DEFAULT 'best_effort',
    mv_live_at            TIMESTAMPTZ,
    backfill_window_end   TIMESTAMPTZ,
    frozen_at_revision    BIGINT,
    backfill_status       TEXT NOT NULL DEFAULT 'pending',
    created_by            TEXT NOT NULL,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    status                TEXT NOT NULL,
    CONSTRAINT cleaners_status_valid
        CHECK (status IN ('pending', 'live', 'quarantined', 'failed')),
    CONSTRAINT cleaners_backfill_valid
        CHECK (backfill IN ('none', 'sync', 'async')),
    CONSTRAINT cleaners_backfill_status_valid
        CHECK (backfill_status IN ('pending', 'running', 'complete', 'failed', 'skipped')),
    CONSTRAINT cleaners_validate_entity_valid
        CHECK (validate_entity IN ('strict', 'best_effort', 'none')),
    CONSTRAINT cleaners_created_by_valid CHECK (
        created_by LIKE 'user:%'
        OR created_by LIKE 'agent:%'
        OR created_by LIKE 'ext:%'
    ),
    CONSTRAINT cleaners_target_valid
        CHECK (target_table IN ('samples', 'events', 'documents'))
);
