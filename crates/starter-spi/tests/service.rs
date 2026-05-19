//! Integration tests for the new `service` module. Covers the bits a
//! provider crate is going to rely on:
//!
//! - shutdown actually flips the watch and clean-exiting services
//!   resolve as `ServiceShutdownOutcome::Clean`;
//! - a service that ignores `ctx.shutdown` gets force-aborted and
//!   shows up as `Aborted` (SCOPE smoke test 5 baseline);
//! - the `FanOut` helper logs-and-continues on `Closed`/`Other` but
//!   bubbles `Saturated`;
//! - the broadcast blanket impl maps no-receivers to `Closed`.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde_json::{json, Value};
use tokio::sync::broadcast;

use starter_spi::error::Result;
use starter_spi::service::{
    Event, EventSink, FanOut, Service, ServiceContext, ServiceHandle, ServiceRegistry,
    ServiceShutdownOutcome, SinkError, SinkResult,
};

/// A service whose loop watches `ctx.shutdown` and exits cleanly.
struct CleanService;

#[async_trait]
impl Service for CleanService {
    fn name(&self) -> &'static str {
        "clean"
    }
    async fn start(&self, ctx: ServiceContext) -> Result<ServiceHandle> {
        let mut shutdown = ctx.shutdown;
        let join = tokio::spawn(async move {
            // Wait for the watch to flip.
            while !*shutdown.borrow() {
                if shutdown.changed().await.is_err() {
                    break;
                }
            }
            Ok::<(), starter_spi::Error>(())
        });
        Ok(ServiceHandle::new(join))
    }
}

/// A service that never observes the shutdown watch.
struct WedgedService;

#[async_trait]
impl Service for WedgedService {
    fn name(&self) -> &'static str {
        "wedged"
    }
    async fn start(&self, _ctx: ServiceContext) -> Result<ServiceHandle> {
        let join = tokio::spawn(async {
            loop {
                tokio::time::sleep(Duration::from_secs(60)).await;
            }
        });
        Ok(ServiceHandle::new(join))
    }
}

struct NullSink;

#[async_trait]
impl EventSink for NullSink {
    async fn emit(&self, _kind: &str, _payload: Value) -> SinkResult<()> {
        Ok(())
    }
}

#[tokio::test]
async fn shutdown_clean_service_reports_clean() {
    let mut reg = ServiceRegistry::new().register(CleanService);
    let metrics = Arc::new(prometheus::Registry::new());
    let sink: Arc<dyn EventSink> = Arc::new(NullSink);
    reg.start_all(metrics, sink).await.unwrap();
    let report = reg.shutdown().await;
    assert_eq!(report.services.len(), 1);
    let (name, outcome) = &report.services[0];
    assert_eq!(name, "clean");
    assert!(matches!(outcome, ServiceShutdownOutcome::Clean));
}

#[tokio::test]
async fn shutdown_wedged_service_is_aborted() {
    let mut reg = ServiceRegistry::new().register(WedgedService);
    let metrics = Arc::new(prometheus::Registry::new());
    let sink: Arc<dyn EventSink> = Arc::new(NullSink);
    reg.start_all(metrics, sink).await.unwrap();
    let report = reg
        .shutdown_with_deadline(Duration::from_millis(50))
        .await;
    let (_, outcome) = &report.services[0];
    assert!(matches!(outcome, ServiceShutdownOutcome::Aborted));
}

struct CountingSink {
    hits: Arc<AtomicUsize>,
}

#[async_trait]
impl EventSink for CountingSink {
    async fn emit(&self, _kind: &str, _payload: Value) -> SinkResult<()> {
        self.hits.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

struct ClosedSink;

#[async_trait]
impl EventSink for ClosedSink {
    async fn emit(&self, kind: &str, _payload: Value) -> SinkResult<()> {
        Err(SinkError::Closed {
            kind: kind.to_string(),
        })
    }
}

struct SaturatedSink;

#[async_trait]
impl EventSink for SaturatedSink {
    async fn emit(&self, kind: &str, _payload: Value) -> SinkResult<()> {
        Err(SinkError::Saturated {
            kind: kind.to_string(),
        })
    }
}

#[tokio::test]
async fn fanout_logs_closed_and_continues() {
    let hits = Arc::new(AtomicUsize::new(0));
    let fan = FanOut::new()
        .with(Arc::new(ClosedSink))
        .with(Arc::new(CountingSink { hits: hits.clone() }));
    fan.emit("x.kind", json!({})).await.unwrap();
    assert_eq!(hits.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn fanout_bubbles_saturated_after_fanout_completes() {
    let hits = Arc::new(AtomicUsize::new(0));
    let fan = FanOut::new()
        .with(Arc::new(SaturatedSink))
        .with(Arc::new(CountingSink { hits: hits.clone() }));
    let err = fan.emit("x.kind", json!({})).await.unwrap_err();
    assert!(matches!(err, SinkError::Saturated { .. }));
    // The other sink was still called — fan-out completes before
    // bubbling.
    assert_eq!(hits.load(Ordering::SeqCst), 1);
}

#[cfg(feature = "broadcast")]
#[tokio::test]
async fn broadcast_blanket_no_receivers_is_closed() {
    let (tx, rx) = broadcast::channel::<Event>(4);
    drop(rx);
    let err = tx.emit("x.kind", json!({})).await.unwrap_err();
    assert!(matches!(err, SinkError::Closed { .. }));
}

#[cfg(feature = "broadcast")]
#[tokio::test]
async fn broadcast_blanket_delivers_to_receiver() {
    let (tx, mut rx) = broadcast::channel::<Event>(4);
    tx.emit("x.kind", json!({"a": 1})).await.unwrap();
    let ev = rx.recv().await.unwrap();
    assert_eq!(ev.kind, "x.kind");
    assert_eq!(ev.payload, json!({"a": 1}));
}
