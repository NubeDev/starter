-- Warehouse SCOPE L2-dim — dictionary bridge to Postgres.
--
-- Per SCOPE / ADR-003:
--   SOURCE(POSTGRESQL(...))      — pull from the dimensions DB
--   LIFETIME(MIN 300 MAX 600)    — W11: 5–10 minute trailing window
--   invalidate_query             — detect updates via max(updated_at)
--   LAYOUT(HASHED())             — full in-memory hash
--
-- Connection knobs are mustache placeholders (double-brace name)
-- that `MigrationRunner` substitutes at apply time from the
-- `pg_source` config; the runner refuses to apply this file if
-- any placeholder is missing. CH parameter substitution
-- (single-brace `name:Type`) is a query-time feature and does not
-- work in DDL — so the substitution is host-side, on the SQL
-- text. Literal mustache braces are avoided in the comments
-- because the runner scans for leftover mustache pairs after
-- substitution and would flag them as unresolved.
--
-- IF NOT EXISTS so re-running the migration is a no-op. Note that
-- CH does not currently support ALTER DICTIONARY for source config
-- changes — you must DROP + CREATE on connection-string changes.
CREATE DICTIONARY IF NOT EXISTS entities_dict (
    id         String,
    kind       String,
    display    String DEFAULT '',
    tags       String DEFAULT '{}'
)
PRIMARY KEY id
SOURCE(POSTGRESQL(
    port {{pg_port}}
    host '{{pg_host}}'
    user '{{pg_user}}'
    password '{{pg_password}}'
    db '{{pg_db}}'
    table 'entities'
    invalidate_query 'SELECT max(updated_at) FROM entities'
))
LIFETIME(MIN 300 MAX 600)
LAYOUT(HASHED());
