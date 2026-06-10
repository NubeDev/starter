-- User / org preferences (WS-11) — units, datetime format, locale, timezone.
--
-- Ports the `starter-prefs` Postgres schema
-- (crates/starter-prefs/migrations/postgres/0001_starter_prefs.sql) into the
-- nexus metadata DB so the reused `starter_prefs::store::PgPrefsStore` finds its
-- tables. The shape is a faithful copy: NULL on disk means "inherit" per the
-- three-layer resolver contract (R3); the resolver collapses user -> org ->
-- system default at read time.
--
-- Tenant isolation: `workspace_id` IS the nexus `tenant_id`. Unlike the other
-- nexus tables, these are NOT guarded by an `app.tenant_id` RLS GUC, because the
-- reused starter `PgPrefsStore` runs its queries directly on the pool and does
-- not open a `tenant_tx`. Isolation is instead enforced at the route layer: every
-- nexus prefs handler pins `workspace_id = principal.tenant_id` and the store
-- filters every read and write by the composite key `(user_id, workspace_id)`, so
-- a caller can only ever reach its own tenant's rows. The client-supplied `?org=`
-- selector the starter router honours is not exposed by the nexus routes.

CREATE TABLE IF NOT EXISTS starter_prefs_org (
    workspace_id     text PRIMARY KEY,
    timezone         text,
    locale           text,
    language         text,
    unit_system      text,
    temperature_unit text,
    pressure_unit    text,
    speed_unit       text,
    length_unit      text,
    mass_unit        text,
    date_format      text,
    time_format      text,
    week_start       text,
    number_format    text,
    currency         text,
    updated_at       bigint
);

CREATE TABLE IF NOT EXISTS starter_prefs_user (
    user_id          text NOT NULL,
    workspace_id     text NOT NULL,
    timezone         text,
    locale           text,
    language         text,
    unit_system      text,
    temperature_unit text,
    pressure_unit    text,
    speed_unit       text,
    length_unit      text,
    mass_unit        text,
    date_format      text,
    time_format      text,
    week_start       text,
    number_format    text,
    currency         text,
    theme            text,
    updated_at       bigint,
    PRIMARY KEY (user_id, workspace_id)
);

GRANT SELECT, INSERT, UPDATE, DELETE ON starter_prefs_org TO nexus_runtime;
GRANT SELECT, INSERT, UPDATE, DELETE ON starter_prefs_user TO nexus_runtime;
