//! In-memory [`OAuthStateStore`] — the default. Lives in process
//! RAM behind a `Mutex<HashMap>`; entries TTL out after
//! [`STATE_TTL`] and are also evicted opportunistically on every
//! `take`.
//!
//! Single-node deploys (the v0.1 default) want this. A multi-node
//! deploy points `OAUTH_STATE_STORE=sqlite|postgres` at one of the
//! durable impls landing in Phase 4 — the trait stays the same.

use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use chrono::Utc;

use super::{OAuthFlowState, OAuthStateError, OAuthStateStore, STATE_TTL};

/// In-process state store. Cheap, has no external dependencies,
/// works for single-instance deploys.
///
/// The Mutex is intentional: `tokio::sync::Mutex` would force every
/// caller into an async-await of a non-async critical section, and
/// the section is two HashMap operations on a map that never
/// grows past a few thousand entries (one per in-flight OAuth
/// redirect). `std::sync::Mutex` is the right tool.
#[derive(Debug, Default)]
pub struct MemoryStateStore {
    inner: Mutex<HashMap<String, OAuthFlowState>>,
}

impl MemoryStateStore {
    /// Build an empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// How many live entries the store is holding. Diagnostic only.
    pub fn len(&self) -> usize {
        self.inner
            .lock()
            .map(|m| m.len())
            .unwrap_or(0)
    }

    /// `true` when the store holds no live entries.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[async_trait]
impl OAuthStateStore for MemoryStateStore {
    async fn put(&self, flow: OAuthFlowState) -> Result<(), OAuthStateError> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|e| OAuthStateError::Backend(format!("mutex poisoned: {e}")))?;
        guard.insert(flow.state.clone(), flow);
        Ok(())
    }

    async fn take(&self, state: &str) -> Result<Option<OAuthFlowState>, OAuthStateError> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|e| OAuthStateError::Backend(format!("mutex poisoned: {e}")))?;

        // Opportunistic eviction: every `take` is also a sweep of
        // expired entries so the map size tracks live traffic
        // without a background task. The TTL window is 10 minutes
        // so the sweep is at worst proportional to in-flight
        // OAuth redirects over that window — a few thousand at
        // realistic load.
        let now = Utc::now();
        let ttl = chrono::Duration::from_std(STATE_TTL)
            .map_err(|e| OAuthStateError::Backend(format!("ttl conversion: {e}")))?;
        guard.retain(|_, flow| now.signed_duration_since(flow.created_at) < ttl);

        Ok(guard.remove(state))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flow(state: &str) -> OAuthFlowState {
        OAuthFlowState {
            provider: "github".to_string(),
            state: state.to_string(),
            pkce_verifier: "verifier".to_string(),
            return_to: None,
            link_mode_user_id: None,
            created_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn put_then_take_roundtrip() {
        let store = MemoryStateStore::new();
        store.put(flow("abc")).await.unwrap();
        let got = store.take("abc").await.unwrap().expect("entry present");
        assert_eq!(got.state, "abc");
        // Consumed on read.
        assert!(store.take("abc").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn expired_entry_is_swept_on_read() {
        let store = MemoryStateStore::new();
        let mut stale = flow("stale");
        stale.created_at = Utc::now() - chrono::Duration::minutes(20);
        store.put(stale).await.unwrap();
        // The sweep inside `take` evicts the expired entry even
        // when we ask for a *different* key.
        assert!(store.take("nope").await.unwrap().is_none());
        assert!(store.is_empty(), "stale entry should have been evicted");
    }
}
