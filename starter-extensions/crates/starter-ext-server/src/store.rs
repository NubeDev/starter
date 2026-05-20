//! Enable/disable persistence.
//!
//! SCOPE.md "Decisions made — enable/disable persistence model": **one DB
//! row per extension id**, queried at host boot to decide which records
//! to bring up. This crate doesn't own the DB; it owns the trait that
//! consumers implement against their own storage.
//!
//! v0.1 ships one concrete impl — [`InMemoryEnablementStore`] — so
//! `TestApp` and the smoke tests work without a database; a real
//! consumer plugs in a sqlx/sqlite/postgres-backed impl.

use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use starter_ext_spi::ExtensionId;

/// Whether an extension is currently enabled or disabled. Default for a
/// freshly-loaded extension is [`EnablementState::Enabled`] — disabling
/// is an explicit operator action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnablementState {
    /// Extension is permitted to run; process-flavour records have a
    /// live supervisor and contribution adapters mount their routes.
    Enabled,
    /// Extension is suppressed; process-flavour records have no
    /// supervisor and contribution adapters skip their routes.
    Disabled,
}

/// Persistence seam for enable/disable state.
///
/// Implementations must be safe to call from any axum handler — i.e.
/// `Send + Sync + 'static` and using async for any IO.
#[async_trait]
pub trait EnablementStore: Send + Sync + 'static {
    /// Look up the persisted state for one id. Returning `Ok(None)` is
    /// the "no row yet" case; the admin endpoint then treats the
    /// extension as [`EnablementState::Enabled`] (the default).
    async fn get(&self, id: &ExtensionId) -> Result<Option<EnablementState>, StoreError>;

    /// Write the state for one id. The store is responsible for an
    /// atomic upsert against the underlying storage.
    async fn set(&self, id: &ExtensionId, state: EnablementState) -> Result<(), StoreError>;
}

/// Error type returned by [`EnablementStore`] implementations. Surfaced
/// as HTTP 500 by the admin endpoints; the human message is logged but
/// not echoed to the caller.
#[derive(Debug, thiserror::Error)]
#[error("enablement store error: {0}")]
pub struct StoreError(pub String);

impl StoreError {
    /// Construct a store error from any displayable type.
    pub fn new(msg: impl std::fmt::Display) -> Self {
        Self(msg.to_string())
    }
}

/// Default in-memory implementation. Backed by a `Mutex<HashMap<…>>`;
/// state is lost on process restart. Useful for `TestApp`, smoke tests,
/// and CLI binaries that don't carry a database.
#[derive(Debug, Default)]
pub struct InMemoryEnablementStore {
    inner: Mutex<HashMap<String, EnablementState>>,
}

impl InMemoryEnablementStore {
    /// Empty store.
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl EnablementStore for InMemoryEnablementStore {
    async fn get(&self, id: &ExtensionId) -> Result<Option<EnablementState>, StoreError> {
        Ok(self
            .inner
            .lock()
            .expect("InMemoryEnablementStore poisoned")
            .get(id.as_str())
            .copied())
    }

    async fn set(&self, id: &ExtensionId, state: EnablementState) -> Result<(), StoreError> {
        self.inner
            .lock()
            .expect("InMemoryEnablementStore poisoned")
            .insert(id.as_str().to_string(), state);
        Ok(())
    }
}
