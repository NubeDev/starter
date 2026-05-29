//! Tag-based invalidation primitives.
//!
//! Two responsibilities:
//!
//! 1. Maintain a per-tag monotonic *invalidation token* (an
//!    `AtomicU64` bumped on every invalidate). The cache layer
//!    snapshots tokens at load-start and re-checks them before
//!    storing a value (race-fix) and again on read (treat
//!    token-moved entries as misses).
//! 2. Provide test introspection — every tag fired so far.
//!
//! v0 deliberately does **not** keep a per-tag "list of keys to
//! drop on fire" registry. Token-on-read covers correctness; the
//! drop-list optimisation can be added later if metrics show
//! `tokens_match` overhead is meaningful. Keeping this small also
//! lets the cache layer stay `forbid(unsafe_code)`.

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

/// Snapshot of the invalidation tokens for the tags a load depends on.
#[derive(Debug, Default, Clone)]
pub struct TokenSnapshot {
    /// Tag → observed token value at snapshot time. An empty snapshot
    /// (no tags) never trips the race check.
    pub tokens: HashMap<String, u64>,
}

impl TokenSnapshot {
    /// Build an empty snapshot.
    pub fn empty() -> Self {
        Self::default()
    }
}

/// Tag-based invalidation port.
///
/// The cache layer talks to the invalidator across this trait so we
/// can swap an in-memory impl for an event-bus impl later without
/// touching call sites.
#[async_trait]
pub trait Invalidator: Send + Sync + 'static {
    /// Bump every named tag's token. Cache entries whose stored
    /// snapshot disagrees with the new token will be served as
    /// misses next time they're read.
    async fn invalidate_tags(&self, tags: &[String]);

    /// Snapshot the current token for each named tag. The cache
    /// layer takes the snapshot **before** firing the loader, then
    /// stores it alongside the loaded value.
    fn snapshot_tokens(&self, tags: &[String]) -> TokenSnapshot;

    /// Returns `true` iff every tag in `snap` still holds its
    /// snapshotted token value.
    fn tokens_match(&self, snap: &TokenSnapshot) -> bool;
}

// ---------------------------------------------------------------------------
// InMemoryInvalidator — the single-process default.
// ---------------------------------------------------------------------------

/// Process-local invalidator. Drives v0; v2 swaps an event-bus impl
/// behind the same trait without touching call sites.
pub struct InMemoryInvalidator {
    inner: Mutex<Inner>,
}

struct Inner {
    tokens: HashMap<String, AtomicU64>,
    /// Test/observability: tags fired so far, in order.
    fired: Vec<String>,
}

impl InMemoryInvalidator {
    /// New invalidator with no fired tags and zero-valued tokens.
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(Inner {
                tokens: HashMap::new(),
                fired: Vec::new(),
            }),
        }
    }

    fn token_value(inner: &mut Inner, tag: &str) -> u64 {
        inner
            .tokens
            .entry(tag.to_string())
            .or_insert_with(|| AtomicU64::new(0))
            .load(Ordering::SeqCst)
    }

    fn bump_token(inner: &mut Inner, tag: &str) {
        inner
            .tokens
            .entry(tag.to_string())
            .or_insert_with(|| AtomicU64::new(0))
            .fetch_add(1, Ordering::SeqCst);
    }

    /// Test introspection — every tag that has fired, in order, since
    /// construction.
    pub fn fired_tags(&self) -> Vec<String> {
        self.inner.lock().unwrap().fired.clone()
    }
}

impl Default for InMemoryInvalidator {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Invalidator for InMemoryInvalidator {
    async fn invalidate_tags(&self, tags: &[String]) {
        let mut g = self.inner.lock().unwrap();
        for tag in tags {
            Self::bump_token(&mut g, tag);
            g.fired.push(tag.clone());
        }
    }

    fn snapshot_tokens(&self, tags: &[String]) -> TokenSnapshot {
        let mut g = self.inner.lock().unwrap();
        let mut out = HashMap::with_capacity(tags.len());
        for tag in tags {
            out.insert(tag.clone(), Self::token_value(&mut g, tag));
        }
        TokenSnapshot { tokens: out }
    }

    fn tokens_match(&self, snap: &TokenSnapshot) -> bool {
        if snap.tokens.is_empty() {
            return true;
        }
        let mut g = self.inner.lock().unwrap();
        for (tag, observed) in &snap.tokens {
            if Self::token_value(&mut g, tag) != *observed {
                return false;
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn token_bumps_on_invalidate() {
        let inv = InMemoryInvalidator::new();
        let tags = vec!["table:readings".to_string()];
        let snap = inv.snapshot_tokens(&tags);
        assert!(inv.tokens_match(&snap));
        inv.invalidate_tags(&tags).await;
        assert!(!inv.tokens_match(&snap));
    }

    #[tokio::test]
    async fn empty_snapshot_always_matches() {
        let inv = InMemoryInvalidator::new();
        inv.invalidate_tags(&["x".into()]).await;
        assert!(inv.tokens_match(&TokenSnapshot::empty()));
    }

    #[tokio::test]
    async fn fired_tags_observed_in_order() {
        let inv = InMemoryInvalidator::new();
        inv.invalidate_tags(&["table:a".to_string()]).await;
        inv.invalidate_tags(&["table:b".to_string()]).await;
        assert_eq!(inv.fired_tags(), vec!["table:a", "table:b"]);
    }
}
