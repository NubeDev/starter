-- Detection federation (WS-15 parity): let a detection run a cross-datasource
-- or file join, not just a single-datasource push-down query.
--
-- A panel/Explore query can name `sources` (alias → datasource id) and run
-- through the RW-05 federation engine — joining Postgres tables across
-- datasources, or reading parquet/csv files directly. The detection runner
-- claimed parity with a panel query but only stored a flat `sql` + one
-- `datasource_id`. This adds the same `sources` channel: when present, the
-- runner dispatches to federation exactly like the query route; when empty
-- (the default), the detection stays on the single-datasource path, byte for
-- byte as before. Purely additive — existing detections have `'[]'` and are
-- unaffected.
ALTER TABLE nexus_detections
    ADD COLUMN sources jsonb NOT NULL DEFAULT '[]'::jsonb;
