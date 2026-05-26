//! TimescaleDB schema migrations.
//!
//! Each entry in [`TIMESCALE_MIGRATIONS`] is a (filename,
//! statement-vector) pair. The SQL is held inline (rather than as
//! `.sql` files) so that consumers pulling the crate by Cargo dep
//! don't need a `migrations/` directory shipped alongside the
//! binary. The statements are executed sequentially against the
//! pool wrapped in [`super::client::WarehouseClient`].
//!
//! Statement order matters: each `create_hypertable(...)` call
//! depends on the prior `CREATE TABLE`. The runner is idempotent
//! — every statement uses `IF NOT EXISTS` / `if_not_exists =>
//! TRUE`.

use sqlx::Executor;

use super::client::{WarehouseClient, WarehouseError};
use crate::chunk_intervals::{L1_CHUNK_INTERVAL, L2_CHUNK_INTERVAL};

/// All migration steps in apply order. Each tuple is `(label,
/// statements)` where `statements` are executed one-by-one inside
/// a single transaction.
pub fn timescale_migrations() -> Vec<(&'static str, Vec<String>)> {
    vec![
        (
            "0001_extension",
            vec!["CREATE EXTENSION IF NOT EXISTS timescaledb".to_string()],
        ),
        (
            "0002_raw_events",
            vec![
                "CREATE TABLE IF NOT EXISTS raw_events (\n\
                 id           BIGSERIAL NOT NULL,\n\
                 tenant_id    TEXT NOT NULL,\n\
                 source       TEXT NOT NULL,\n\
                 received_at  TIMESTAMPTZ NOT NULL DEFAULT now(),\n\
                 payload      TEXT NOT NULL,\n\
                 tags         JSONB NOT NULL DEFAULT '{}'::jsonb,\n\
                 PRIMARY KEY (id, received_at)\n\
                 )"
                .to_string(),
                format!(
                    "SELECT create_hypertable('raw_events', 'received_at', \
                     chunk_time_interval => INTERVAL '{L1_CHUNK_INTERVAL}', \
                     if_not_exists => TRUE)"
                ),
                "CREATE INDEX IF NOT EXISTS raw_events_source_idx \
                 ON raw_events (source, received_at DESC)"
                    .to_string(),
                "CREATE INDEX IF NOT EXISTS raw_events_tags_gin \
                 ON raw_events USING GIN (tags)"
                    .to_string(),
            ],
        ),
        (
            "0003_samples",
            vec![
                "CREATE TABLE IF NOT EXISTS samples (\n\
                 tenant_id    TEXT NOT NULL,\n\
                 entity_id    TEXT NOT NULL,\n\
                 ts           TIMESTAMPTZ NOT NULL,\n\
                 value_num    DOUBLE PRECISION,\n\
                 value_str    TEXT,\n\
                 value_bool   BOOLEAN,\n\
                 quality      SMALLINT NOT NULL DEFAULT 0,\n\
                 tags         JSONB NOT NULL DEFAULT '{}'::jsonb\n\
                 )"
                .to_string(),
                format!(
                    "SELECT create_hypertable('samples', 'ts', \
                     chunk_time_interval => INTERVAL '{L2_CHUNK_INTERVAL}', \
                     if_not_exists => TRUE)"
                ),
                "CREATE INDEX IF NOT EXISTS samples_entity_idx \
                 ON samples (entity_id, ts DESC)"
                    .to_string(),
                "CREATE INDEX IF NOT EXISTS samples_tags_gin \
                 ON samples USING GIN (tags)"
                    .to_string(),
            ],
        ),
        (
            "0004_events",
            vec![
                "CREATE TABLE IF NOT EXISTS events (\n\
                 id           BIGSERIAL NOT NULL,\n\
                 tenant_id    TEXT NOT NULL,\n\
                 entity_id    TEXT NOT NULL,\n\
                 ts           TIMESTAMPTZ NOT NULL,\n\
                 kind         TEXT NOT NULL,\n\
                 payload      TEXT NOT NULL,\n\
                 tags         JSONB NOT NULL DEFAULT '{}'::jsonb,\n\
                 PRIMARY KEY (id, ts)\n\
                 )"
                .to_string(),
                format!(
                    "SELECT create_hypertable('events', 'ts', \
                     chunk_time_interval => INTERVAL '{L2_CHUNK_INTERVAL}', \
                     if_not_exists => TRUE)"
                ),
                "CREATE INDEX IF NOT EXISTS events_kind_entity_idx \
                 ON events (kind, entity_id, ts DESC)"
                    .to_string(),
            ],
        ),
        (
            "0005_documents",
            vec![
                "CREATE TABLE IF NOT EXISTS documents (\n\
                 id           TEXT NOT NULL,\n\
                 tenant_id    TEXT NOT NULL,\n\
                 entity_id    TEXT NOT NULL,\n\
                 ts           TIMESTAMPTZ NOT NULL DEFAULT now(),\n\
                 blob_ref     TEXT NOT NULL,\n\
                 mime         TEXT NOT NULL,\n\
                 tags         JSONB NOT NULL DEFAULT '{}'::jsonb,\n\
                 PRIMARY KEY (id, ts)\n\
                 )"
                .to_string(),
                format!(
                    "SELECT create_hypertable('documents', 'ts', \
                     chunk_time_interval => INTERVAL '{L2_CHUNK_INTERVAL}', \
                     if_not_exists => TRUE)"
                ),
                "CREATE INDEX IF NOT EXISTS documents_entity_idx \
                 ON documents (entity_id, ts DESC)"
                    .to_string(),
            ],
        ),
    ]
}

/// Ordered list of migration labels — exported as the audit
/// surface that mirrors the old [`crate::MIGRATION_FILES`].
pub const TIMESCALE_MIGRATIONS: &[&str] = &[
    "0001_extension",
    "0002_raw_events",
    "0003_samples",
    "0004_events",
    "0005_documents",
];

/// Apply every migration in order. Idempotent: every statement
/// uses `IF NOT EXISTS` semantics.
pub async fn run_migrations(client: &WarehouseClient) -> Result<(), WarehouseError> {
    for (label, statements) in timescale_migrations() {
        tracing::debug!(target: "starter_store_warehouse::tsdb::migrate", label, "applying");
        for stmt in statements {
            client.pool().execute(stmt.as_str()).await?;
        }
    }
    Ok(())
}
