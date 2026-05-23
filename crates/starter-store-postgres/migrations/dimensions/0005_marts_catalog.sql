-- Warehouse SCOPE W5 + W12: marts catalog.
--
-- The partial index `marts_live_count_idx` powers the per-deployment
-- live-mart quota trigger. The trigger only scans `status = 'live'`
-- rows, never the full catalog — `tests/dimensions_marts.rs`
-- (`live_mart_quota_trigger_only_scans_live_rows`) pins that
-- behaviour by inserting 200 `quarantined`/`failed` rows and
-- asserting the live insert path is unaffected.

CREATE TABLE IF NOT EXISTS marts (
    name             TEXT PRIMARY KEY,
    description      TEXT,
    source_table     TEXT NOT NULL,
    filter           JSONB NOT NULL,
    time_bucket      INTERVAL NOT NULL,
    group_by         TEXT[] NOT NULL,
    aggregations     JSONB NOT NULL,
    definition_hash  TEXT NOT NULL,
    created_by       TEXT NOT NULL,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    status           TEXT NOT NULL,
    CONSTRAINT marts_status_valid
        CHECK (status IN ('pending', 'live', 'quarantined', 'failed')),
    CONSTRAINT marts_created_by_valid CHECK (
        created_by LIKE 'user:%'
        OR created_by LIKE 'agent:%'
        OR created_by LIKE 'ext:%'
    ),
    CONSTRAINT marts_name_shape
        CHECK (name ~ '^mart_[a-z0-9_]+$')
);

-- Partial index — the trigger below scans only this index, so the
-- quota check is O(live_count) not O(total_marts).
CREATE INDEX IF NOT EXISTS marts_live_count_idx
    ON marts (status)
    WHERE status = 'live';

CREATE OR REPLACE FUNCTION marts_live_quota_check()
RETURNS TRIGGER AS $$
DECLARE
    quota   INT;
    current INT;
BEGIN
    -- Skip when the row is not (or no longer) live; only live
    -- transitions need to verify the cap.
    IF NEW.status IS DISTINCT FROM 'live' THEN
        RETURN NEW;
    END IF;
    IF TG_OP = 'UPDATE' AND OLD.status = 'live' THEN
        RETURN NEW;
    END IF;
    quota := COALESCE(
        current_setting('warehouse.live_mart_quota', true)::int,
        50
    );
    -- Partial-index-backed count: planner uses `marts_live_count_idx`.
    SELECT count(*) INTO current FROM marts WHERE status = 'live';
    IF current >= quota THEN
        RAISE EXCEPTION
            'live mart quota exceeded (% / %): drop or quarantine an existing live mart first',
            current, quota
            USING ERRCODE = 'check_violation';
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS marts_live_quota ON marts;
CREATE TRIGGER marts_live_quota
    BEFORE INSERT OR UPDATE ON marts
    FOR EACH ROW
    EXECUTE FUNCTION marts_live_quota_check();
