//! Integration scenarios for the v0 cache layer.
//!
//! Each test maps to one of the five scenarios from the
//! [opt-in caching proposal][1] §"Test story". v0 reinterprets the
//! scenarios that name v1+ features (SWR, bucket tags, `empty_ttl`)
//! into the closest v0-honest analog so the surface still gets test
//! coverage at this slice.
//!
//! [1]: ../../rubix/docs/proposal/fe-cache-opt-in.md

use starter_cache::{
    CacheLayer, CacheScope, CacheSpec, CallerScope, InMemoryInvalidator, Invalidator, LayerConfig,
    SystemClock,
};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

fn b(v: &str) -> starter_cache::Bytes {
    Arc::new(v.as_bytes().to_vec())
}

/// Scenario 1 — tag fired during in-flight load → store is dropped.
///
/// The invalidation-token race fix. Without it, the loader's result
/// would silently overwrite the in-flight invalidation and live
/// until TTL.
#[tokio::test]
async fn s1_tag_fired_during_in_flight_load_drops_store() {
    let invalidator = Arc::new(InMemoryInvalidator::new());
    let layer = CacheLayer::with_parts(
        LayerConfig::default(),
        Arc::new(SystemClock),
        invalidator.clone(),
    );
    let spec = CacheSpec::ttl(Duration::from_secs(60))
        .scope(CacheScope::Tenant)
        .invalidate_on_table("readings");
    let caller = CallerScope::new("tA", "uX");
    let calls = Arc::new(AtomicU32::new(0));

    // Fire invalidation mid-load. The loader sleeps long enough that
    // we can land the invalidate before the store happens.
    let inv = invalidator.clone();
    let calls_clone = calls.clone();
    let _ = layer
        .get_or_load(&spec, &caller, "k1", || async move {
            calls_clone.fetch_add(1, Ordering::SeqCst);
            // Mid-load invalidation: bumps the readings token.
            inv.invalidate_tags(&["table:readings".to_string()]).await;
            tokio::time::sleep(Duration::from_millis(5)).await;
            Ok::<_, std::convert::Infallible>(b("v1"))
        })
        .await
        .unwrap();

    // The store must have been dropped — the next call has to run
    // the loader again instead of serving the cached "v1".
    let calls_clone = calls.clone();
    let _ = layer
        .get_or_load(&spec, &caller, "k1", || async move {
            calls_clone.fetch_add(1, Ordering::SeqCst);
            Ok::<_, std::convert::Infallible>(b("v2"))
        })
        .await
        .unwrap();

    assert_eq!(
        calls.load(Ordering::SeqCst),
        2,
        "mid-load invalidation must drop the store; second call must re-load"
    );
}

/// Scenario 2 — "SWR refresh in progress when invalidation fires →
/// in-flight store dropped". v0 has no SWR; the v0-honest analog is
/// "two concurrent loaders for the same key, one of them sees a
/// mid-load invalidation, and the post-invalidation read pays a
/// fresh miss". We exercise the post-invalidation read explicitly.
#[tokio::test]
async fn s2_post_invalidate_read_pays_fresh_miss() {
    let invalidator = Arc::new(InMemoryInvalidator::new());
    let layer = CacheLayer::with_parts(
        LayerConfig::default(),
        Arc::new(SystemClock),
        invalidator.clone(),
    );
    let spec = CacheSpec::ttl(Duration::from_secs(60))
        .scope(CacheScope::Tenant)
        .invalidate_on_table("readings");
    let caller = CallerScope::new("tA", "uX");
    let calls = Arc::new(AtomicU32::new(0));

    // Populate.
    let calls_clone = calls.clone();
    let _ = layer
        .get_or_load(&spec, &caller, "k1", || async move {
            calls_clone.fetch_add(1, Ordering::SeqCst);
            Ok::<_, std::convert::Infallible>(b("v1"))
        })
        .await
        .unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);

    // Tag fires.
    invalidator
        .invalidate_tags(&["table:readings".to_string()])
        .await;

    // Next read must be a fresh miss (token-on-read check fails on
    // the stored snapshot).
    let calls_clone = calls.clone();
    let _ = layer
        .get_or_load(&spec, &caller, "k1", || async move {
            calls_clone.fetch_add(1, Ordering::SeqCst);
            Ok::<_, std::convert::Infallible>(b("v2"))
        })
        .await
        .unwrap();
    assert_eq!(
        calls.load(Ordering::SeqCst),
        2,
        "post-invalidation read must pay a fresh miss"
    );
}

/// Scenario 3 — "bucket-level invalidation hits the right bucket and
/// only the right bucket." v0 has no bucket tags; the v0-honest
/// analog is "table-tag invalidation for one table does not affect
/// entries keyed to a different table". Verifies the registry maps
/// tags surgically, not as a wildcard.
#[tokio::test]
async fn s3_other_tables_unaffected_by_invalidate() {
    let invalidator = Arc::new(InMemoryInvalidator::new());
    let layer = CacheLayer::with_parts(
        LayerConfig::default(),
        Arc::new(SystemClock),
        invalidator.clone(),
    );
    let caller = CallerScope::new("tA", "uX");
    let spec_a = CacheSpec::ttl(Duration::from_secs(60)).invalidate_on_table("alpha");
    let spec_b = CacheSpec::ttl(Duration::from_secs(60)).invalidate_on_table("beta");
    let calls = Arc::new(AtomicU32::new(0));

    // Populate one entry per spec.
    for (spec, key) in [(&spec_a, "a"), (&spec_b, "b")] {
        let calls_clone = calls.clone();
        let label = key.to_string();
        let _ = layer
            .get_or_load(spec, &caller, key, || async move {
                calls_clone.fetch_add(1, Ordering::SeqCst);
                Ok::<_, std::convert::Infallible>(b(&label))
            })
            .await
            .unwrap();
    }
    assert_eq!(calls.load(Ordering::SeqCst), 2);

    // Invalidate alpha. The beta entry must still be a hit.
    invalidator
        .invalidate_tags(&["table:alpha".to_string()])
        .await;

    let calls_clone = calls.clone();
    let _ = layer
        .get_or_load(&spec_b, &caller, "b", || async move {
            calls_clone.fetch_add(1, Ordering::SeqCst);
            Ok::<_, std::convert::Infallible>(b("b2"))
        })
        .await
        .unwrap();
    assert_eq!(
        calls.load(Ordering::SeqCst),
        2,
        "beta entry must still be a hit after alpha invalidate"
    );

    // ... and the alpha entry must be a miss.
    let calls_clone = calls.clone();
    let _ = layer
        .get_or_load(&spec_a, &caller, "a", || async move {
            calls_clone.fetch_add(1, Ordering::SeqCst);
            Ok::<_, std::convert::Infallible>(b("a2"))
        })
        .await
        .unwrap();
    assert_eq!(
        calls.load(Ordering::SeqCst),
        3,
        "alpha entry must be a miss after alpha invalidate"
    );
}

/// Scenario 4 — "empty result respects `empty_ttl`, not `ttl`". v0
/// has no `empty_ttl`. The v0-honest analog is "loader errors are
/// never cached" — the closest correctness invariant we have at this
/// slice. A loader error returning `Err` must not poison the entry.
#[tokio::test]
async fn s4_loader_error_is_not_cached() {
    let layer = CacheLayer::new(LayerConfig::default());
    let spec = CacheSpec::ttl(Duration::from_secs(60)).invalidate_on_table("readings");
    let caller = CallerScope::new("tA", "uX");
    let calls = Arc::new(AtomicU32::new(0));

    // First call errors.
    let calls_clone = calls.clone();
    let err = layer
        .get_or_load::<_, _, String>(&spec, &caller, "k1", || async move {
            calls_clone.fetch_add(1, Ordering::SeqCst);
            Err::<starter_cache::Bytes, String>("boom".into())
        })
        .await
        .unwrap_err();
    assert_eq!(err, "boom");

    // Second call must run the loader again — the error was not
    // cached. Returns Ok this time.
    let calls_clone = calls.clone();
    let v = layer
        .get_or_load::<_, _, String>(&spec, &caller, "k1", || async move {
            calls_clone.fetch_add(1, Ordering::SeqCst);
            Ok::<_, String>(b("ok"))
        })
        .await
        .unwrap();
    assert_eq!(&*v, b"ok");
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

/// Scenario 5 — per-tenant weight cap evicts the noisy tenant's
/// entries, not its neighbours'.
///
/// v0 implements per-tenant caps by partitioning into one moka cache
/// per tenant. We verify this directly: tenant A overflows its cap;
/// tenant B's entries stay intact.
#[tokio::test]
async fn s5_per_tenant_cap_isolates_noisy_tenant() {
    let layer = CacheLayer::with_parts(
        LayerConfig {
            per_tenant_max_entries: 4,
        },
        Arc::new(SystemClock),
        Arc::new(InMemoryInvalidator::new()),
    );
    let spec = CacheSpec::ttl(Duration::from_secs(60)).scope(CacheScope::Tenant);

    // Tenant B: insert 2 entries (well under cap).
    let caller_b = CallerScope::new("tB", "uB");
    for i in 0..2 {
        let key = format!("b{i}");
        let _ = layer
            .get_or_load::<_, _, std::convert::Infallible>(&spec, &caller_b, &key, || async move {
                Ok(b("vB"))
            })
            .await
            .unwrap();
    }

    // Tenant A: insert 50 entries — well over cap (4). moka will
    // evict among A's own entries.
    let caller_a = CallerScope::new("tA", "uA");
    for i in 0..50 {
        let key = format!("a{i}");
        let _ = layer
            .get_or_load::<_, _, std::convert::Infallible>(&spec, &caller_a, &key, || async move {
                Ok(b("vA"))
            })
            .await
            .unwrap();
    }

    // Force moka's pending tasks to drain — entry_count is otherwise
    // eventually consistent.
    layer.run_pending_tasks().await;

    // Tenant B's entry count must be unchanged (2). Tenant A's
    // count must be at or under the cap.
    let b_count = layer.tenant_entry_count("tB");
    let a_count = layer.tenant_entry_count("tA");
    assert_eq!(
        b_count, 2,
        "noisy tenant A must not evict tenant B's entries (B had {b_count})"
    );
    assert!(
        a_count <= 4,
        "tenant A must respect its own cap (saw {a_count})"
    );
}
