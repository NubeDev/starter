//! Clipboard service: copy + paste + duplicate.
//!
//! `copy` persists the `after` snapshot of a resource keyed by
//! `(principal, kind)`. `paste` looks up the snapshot and calls
//! [`Reversible::clone_with`], so the resulting rows land under one
//! `group_id` and undo collapses paste into a single step (SCOPE
//! §"Feature mapping").

use std::sync::Arc;

use chrono::Duration;
use starter_spi::auth::Principal;
use starter_spi::authz::ResourceRef;
use starter_spi::changelog::{ChangeTx, Reversible};
use starter_spi::{Error, Result};

use crate::store::{new_entry, ClipboardEntry, ClipboardStore};

/// Default clipboard TTL — one hour. SCOPE leaves the value to the
/// consumer; the default keeps a dev-mode UI from accumulating
/// stale entries.
pub const DEFAULT_TTL_SECS: i64 = 60 * 60;

/// High-level copy / paste / duplicate.
pub struct ClipboardService {
    store: Arc<dyn ClipboardStore>,
    ttl: Duration,
}

impl ClipboardService {
    /// Wrap a store with the default TTL.
    pub fn new(store: Arc<dyn ClipboardStore>) -> Self {
        Self {
            store,
            ttl: Duration::seconds(DEFAULT_TTL_SECS),
        }
    }

    /// Override the TTL.
    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl;
        self
    }

    /// Look up an entry by id under the principal's scope. `None`
    /// when the entry is missing or expired. Exposed for transports
    /// that need to inspect `resource_kind` before opening a
    /// recorder transaction (paste dispatches by kind).
    pub async fn store_get(&self, principal_id: &str, id: &str) -> Result<Option<ClipboardEntry>> {
        self.store.get(principal_id, id).await
    }

    /// Persist a copy of `payload` (the `after` snapshot of a
    /// resource of `kind`). Returns the new entry id.
    pub async fn copy(
        &self,
        principal: &Principal,
        kind: impl Into<String>,
        payload: serde_json::Value,
    ) -> Result<String> {
        let entry = new_entry(principal, kind, payload, self.ttl)?;
        let id = entry.id.clone();
        self.store.put(entry).await?;
        Ok(id)
    }

    /// Paste a clipboard entry as new rows. The reversible impl
    /// MUST emit one [`ChangeTx::record`] per new row so paste
    /// collapses into a single undo step.
    ///
    /// Returns the `Vec<ResourceRef>` of newly created rows.
    pub async fn paste(
        &self,
        principal: &Principal,
        reversible: &dyn Reversible,
        tx: &dyn ChangeTx,
        entry_id: &str,
        overrides: serde_json::Value,
    ) -> Result<Vec<ResourceRef>> {
        let entry = self
            .store
            .get(&principal.subject, entry_id)
            .await?
            .ok_or_else(|| Error::NotFound {
                what: format!("clipboard entry {entry_id}"),
            })?;

        if entry.resource_kind != reversible.kind() {
            return Err(Error::Invalid {
                message: format!(
                    "clipboard kind {} does not match reversible kind {}",
                    entry.resource_kind,
                    reversible.kind()
                ),
            });
        }

        // The clipboard payload is the source `after` snapshot. We
        // pass `entry.payload` to `clone_with` as the source view via
        // a synthetic `ResourceRef` and let the implementation merge
        // overrides on top. Implementations that need richer source
        // context can load by id themselves — the `ResourceRef`
        // gives them the kind + id.
        let src = ResourceRef {
            kind: entry.resource_kind.clone(),
            id: entry
                .payload
                .get("id")
                .and_then(|v| v.as_str())
                .map(str::to_owned),
            owner: None,
            tenant: None,
        };

        reversible
            .clone_with(tx, &src, merge_overrides(&entry, overrides))
            .await
    }

    /// Convenience: duplicate = copy + paste in one call. The
    /// reversible impl receives `src` directly (no clipboard
    /// round-trip) and `overrides` verbatim.
    pub async fn duplicate(
        &self,
        reversible: &dyn Reversible,
        tx: &dyn ChangeTx,
        src: &ResourceRef,
        overrides: serde_json::Value,
    ) -> Result<Vec<ResourceRef>> {
        reversible.clone_with(tx, src, overrides).await
    }
}

/// Merge the clipboard payload with the caller's overrides. The
/// override keys win. Non-object payloads pass `overrides` through
/// unchanged so the reversible impl can still see them.
fn merge_overrides(entry: &ClipboardEntry, overrides: serde_json::Value) -> serde_json::Value {
    match (&entry.payload, overrides) {
        (serde_json::Value::Object(base), serde_json::Value::Object(over)) => {
            let mut merged = base.clone();
            for (k, v) in over {
                merged.insert(k, v);
            }
            serde_json::Value::Object(merged)
        }
        (_, over) => over,
    }
}
