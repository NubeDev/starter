//! Backing store + [`Reversible`] glue for the clickhouse-ruler
//! verbs.
//!
//! The three write verbs (`clickhouse.rule.write`,
//! `clickhouse.mart.create`, `clickhouse.retention.set`) talk to a
//! small [`ChWriter`] trait so the production binary can swap a
//! `ChClient`-backed impl in without touching the verb files. The
//! [`InMemoryChWriter`] fake is enough for unit tests, the agent
//! loop's recorded-LLM integration tests, and the smoke session
//! that lights the verbs end-to-end. The CH-backed impl lands in a
//! follow-up phase that wires `starter-store-clickhouse::ChClient`
//! to the same trait.
//!
//! Snapshots are JSON blobs carried in [`Change::before`] /
//! [`Change::after`]. Three resource kinds are registered, one per
//! verb, each with its own snapshot shape; see
//! [docs/design/clickhouse-rules/](../../../../docs/design/clickhouse-rules/README.md)
//! §"Snapshot shape".

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use starter_spi::authz::ResourceRef;
use starter_spi::changelog::{Change, ChangeTx, Reversible};
use starter_spi::error::{Error, Result};

/// Resource-kind discriminator for rule rows.
pub const CH_RULE_KIND: &str = "clickhouse_rule";
/// Resource-kind discriminator for mart rows.
pub const CH_MART_KIND: &str = "clickhouse_mart";
/// Resource-kind discriminator for retention rows.
pub const CH_RETENTION_KIND: &str = "clickhouse_retention";

/// Snapshot of a CH derived-state rule. `ddl` is the body returned
/// by `SHOW CREATE TABLE <rule_name>` at snapshot time, or `None`
/// when the rule did not exist (the inverse op for that case is
/// `DROP TABLE IF EXISTS`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChRuleSnapshot {
    /// Fully-qualified rule name (resource id).
    pub rule_name: String,
    /// Prior DDL body, or `None` when the rule was absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ddl: Option<String>,
}

/// Snapshot of a CH mart table. Same shape as [`ChRuleSnapshot`];
/// the dedicated type keeps the resource kinds distinct in the
/// undo audit trail and lets the `Reversible` impl pick the right
/// inverse-op path (DROP TABLE for absent prior, replay DDL for
/// present prior).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChMartSnapshot {
    /// Fully-qualified mart name (resource id).
    pub mart_name: String,
    /// Prior DDL body, or `None` when the mart was absent. When
    /// `None`, undo issues `DROP TABLE IF EXISTS` — the schema is
    /// restored to its pre-create state but rows ingested between
    /// the create and the undo are lost.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ddl: Option<String>,
}

/// Snapshot of a CH table's TTL. `days = None` means the table had
/// no TTL clause at snapshot time.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChRetentionSnapshot {
    /// Fully-qualified table name (resource id).
    pub table_name: String,
    /// Prior retention in days; `None` when no TTL was set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub days: Option<u32>,
}

/// Persistence + DDL surface the three CH write verbs target.
///
/// Method shape mirrors the snapshot-before-write contract:
/// every mutator returns `(prior_snapshot, new_snapshot)` so the
/// verb can stamp the `Change` envelope without a second probe.
#[async_trait]
pub trait ChWriter: Send + Sync {
    /// Run `SHOW CREATE TABLE <rule_name>`; `Ok(None)` when the
    /// object does not exist.
    async fn show_create_rule(&self, rule_name: &str) -> Result<Option<String>>;
    /// Execute the rule DDL and return `(prior, new)` snapshots.
    async fn apply_rule_ddl(
        &self,
        rule_name: &str,
        ddl: &str,
    ) -> Result<(ChRuleSnapshot, ChRuleSnapshot)>;
    /// Restore a rule from a snapshot. `snap.ddl = None` ⇒
    /// `DROP TABLE IF EXISTS`; otherwise replay the DDL verbatim
    /// (the verb's responsibility is to ensure the snapshot DDL
    /// parses on this CH version).
    async fn restore_rule(&self, snap: &ChRuleSnapshot) -> Result<()>;

    /// Run `SHOW CREATE TABLE <mart_name>`; `Ok(None)` when absent.
    async fn show_create_mart(&self, mart_name: &str) -> Result<Option<String>>;
    /// Execute the mart DDL. Returns `(prior, new)` where the
    /// `was_already_present` boolean falls out of `prior.ddl.is_some()`.
    async fn apply_mart_ddl(
        &self,
        mart_name: &str,
        ddl: &str,
    ) -> Result<(ChMartSnapshot, ChMartSnapshot)>;
    /// Restore a mart from a snapshot. `snap.ddl = None` ⇒
    /// `DROP TABLE IF EXISTS` (the data-loss path).
    async fn restore_mart(&self, snap: &ChMartSnapshot) -> Result<()>;

    /// Probe `system.tables` for the current TTL on `table_name`.
    /// Returns `Ok(None)` when the table has no TTL clause or
    /// `Err(NotFound)` when the table itself does not exist.
    async fn current_retention(&self, table_name: &str) -> Result<Option<u32>>;
    /// Apply the retention. Returns `(prior, new)`.
    async fn apply_retention(
        &self,
        table_name: &str,
        days: u32,
    ) -> Result<(ChRetentionSnapshot, ChRetentionSnapshot)>;
    /// Restore retention from a snapshot.
    async fn restore_retention(&self, snap: &ChRetentionSnapshot) -> Result<()>;
}

/// In-memory [`ChWriter`] for tests and the in-process smoke
/// session. Tracks the latest DDL body per (kind, name) and the
/// current TTL per table; ignores SQL semantics.
#[derive(Default, Clone)]
pub struct InMemoryChWriter {
    rules: Arc<Mutex<HashMap<String, String>>>,
    marts: Arc<Mutex<HashMap<String, String>>>,
    ttl: Arc<Mutex<HashMap<String, u32>>>,
}

impl InMemoryChWriter {
    /// New empty writer.
    pub fn new() -> Self {
        Self::default()
    }

    /// Seed an existing mart — handy for tests that exercise the
    /// `was_already_present` path.
    pub fn seed_mart(&self, mart_name: &str, ddl: &str) {
        self.marts
            .lock()
            .expect("ChWriter mutex poisoned")
            .insert(mart_name.to_owned(), ddl.to_owned());
    }

    /// Seed an existing TTL.
    pub fn seed_retention(&self, table_name: &str, days: u32) {
        self.ttl
            .lock()
            .expect("ChWriter mutex poisoned")
            .insert(table_name.to_owned(), days);
    }

    /// Inspect the current rule DDL (test helper).
    pub fn rule(&self, rule_name: &str) -> Option<String> {
        self.rules
            .lock()
            .expect("ChWriter mutex poisoned")
            .get(rule_name)
            .cloned()
    }

    /// Inspect the current mart DDL (test helper).
    pub fn mart(&self, mart_name: &str) -> Option<String> {
        self.marts
            .lock()
            .expect("ChWriter mutex poisoned")
            .get(mart_name)
            .cloned()
    }

    /// Inspect the current TTL (test helper).
    pub fn retention(&self, table_name: &str) -> Option<u32> {
        self.ttl
            .lock()
            .expect("ChWriter mutex poisoned")
            .get(table_name)
            .copied()
    }
}

#[async_trait]
impl ChWriter for InMemoryChWriter {
    async fn show_create_rule(&self, rule_name: &str) -> Result<Option<String>> {
        Ok(self.rule(rule_name))
    }
    async fn apply_rule_ddl(
        &self,
        rule_name: &str,
        ddl: &str,
    ) -> Result<(ChRuleSnapshot, ChRuleSnapshot)> {
        let prior = ChRuleSnapshot {
            rule_name: rule_name.to_owned(),
            ddl: self.rule(rule_name),
        };
        self.rules
            .lock()
            .expect("ChWriter mutex poisoned")
            .insert(rule_name.to_owned(), ddl.to_owned());
        let new = ChRuleSnapshot {
            rule_name: rule_name.to_owned(),
            ddl: Some(ddl.to_owned()),
        };
        Ok((prior, new))
    }
    async fn restore_rule(&self, snap: &ChRuleSnapshot) -> Result<()> {
        let mut guard = self.rules.lock().expect("ChWriter mutex poisoned");
        match &snap.ddl {
            Some(body) => {
                guard.insert(snap.rule_name.clone(), body.clone());
            }
            None => {
                guard.remove(&snap.rule_name);
            }
        }
        Ok(())
    }

    async fn show_create_mart(&self, mart_name: &str) -> Result<Option<String>> {
        Ok(self.mart(mart_name))
    }
    async fn apply_mart_ddl(
        &self,
        mart_name: &str,
        ddl: &str,
    ) -> Result<(ChMartSnapshot, ChMartSnapshot)> {
        let prior_ddl = self.mart(mart_name);
        let prior = ChMartSnapshot {
            mart_name: mart_name.to_owned(),
            ddl: prior_ddl.clone(),
        };
        // Idempotent: if the mart was already present, keep the
        // current DDL — `CREATE TABLE IF NOT EXISTS` semantics.
        let new_body = prior_ddl.clone().unwrap_or_else(|| ddl.to_owned());
        self.marts
            .lock()
            .expect("ChWriter mutex poisoned")
            .insert(mart_name.to_owned(), new_body.clone());
        let new = ChMartSnapshot {
            mart_name: mart_name.to_owned(),
            ddl: Some(new_body),
        };
        Ok((prior, new))
    }
    async fn restore_mart(&self, snap: &ChMartSnapshot) -> Result<()> {
        let mut guard = self.marts.lock().expect("ChWriter mutex poisoned");
        match &snap.ddl {
            Some(body) => {
                guard.insert(snap.mart_name.clone(), body.clone());
            }
            None => {
                // Inverse of "create a brand-new mart" is DROP TABLE
                // IF EXISTS. The mock just drops the entry; the
                // production impl issues the DDL. See the design doc
                // for the data-loss caveat.
                guard.remove(&snap.mart_name);
            }
        }
        Ok(())
    }

    async fn current_retention(&self, table_name: &str) -> Result<Option<u32>> {
        Ok(self.retention(table_name))
    }
    async fn apply_retention(
        &self,
        table_name: &str,
        days: u32,
    ) -> Result<(ChRetentionSnapshot, ChRetentionSnapshot)> {
        let prior = ChRetentionSnapshot {
            table_name: table_name.to_owned(),
            days: self.retention(table_name),
        };
        let mut guard = self.ttl.lock().expect("ChWriter mutex poisoned");
        if days == 0 {
            guard.remove(table_name);
        } else {
            guard.insert(table_name.to_owned(), days);
        }
        let new = ChRetentionSnapshot {
            table_name: table_name.to_owned(),
            days: if days == 0 { None } else { Some(days) },
        };
        Ok((prior, new))
    }
    async fn restore_retention(&self, snap: &ChRetentionSnapshot) -> Result<()> {
        let mut guard = self.ttl.lock().expect("ChWriter mutex poisoned");
        match snap.days {
            Some(d) => {
                guard.insert(snap.table_name.clone(), d);
            }
            None => {
                guard.remove(&snap.table_name);
            }
        }
        Ok(())
    }
}

// ----- Reversible impls ----------------------------------------------------

/// [`Reversible`] for the `"clickhouse_rule"` kind.
pub struct ChRuleReversible {
    writer: Arc<dyn ChWriter>,
}

impl ChRuleReversible {
    /// Wrap the shared writer.
    pub fn new(writer: Arc<dyn ChWriter>) -> Self {
        Self { writer }
    }
}

#[async_trait]
impl Reversible for ChRuleReversible {
    fn kind(&self) -> &'static str {
        CH_RULE_KIND
    }

    async fn apply_inverse(&self, ch: &Change) -> Result<()> {
        let prior = parse::<ChRuleSnapshot>(ch.before.as_ref(), "before")?;
        self.writer.restore_rule(&prior).await
    }

    async fn apply_forward(&self, ch: &Change) -> Result<()> {
        let after = parse::<ChRuleSnapshot>(ch.after.as_ref(), "after")?;
        self.writer.restore_rule(&after).await
    }

    async fn clone_with(
        &self,
        _tx: &dyn ChangeTx,
        _src: &ResourceRef,
        _overrides: serde_json::Value,
    ) -> Result<Vec<ResourceRef>> {
        Err(Error::Invalid {
            message: "clickhouse_rule does not support clone".to_owned(),
        })
    }
}

/// [`Reversible`] for the `"clickhouse_mart"` kind.
pub struct ChMartReversible {
    writer: Arc<dyn ChWriter>,
}

impl ChMartReversible {
    /// Wrap the shared writer.
    pub fn new(writer: Arc<dyn ChWriter>) -> Self {
        Self { writer }
    }
}

#[async_trait]
impl Reversible for ChMartReversible {
    fn kind(&self) -> &'static str {
        CH_MART_KIND
    }

    async fn apply_inverse(&self, ch: &Change) -> Result<()> {
        let prior = parse::<ChMartSnapshot>(ch.before.as_ref(), "before")?;
        self.writer.restore_mart(&prior).await
    }

    async fn apply_forward(&self, ch: &Change) -> Result<()> {
        let after = parse::<ChMartSnapshot>(ch.after.as_ref(), "after")?;
        self.writer.restore_mart(&after).await
    }

    async fn clone_with(
        &self,
        _tx: &dyn ChangeTx,
        _src: &ResourceRef,
        _overrides: serde_json::Value,
    ) -> Result<Vec<ResourceRef>> {
        Err(Error::Invalid {
            message: "clickhouse_mart does not support clone".to_owned(),
        })
    }
}

/// [`Reversible`] for the `"clickhouse_retention"` kind.
pub struct ChRetentionReversible {
    writer: Arc<dyn ChWriter>,
}

impl ChRetentionReversible {
    /// Wrap the shared writer.
    pub fn new(writer: Arc<dyn ChWriter>) -> Self {
        Self { writer }
    }
}

#[async_trait]
impl Reversible for ChRetentionReversible {
    fn kind(&self) -> &'static str {
        CH_RETENTION_KIND
    }

    async fn apply_inverse(&self, ch: &Change) -> Result<()> {
        let prior = parse::<ChRetentionSnapshot>(ch.before.as_ref(), "before")?;
        self.writer.restore_retention(&prior).await
    }

    async fn apply_forward(&self, ch: &Change) -> Result<()> {
        let after = parse::<ChRetentionSnapshot>(ch.after.as_ref(), "after")?;
        self.writer.restore_retention(&after).await
    }

    async fn clone_with(
        &self,
        _tx: &dyn ChangeTx,
        _src: &ResourceRef,
        _overrides: serde_json::Value,
    ) -> Result<Vec<ResourceRef>> {
        Err(Error::Invalid {
            message: "clickhouse_retention does not support clone".to_owned(),
        })
    }
}

fn parse<T: for<'de> Deserialize<'de>>(payload: Option<&Value>, field: &str) -> Result<T> {
    let v = payload.ok_or_else(|| Error::Invalid {
        message: format!("ch reversible: Change::{field} is None"),
    })?;
    serde_json::from_value::<T>(v.clone()).map_err(|e| Error::Invalid {
        message: format!("ch reversible: Change::{field} parse: {e}"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mart_create_then_undo_drops_when_prior_was_absent() {
        let w = InMemoryChWriter::new();
        let (prior, _new) = w
            .apply_mart_ddl("system_disk_history", "CREATE TABLE ...")
            .await
            .unwrap();
        assert!(prior.ddl.is_none(), "fresh create has no prior body");
        assert!(w.mart("system_disk_history").is_some());
        // Undo: restore the empty prior → DROP TABLE.
        w.restore_mart(&prior).await.unwrap();
        assert!(
            w.mart("system_disk_history").is_none(),
            "restoring an empty snapshot drops the mart",
        );
    }

    #[tokio::test]
    async fn retention_unchanged_when_value_matches_current() {
        let w = InMemoryChWriter::new();
        w.seed_retention("system_disk_history", 30);
        let (prior, new) = w.apply_retention("system_disk_history", 30).await.unwrap();
        assert_eq!(prior.days, Some(30));
        assert_eq!(new.days, Some(30));
    }
}
