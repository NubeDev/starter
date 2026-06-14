-- migrate-rename-remote.down.sql
--
-- Reverse `migrate-rename-remote.up.sql`: rename the host-prefixed
-- tables back to their bare Rubix-OS names and drop the host-owned
-- `tenant_id` column we added. Idempotent; run inside a transaction.
--
-- Only the `tenant_id` column is dropped — every original column is
-- left untouched, so this is a clean round-trip back to the source
-- schema.

DO $$
DECLARE
    t     text;
    src   text;  -- host-prefixed (current) name
    dst   text;  -- bare (restored) name
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
        src := 'com_nubeio_rubixos__' || t;
        dst := t;

        IF EXISTS (
            SELECT 1 FROM information_schema.tables
            WHERE table_schema = 'public' AND table_name = src
        ) THEN
            EXECUTE format('ALTER TABLE public.%I DROP COLUMN IF EXISTS tenant_id', src);
            EXECUTE format('ALTER TABLE public.%I RENAME TO %I', src, dst);
            RAISE NOTICE 'reverted public.% -> public.%', src, dst;
        ELSE
            RAISE NOTICE 'nothing to revert for public.% (absent)', src;
        END IF;
    END LOOP;
END
$$;
