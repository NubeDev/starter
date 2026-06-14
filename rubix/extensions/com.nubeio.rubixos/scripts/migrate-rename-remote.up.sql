-- migrate-rename-remote.up.sql
--
-- Adopt an existing Rubix-OS Postgres/TimescaleDB instance into the
-- `com.nubeio.rubixos` extension's table convention WITHOUT copying
-- data. For each of the 8 source tables in `public` this:
--
--   1. adds the host-owned `tenant_id TEXT NOT NULL DEFAULT 'system'`
--      column the templates filter on (PG14 fast-default: metadata
--      only, no heap/chunk rewrite — safe even on the 17M-row
--      `histories` hypertable), then
--   2. renames `public.<name>` -> `public.com_nubeio_rubixos__<name>`
--      (instant, metadata-only). `histories` stays a hypertable
--      across the rename, so `approximate_row_count()` /
--      `time_bucket()` in the templates keep working.
--
-- Idempotent and re-runnable: already-migrated tables are skipped,
-- and a missing source table only warns. Run inside a transaction
-- (the runner wraps it) so a mid-way failure rolls the whole thing
-- back. Reverse with `migrate-rename-remote.down.sql`.
--
-- NOTE: this renames the base tables in place. Only run it when no
-- live Rubix-OS process is still writing to `public.points` /
-- `public.histories` / etc. — confirmed static for this dataset.

DO $$
DECLARE
    t     text;
    src   text;
    dst   text;
    names text[] := ARRAY[
        'histories',
        'points',
        'device_tags',
        'device_meta_tags',
        'network_tags',
        'network_meta_tags',
        'point_tags',
        'point_meta_tags'
    ];
BEGIN
    FOREACH t IN ARRAY names LOOP
        src := t;
        dst := 'com_nubeio_rubixos__' || t;

        IF EXISTS (
            SELECT 1 FROM information_schema.tables
            WHERE table_schema = 'public' AND table_name = src
        ) THEN
            -- 1. host-owned tenant scoping column (fast default).
            EXECUTE format(
                'ALTER TABLE public.%I ADD COLUMN IF NOT EXISTS tenant_id text NOT NULL DEFAULT %L',
                src, 'system'
            );
            -- 2. adopt the host-prefixed name.
            EXECUTE format('ALTER TABLE public.%I RENAME TO %I', src, dst);
            RAISE NOTICE 'migrated public.% -> public.%', src, dst;

        ELSIF EXISTS (
            SELECT 1 FROM information_schema.tables
            WHERE table_schema = 'public' AND table_name = dst
        ) THEN
            -- Already migrated on a prior run — make sure tenant_id is
            -- present (covers a half-applied state) and move on.
            EXECUTE format(
                'ALTER TABLE public.%I ADD COLUMN IF NOT EXISTS tenant_id text NOT NULL DEFAULT %L',
                dst, 'system'
            );
            RAISE NOTICE 'already migrated: public.% (skipped)', dst;

        ELSE
            RAISE WARNING 'source table public.% not found and public.% absent; skipping', src, dst;
        END IF;
    END LOOP;
END
$$;
