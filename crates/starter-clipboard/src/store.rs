//! [`ClipboardStore`] trait + an in-memory impl.
//!
//! TODO(security): a production backend should HMAC-sign each entry
//! with a key fetched via `starter_spi::SecretStore` under
//! `starter.clipboard.hmac` (SCOPE §"Crates" / §"Open questions
//! resolved"). The in-memory impl is intentionally unsigned — it is
//! the dev-mode default while the SQLite/PG backends land.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use starter_spi::auth::Principal;
use starter_spi::{Error, Result};
use tokio::sync::Mutex;

/// One persisted clipboard entry. The `payload` is the `after`
/// snapshot of the source resource.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardEntry {
    /// Server-assigned entry id.
    pub id: String,
    /// Subject of the principal that owns this entry. Scope is
    /// per-principal — no cross-account paste.
    pub principal_id: String,
    /// Source `ResourceRef::kind`.
    pub resource_kind: String,
    /// `after` snapshot of the source resource.
    pub payload: serde_json::Value,
    /// When the entry was copied.
    pub created_at: DateTime<Utc>,
    /// Hard expiry; backends MUST refuse to return expired entries.
    pub expires_at: DateTime<Utc>,
}

/// Persistence seam.
#[async_trait]
pub trait ClipboardStore: Send + Sync {
    /// Persist a fresh entry.
    async fn put(&self, entry: ClipboardEntry) -> Result<()>;

    /// Load by id. Returns `None` if missing or expired.
    async fn get(&self, principal_id: &str, id: &str) -> Result<Option<ClipboardEntry>>;
}

/// Dev-mode in-memory store. Not durable, not HMAC-signed.
#[derive(Default)]
pub struct InMemoryClipboard {
    entries: Mutex<HashMap<String, ClipboardEntry>>,
}

impl InMemoryClipboard {
    /// Empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Convenience constructor matching the production seam.
    pub fn shared() -> Arc<Self> {
        Arc::new(Self::default())
    }
}

#[async_trait]
impl ClipboardStore for InMemoryClipboard {
    async fn put(&self, entry: ClipboardEntry) -> Result<()> {
        let mut entries = self.entries.lock().await;
        entries.insert(entry.id.clone(), entry);
        Ok(())
    }

    async fn get(&self, principal_id: &str, id: &str) -> Result<Option<ClipboardEntry>> {
        let entries = self.entries.lock().await;
        let Some(entry) = entries.get(id) else {
            return Ok(None);
        };
        if entry.principal_id != principal_id {
            // Treat cross-principal lookups as not-found so we don't
            // leak entry existence to other principals.
            return Ok(None);
        }
        if entry.expires_at <= Utc::now() {
            return Ok(None);
        }
        Ok(Some(entry.clone()))
    }
}

/// Build a fresh entry from `(principal, kind, payload)` with the
/// default TTL.
pub fn new_entry(
    principal: &Principal,
    resource_kind: impl Into<String>,
    payload: serde_json::Value,
    ttl: Duration,
) -> Result<ClipboardEntry> {
    let now = Utc::now();
    let expires_at = now.checked_add_signed(ttl).ok_or_else(|| Error::Invalid {
        message: "clipboard ttl overflow".into(),
    })?;
    Ok(ClipboardEntry {
        id: uuid::Uuid::now_v7().to_string(),
        principal_id: principal.subject.clone(),
        resource_kind: resource_kind.into(),
        payload,
        created_at: now,
        expires_at,
    })
}
