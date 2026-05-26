//! [`WarehouseWriter`] adapter that targets a live [`ChClient`].
//!
//! The seven `rubix.warehouse.*` verbs are written against the
//! [`WarehouseWriter`] trait so the production binary can swap a
//! ClickHouse-backed impl in without touching the verb files. This
//! module is that swap: every method translates the verb's intent
//! into the SQL string the underlying `clickhouse` crate already
//! knows how to ship, and snapshots the prior + new state by
//! probing `system.tables`.
//!
//! Connection database is pinned at construction (`database` arg);
//! every query that needs a fully-qualified name uses that prefix
//! so DDL never accidentally targets `default`. The agent boot
//! wiring (`registry::build_tool_registry`) feeds in
//! [`crate::warehouse::store::WarehouseWriter`] via a [`ChClient`]
//! already bound to the rubix database.
//!
//! Append-only / overwrite semantics for *individual* verbs mirror
//! the in-memory impl as closely as the SQL surface allows; see the
//! per-method doc-comments for the small divergences (mostly
//! around "what does `restore` mean when the snapshot DDL is
//! `None` for a mart that has rows").
//!
//! ## Out of scope (today)
//!
//! `list_rules` / `list_marts` fall back to the default `Ok(vec![])`
//! because ClickHouse does not natively distinguish "rule" from
//! "mart" — those are rubix-side classifications layered on top of
//! plain CH tables. A future change can persist the classification
//! into a sidecar table (or `system.tables.comment`) and lift this
//! restriction. `list_tables` is implemented against `system.tables`.

use async_trait::async_trait;
use serde::Deserialize;
use starter_spi::error::{Error, Result};
use starter_store_warehouse::clickhouse;
use starter_store_warehouse::clickhouse::Row;
use starter_store_warehouse::ChClient;

use crate::warehouse::store::{
    WarehouseMartSnapshot, WarehouseRetentionSnapshot, WarehouseRuleSnapshot,
    WarehouseTableSummary, WarehouseWriter,
};

/// [`WarehouseWriter`] backed by a live [`ChClient`] bound to a named
/// ClickHouse database. Construct one per process and share via
/// [`std::sync::Arc`]; the underlying client is itself cheap to
/// share (it wraps an HTTP client pool).
pub struct WarehouseClientWriter {
    client: ChClient,
    database: String,
}

impl std::fmt::Debug for WarehouseClientWriter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WarehouseClientWriter")
            .field("database", &self.database)
            .finish()
    }
}

impl WarehouseClientWriter {
    /// Wrap a [`ChClient`] pinned to `database`. `database` is used
    /// **only** for `system.tables` lookups — the client itself
    /// must already be bound to the same database so unqualified
    /// DDL resolves there.
    pub fn new(client: ChClient, database: impl Into<String>) -> Self {
        Self {
            client,
            database: database.into(),
        }
    }

    /// Probe `system.tables` for the table's `create_table_query`
    /// and `engine_full`. `Ok(None)` when the table does not exist
    /// in the writer's bound database.
    async fn lookup(&self, name: &str) -> Result<Option<TableMeta>> {
        let name_lit = escape_sql_string(name);
        let db_lit = escape_sql_string(&self.database);
        let sql = format!(
            "SELECT create_table_query, engine_full \
             FROM system.tables \
             WHERE database = '{db_lit}' AND name = '{name_lit}' \
             LIMIT 1"
        );
        #[derive(Row, Deserialize)]
        struct R {
            create_table_query: String,
            engine_full: String,
        }
        let mut rows = self
            .client
            .inner()
            .query(&sql)
            .fetch_all::<R>()
            .await
            .map_err(internal)?;
        Ok(rows.pop().map(|r| TableMeta {
            create_table_query: r.create_table_query,
            engine_full: r.engine_full,
        }))
    }

    async fn execute(&self, sql: &str) -> Result<()> {
        self.client
            .inner()
            .query(sql)
            .execute()
            .await
            .map_err(internal)
    }
}

struct TableMeta {
    create_table_query: String,
    engine_full: String,
}

#[async_trait]
impl WarehouseWriter for WarehouseClientWriter {
    async fn show_create_rule(&self, rule_name: &str) -> Result<Option<String>> {
        Ok(self.lookup(rule_name).await?.map(|m| m.create_table_query))
    }

    async fn apply_rule_ddl(
        &self,
        rule_name: &str,
        ddl: &str,
    ) -> Result<(WarehouseRuleSnapshot, WarehouseRuleSnapshot)> {
        let prior = self.lookup(rule_name).await?.map(|m| m.create_table_query);
        self.execute(ddl).await?;
        // Re-probe so the snapshot carries the actual normalised
        // `create_table_query` CH stored — keeps the audit trail
        // honest if the operator sent a slightly different DDL than
        // CH's canonical form.
        let new = self
            .lookup(rule_name)
            .await?
            .map(|m| m.create_table_query)
            .unwrap_or_else(|| ddl.to_owned());
        Ok((
            WarehouseRuleSnapshot {
                rule_name: rule_name.to_owned(),
                ddl: prior,
            },
            WarehouseRuleSnapshot {
                rule_name: rule_name.to_owned(),
                ddl: Some(new),
            },
        ))
    }

    async fn restore_rule(&self, snap: &WarehouseRuleSnapshot) -> Result<()> {
        match &snap.ddl {
            Some(body) => self.execute(body).await,
            None => {
                self.execute(&format!(
                    "DROP TABLE IF EXISTS {}",
                    quote_ident(&snap.rule_name)
                ))
                .await
            }
        }
    }

    async fn show_create_mart(&self, mart_name: &str) -> Result<Option<String>> {
        Ok(self.lookup(mart_name).await?.map(|m| m.create_table_query))
    }

    async fn apply_mart_ddl(
        &self,
        mart_name: &str,
        ddl: &str,
    ) -> Result<(WarehouseMartSnapshot, WarehouseMartSnapshot)> {
        let prior = self.lookup(mart_name).await?.map(|m| m.create_table_query);
        // Idempotent: if the mart was already present, do NOT
        // re-execute the DDL (CH `CREATE TABLE` without
        // `IF NOT EXISTS` would error). The in-memory impl shares
        // the same "first write wins" contract.
        if prior.is_none() {
            self.execute(ddl).await?;
        }
        let new = self
            .lookup(mart_name)
            .await?
            .map(|m| m.create_table_query)
            .unwrap_or_else(|| ddl.to_owned());
        Ok((
            WarehouseMartSnapshot {
                mart_name: mart_name.to_owned(),
                ddl: prior,
            },
            WarehouseMartSnapshot {
                mart_name: mart_name.to_owned(),
                ddl: Some(new),
            },
        ))
    }

    async fn restore_mart(&self, snap: &WarehouseMartSnapshot) -> Result<()> {
        match &snap.ddl {
            Some(body) => self.execute(body).await,
            None => {
                self.execute(&format!(
                    "DROP TABLE IF EXISTS {}",
                    quote_ident(&snap.mart_name)
                ))
                .await
            }
        }
    }

    async fn current_retention(&self, table_name: &str) -> Result<Option<u32>> {
        let meta = self
            .lookup(table_name)
            .await?
            .ok_or_else(|| Error::NotFound {
                what: format!("clickhouse table `{table_name}`"),
            })?;
        Ok(parse_ttl_days(&meta.engine_full))
    }

    async fn apply_retention(
        &self,
        table_name: &str,
        days: u32,
    ) -> Result<(WarehouseRetentionSnapshot, WarehouseRetentionSnapshot)> {
        let prior_meta = self
            .lookup(table_name)
            .await?
            .ok_or_else(|| Error::NotFound {
                what: format!("clickhouse table `{table_name}`"),
            })?;
        let prior_days = parse_ttl_days(&prior_meta.engine_full);

        let ident = quote_ident(table_name);
        if days == 0 {
            self.execute(&format!("ALTER TABLE {ident} REMOVE TTL"))
                .await?;
        } else {
            // The TTL anchor `toDateTime(epoch_ms / 1000)` matches
            // the rubix L1 schema convention (see
            // `migrations/0003_meter_readings_raw/up.sql` and
            // `0002_history/up.sql` — both partition on the same
            // expression). The DELETE action is implicit in MergeTree
            // TTL clauses but we name it for round-trip clarity.
            self.execute(&format!(
                "ALTER TABLE {ident} MODIFY TTL toDateTime(epoch_ms / 1000) + toIntervalDay({days}) DELETE"
            ))
            .await?;
        }
        Ok((
            WarehouseRetentionSnapshot {
                table_name: table_name.to_owned(),
                days: prior_days,
            },
            WarehouseRetentionSnapshot {
                table_name: table_name.to_owned(),
                days: if days == 0 { None } else { Some(days) },
            },
        ))
    }

    async fn restore_retention(&self, snap: &WarehouseRetentionSnapshot) -> Result<()> {
        let ident = quote_ident(&snap.table_name);
        match snap.days {
            None => self.execute(&format!("ALTER TABLE {ident} REMOVE TTL")).await,
            Some(d) => {
                self.execute(&format!(
                    "ALTER TABLE {ident} MODIFY TTL toDateTime(epoch_ms / 1000) + toIntervalDay({d}) DELETE"
                ))
                .await
            }
        }
    }

    // `list_rules` / `list_marts` use the trait default (empty).
    // Rationale: CH does not classify rules vs marts natively; a
    // sidecar registry would be needed to round-trip the
    // distinction. Out of scope for the initial B1 swap; `tables`
    // surface is enough for the operator UI today.

    async fn list_tables(&self) -> Result<Vec<WarehouseTableSummary>> {
        let db_lit = escape_sql_string(&self.database);
        let sql = format!(
            "SELECT name, engine, engine_full, total_rows \
             FROM system.tables \
             WHERE database = '{db_lit}' AND is_temporary = 0 \
             ORDER BY name"
        );
        #[derive(Row, Deserialize)]
        struct R {
            name: String,
            engine: String,
            engine_full: String,
            total_rows: Option<u64>,
        }
        let rows = self
            .client
            .inner()
            .query(&sql)
            .fetch_all::<R>()
            .await
            .map_err(internal)?;
        Ok(rows
            .into_iter()
            .map(|r| WarehouseTableSummary {
                retention_days: parse_ttl_days(&r.engine_full),
                table_name: r.name,
                engine: r.engine,
                row_count: r.total_rows,
            })
            .collect())
    }
}

/// Parse the day count out of a TTL clause in `engine_full`.
///
/// ClickHouse renders MergeTree TTL as
/// `... TTL <expr> + toIntervalDay(N) DELETE ...` (or similar with
/// HOUR / MONTH units). We pick out `toIntervalDay(N)` because the
/// only rubix-side writer (`apply_retention`) only ever emits days,
/// and the verb's interface is `u32 days`. Other units are reported
/// as `None` — surfacing them as "0 days" would be a lie.
fn parse_ttl_days(engine_full: &str) -> Option<u32> {
    let marker = "toIntervalDay(";
    let start = engine_full.find(marker)? + marker.len();
    let rest = &engine_full[start..];
    let end = rest.find(')')?;
    rest[..end].trim().parse::<u32>().ok()
}

/// Escape a single-quoted SQL string literal. The clickhouse crate
/// does not expose a parameter binder for free-form `query()` calls
/// (it requires the `?` placeholder shape which we cannot use for
/// identifiers and is also not portable across the meta-queries we
/// run against `system.tables`).
fn escape_sql_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\'', "''")
}

/// Quote an identifier with backticks, doubling embedded backticks.
/// CH's identifier quoting matches MySQL's.
fn quote_ident(s: &str) -> String {
    format!("`{}`", s.replace('`', "``"))
}

fn internal(e: impl std::error::Error + Send + Sync + 'static) -> Error {
    Error::Internal {
        source: Box::new(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ttl_days_picks_day_count() {
        let engine = "MergeTree() PARTITION BY toYYYYMM(toDateTime(epoch_ms / 1000)) \
                      ORDER BY (tenant_id, meter_id, epoch_ms) \
                      TTL toDateTime(epoch_ms / 1000) + toIntervalDay(14) DELETE \
                      SETTINGS index_granularity = 8192";
        assert_eq!(parse_ttl_days(engine), Some(14));
    }

    #[test]
    fn parse_ttl_days_none_when_no_ttl_clause() {
        let engine = "MergeTree() PARTITION BY toYYYYMM(toDateTime(epoch_ms / 1000)) \
                      ORDER BY (tenant_id, meter_id, epoch_ms) \
                      SETTINGS index_granularity = 8192";
        assert!(parse_ttl_days(engine).is_none());
    }

    #[test]
    fn parse_ttl_days_none_for_non_day_unit() {
        // Hour-granularity TTL — we honestly report None rather than
        // round-down to 0; the verb's contract is days.
        let engine = "MergeTree() ORDER BY x TTL t + toIntervalHour(12) DELETE";
        assert!(parse_ttl_days(engine).is_none());
    }

    #[test]
    fn escape_sql_string_doubles_quotes_and_backslashes() {
        assert_eq!(escape_sql_string("a'b"), "a''b");
        assert_eq!(escape_sql_string("a\\b"), "a\\\\b");
    }

    #[test]
    fn quote_ident_wraps_in_backticks_and_doubles_them() {
        assert_eq!(quote_ident("meter_readings_raw"), "`meter_readings_raw`");
        assert_eq!(quote_ident("weird`name"), "`weird``name`");
    }
}
