//! Backing store for the four `rubix.insights.rule.*` verbs.
//!
//! A small trait so the production binary can swap a PG-backed
//! impl in without touching the verb files. The [`InMemoryInsightsStore`]
//! variant is enough for the in-process smoke session and unit
//! tests; mutations land in a `Mutex<HashMap>` and do not survive
//! restart.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use starter_spi::error::Result;

/// One insights-rule row.
#[derive(Debug, Clone, PartialEq)]
pub struct InsightsRuleRow {
    /// Stable id (resource key).
    pub rule_id: String,
    /// Human-facing name. Defaults to `rule_id` when unset.
    pub name: String,
    /// Whether the rule is active.
    pub enabled: bool,
    /// Raw YAML body.
    pub body_yaml: String,
    /// Epoch milliseconds of the most recent write.
    pub updated_at_ms: i64,
}

/// Outcome of an upsert.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpsertOutcome {
    /// The rule did not previously exist.
    Created,
    /// The rule existed and the body was replaced.
    Replaced,
}

/// Outcome of a toggle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToggleOutcome {
    /// The rule existed and the active flag now matches the
    /// requested state (may have been a no-op).
    Applied,
    /// The rule id was unknown.
    NotFound,
}

/// Persistence surface for the insights tool family.
#[async_trait]
pub trait InsightsRuleStore: Send + Sync {
    /// Enumerate every rule the store holds, sorted by `rule_id`.
    async fn list(&self) -> Result<Vec<InsightsRuleRow>>;
    /// Idempotent upsert. New rules default to `enabled = true`.
    async fn upsert(
        &self,
        rule_id: &str,
        body_yaml: &str,
        now_ms: i64,
    ) -> Result<UpsertOutcome>;
    /// Flip the enabled flag. Returns [`ToggleOutcome::NotFound`]
    /// when the id is unknown so the verb can surface a
    /// `rubix.insights.rule.not_found` diagnostic instead of a
    /// generic error.
    async fn set_enabled(
        &self,
        rule_id: &str,
        enabled: bool,
        now_ms: i64,
    ) -> Result<ToggleOutcome>;
}

/// In-memory [`InsightsRuleStore`] for tests and the in-process
/// smoke session.
#[derive(Default, Clone)]
pub struct InMemoryInsightsStore {
    rows: Arc<Mutex<HashMap<String, InsightsRuleRow>>>,
}

impl InMemoryInsightsStore {
    /// New empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Test helper: row count.
    pub fn len(&self) -> usize {
        self.rows.lock().expect("insights store mutex poisoned").len()
    }

    /// Test helper: emptiness probe.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[async_trait]
impl InsightsRuleStore for InMemoryInsightsStore {
    async fn list(&self) -> Result<Vec<InsightsRuleRow>> {
        let mut rows: Vec<InsightsRuleRow> = self
            .rows
            .lock()
            .expect("insights store mutex poisoned")
            .values()
            .cloned()
            .collect();
        rows.sort_by(|a, b| a.rule_id.cmp(&b.rule_id));
        Ok(rows)
    }

    async fn upsert(
        &self,
        rule_id: &str,
        body_yaml: &str,
        now_ms: i64,
    ) -> Result<UpsertOutcome> {
        let mut guard = self.rows.lock().expect("insights store mutex poisoned");
        let outcome = if guard.contains_key(rule_id) {
            UpsertOutcome::Replaced
        } else {
            UpsertOutcome::Created
        };
        let row = InsightsRuleRow {
            rule_id: rule_id.to_owned(),
            name: rule_id.to_owned(),
            enabled: true,
            body_yaml: body_yaml.to_owned(),
            updated_at_ms: now_ms,
        };
        guard.insert(rule_id.to_owned(), row);
        Ok(outcome)
    }

    async fn set_enabled(
        &self,
        rule_id: &str,
        enabled: bool,
        now_ms: i64,
    ) -> Result<ToggleOutcome> {
        let mut guard = self.rows.lock().expect("insights store mutex poisoned");
        match guard.get_mut(rule_id) {
            Some(row) => {
                row.enabled = enabled;
                row.updated_at_ms = now_ms;
                Ok(ToggleOutcome::Applied)
            }
            None => Ok(ToggleOutcome::NotFound),
        }
    }
}

/// Helper: epoch milliseconds for the verb call sites.
pub fn now_epoch_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
