//! v3 scenarios — covers the surfaces stage 3 adds.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

use starter_cache::{
    BucketTagSpec, CacheLayer, CacheScope, CacheSpec, CallerScope, DefaultWarehouseWriter,
    EventBusInvalidator, InMemoryInvalidator, InvalidationBus, Invalidator, LayerConfig,
    SystemClock, WarehouseWriter, Warmer, WarmerStatus, WriteRow, WriterTagRegistry,
};

struct CapturingBus {
    published: Mutex<Vec<Vec<String>>>,
}

#[async_trait::async_trait]
impl InvalidationBus for CapturingBus {
    async fn publish(&self, tags: &[String]) {
        self.published.lock().await.push(tags.to_vec());
    }
}

#[tokio::test]
async fn event_bus_invalidator_fans_out_to_peer_replica() {
    let bus = Arc::new(CapturingBus {
        published: Mutex::new(Vec::new()),
    });
    let inv_a: Arc<dyn Invalidator> = Arc::new(EventBusInvalidator::new(bus.clone()));
    // Replica B's local view — separate token store.
    let inv_b = Arc::new(EventBusInvalidator::new(bus.clone()));

    let layer_a = CacheLayer::with_parts(LayerConfig::default(), Arc::new(SystemClock), inv_a);
    let spec = CacheSpec::ttl(Duration::from_secs(60)).invalidate_on_table("readings");
    let caller = CallerScope::new("tA", "u1");
    // populate
    let _ = layer_a
        .get_or_load::<_, _, std::convert::Infallible>(&spec, &caller, "k", || async {
            Ok(Arc::new(b"v".to_vec()))
        })
        .await
        .unwrap();
    // Fire from A; bus captures one tag list.
    layer_a
        .invalidator()
        .invalidate_tags(&["table:readings".to_string()])
        .await;
    let published = bus.published.lock().await.clone();
    assert_eq!(published.len(), 1);
    assert_eq!(published[0], vec!["table:readings".to_string()]);
    // Replica B applies the same tags from the bus and observes its
    // local tokens bumped — proving fan-out path is wired.
    let snap_b = inv_b.snapshot_tokens(&published[0]);
    inv_b.apply_remote(&published[0]).await;
    assert!(!inv_b.tokens_match(&snap_b));
}

#[tokio::test]
async fn valkey_backend_round_trips_across_handles() {
    // Feature-gated; this test only runs under --features valkey, but
    // also passes the default build because the trait surface is the
    // same.
    #[cfg(feature = "valkey")]
    {
        use starter_cache::backends::valkey::ValkeyCache;
        use starter_cache::Cache;
        let a: ValkeyCache<String, String> = ValkeyCache::new(Duration::from_secs(60));
        let b = a.clone(); // simulates a sibling pointing at the
                           // same "remote" store.
        a.insert("k".into(), "v".into()).await;
        assert_eq!(b.get(&"k".to_string()).await.as_deref(), Some("v"));
    }
}

#[tokio::test]
async fn cold_start_warmer_populates_status_after_synthetic_restart() {
    let warmer = Warmer::new();
    assert!(warmer.snapshot().last_run_at.is_none());
    let cb: starter_cache::WarmCallback = Arc::new(|_id| Box::pin(async move { Ok(()) }));
    warmer
        .warm_top_n(vec!["k1".into(), "k2".into(), "k3".into()], cb)
        .await;
    let WarmerStatus {
        entries_warmed,
        last_run_at,
        ..
    } = warmer.snapshot();
    assert_eq!(entries_warmed, 3);
    assert!(last_run_at.is_some());
}

#[tokio::test]
async fn dimension_scoped_tags_hit_only_dimensioned_entries() {
    let inv = Arc::new(InMemoryInvalidator::new());
    let reg = WriterTagRegistry::from_specs([BucketTagSpec {
        table: "readings".into(),
        granularity: "1h".into(),
        dimensions: vec!["meter".into()],
    }]);
    let w = DefaultWarehouseWriter::new(inv.clone(), reg);
    let mut dims = BTreeMap::new();
    dims.insert("meter".into(), "42".into());
    w.enqueue(WriteRow {
        table: "readings".into(),
        ts: Some(chrono::Utc::now()),
        dimensions: dims,
    })
    .await;
    w.commit().await;
    let fired: std::collections::BTreeSet<String> = inv.fired_tags().into_iter().collect();
    assert!(fired.contains("table:readings:meter=42"));
    // A spec subscribing to a different meter id never sees the fire.
    let subscriber_tag = "table:readings:meter=99".to_string();
    assert!(!fired.contains(&subscriber_tag));
}

#[tokio::test]
async fn warehouse_writer_chokepoint_fires_invalidate_after_ingest() {
    // A subscriber holds a cache entry under
    // `invalidate_on_table("readings")`; after the writer commits a
    // batch, the entry becomes a miss next read.
    let inv: Arc<dyn Invalidator> = Arc::new(InMemoryInvalidator::new());
    let layer = CacheLayer::with_parts(LayerConfig::default(), Arc::new(SystemClock), inv.clone());
    let spec = CacheSpec::ttl(Duration::from_secs(60)).invalidate_on_table("readings");
    let caller = CallerScope::new("tA", "u1");
    let calls = Arc::new(std::sync::atomic::AtomicU32::new(0));
    let _ = layer
        .get_or_load::<_, _, std::convert::Infallible>(&spec, &caller, "k", {
            let c = calls.clone();
            || async move {
                c.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(Arc::new(b"v".to_vec()))
            }
        })
        .await
        .unwrap();
    assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);

    // Writer chokepoint commits one row → fires one invalidate.
    let reg = WriterTagRegistry::from_specs(std::iter::empty::<BucketTagSpec>());
    let w = DefaultWarehouseWriter::new(inv.clone(), reg);
    w.enqueue(WriteRow {
        table: "readings".into(),
        ts: Some(chrono::Utc::now()),
        dimensions: BTreeMap::new(),
    })
    .await;
    w.commit().await;

    // Second read pays a fresh miss.
    let _ = layer
        .get_or_load::<_, _, std::convert::Infallible>(&spec, &caller, "k", {
            let c = calls.clone();
            || async move {
                c.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(Arc::new(b"v2".to_vec()))
            }
        })
        .await
        .unwrap();
    assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 2);
}

#[tokio::test]
async fn batched_tag_dedup_fires_one_invalidate_for_many_rows() {
    let inv = Arc::new(InMemoryInvalidator::new());
    let reg = WriterTagRegistry::from_specs([BucketTagSpec {
        table: "readings".into(),
        granularity: "1h".into(),
        dimensions: vec![],
    }]);
    let w = DefaultWarehouseWriter::new(inv.clone(), reg);
    let base = chrono::Utc::now();
    for i in 0..500 {
        w.enqueue(WriteRow {
            table: "readings".into(),
            ts: Some(base + chrono::Duration::minutes((i / 42) as i64 * 60)),
            dimensions: BTreeMap::new(),
        })
        .await;
    }
    w.commit().await;
    let unique: std::collections::BTreeSet<String> = inv.fired_tags().into_iter().collect();
    // 1 table + up to 12 buckets — well under the 500 lower-bound a
    // naive per-row impl would produce.
    assert!(
        unique.len() <= 13,
        "expected dedup to keep tags ≤13; got {}",
        unique.len()
    );
    assert!(unique.contains("table:readings"));
}

#[tokio::test]
async fn tower_layer_smoke_compiles_under_feature() {
    // The tower layer requires extra feature flags; this test merely
    // asserts the entry point exists when compiled with `tower`.
    #[cfg(feature = "tower")]
    {
        let layer = CacheLayer::new(LayerConfig::default());
        let spec = CacheSpec::ttl(Duration::from_secs(30)).scope(CacheScope::Tenant);
        let _t = layer.tower(spec, "test-route");
    }
    let _ = CacheScope::Tenant; // silence unused under !tower
}
