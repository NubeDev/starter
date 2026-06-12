//! Core pipeline behaviour tests, built on in-test node doubles so the engine
//! core is proven without any real source/sink (those land in RW-02).
//!
//! Covered: a finite source runs to completion with the sink seeing every batch
//! in order and closed exactly once; mid-stream cancellation drains the in-flight
//! batch and closes the sink exactly once; the bounded channel blocks a fast
//! source against a gated slow sink (backpressure); the registry rejects an
//! unknown node type with a useful message.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use datafusion::arrow::array::{Int32Array, RecordBatch};
use datafusion::arrow::datatypes::{DataType, Field, Schema};
use nexus_engine::core::{
    EngineError, EngineResult, Pipeline, PipelineConfig, Processor, Registry, RunOutcome, Sink,
    Source,
};
use serde_json::json;
use tokio::sync::Notify;
use tokio::time::{timeout, Duration};
use tokio_util::sync::CancellationToken;

/// A one-column `n: Int32` batch carrying `value`, so tests can assert ordering
/// by reading the column back.
fn batch(value: i32) -> RecordBatch {
    let schema = Arc::new(Schema::new(vec![Field::new("n", DataType::Int32, false)]));
    RecordBatch::try_new(schema, vec![Arc::new(Int32Array::from(vec![value]))]).unwrap()
}

fn first_n(batch: &RecordBatch) -> i32 {
    batch
        .column(0)
        .as_any()
        .downcast_ref::<Int32Array>()
        .unwrap()
        .value(0)
}

/// Emits `0..count` as one batch each, then `None`. Optionally pings a `Notify`
/// after each successful send so a test can observe how far a fast source got
/// past a bounded channel.
struct CountSource {
    next: i32,
    count: i32,
    sent: Arc<AtomicUsize>,
    on_send: Option<Arc<Notify>>,
}

#[async_trait]
impl Source for CountSource {
    async fn read(&mut self) -> EngineResult<Option<RecordBatch>> {
        if self.next >= self.count {
            return Ok(None);
        }
        let b = batch(self.next);
        self.next += 1;
        self.sent.fetch_add(1, Ordering::SeqCst);
        if let Some(n) = &self.on_send {
            n.notify_one();
        }
        Ok(Some(b))
    }
}

/// Records every batch it is asked to write and how many times `close` ran, over
/// shared handles a test can inspect after the run.
struct RecordingSink {
    seen: Arc<std::sync::Mutex<Vec<i32>>>,
    closes: Arc<AtomicUsize>,
    gate: Option<Arc<Notify>>,
}

#[async_trait]
impl Sink for RecordingSink {
    async fn write(&mut self, batch: &RecordBatch) -> EngineResult<()> {
        if let Some(gate) = &self.gate {
            // Hold the consumer here so the bounded channel fills behind us.
            gate.notified().await;
        }
        self.seen.lock().unwrap().push(first_n(batch));
        Ok(())
    }

    async fn close(&mut self) -> EngineResult<()> {
        self.closes.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

/// Passes batches through unchanged; stands in for a real processor chain.
struct Identity;

#[async_trait]
impl Processor for Identity {
    async fn process(&mut self, batch: RecordBatch) -> EngineResult<Vec<RecordBatch>> {
        Ok(vec![batch])
    }
}

#[tokio::test]
async fn finite_source_completes_with_ordered_batches_and_one_close() {
    let sent = Arc::new(AtomicUsize::new(0));
    let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
    let closes = Arc::new(AtomicUsize::new(0));

    let mut registry = Registry::new();
    let s = sent.clone();
    registry.register_source(
        "count",
        Box::new(move |_| {
            Ok(Box::new(CountSource {
                next: 0,
                count: 5,
                sent: s.clone(),
                on_send: None,
            }) as Box<dyn Source>)
        }),
    );
    registry.register_processor(
        "identity",
        Box::new(|_| Ok(Box::new(Identity) as Box<dyn Processor>)),
    );
    let seen_b = seen.clone();
    let closes_b = closes.clone();
    registry.register_sink(
        "record",
        Box::new(move |_| {
            Ok(Box::new(RecordingSink {
                seen: seen_b.clone(),
                closes: closes_b.clone(),
                gate: None,
            }) as Box<dyn Sink>)
        }),
    );

    let config = PipelineConfig::from_value(json!({
        "input": { "type": "count" },
        "pipeline": { "processors": [{ "type": "identity" }] },
        "output": { "type": "record" },
    }))
    .unwrap();

    let pipeline = Pipeline::build(&registry, &config).unwrap();
    let outcome = pipeline.run(CancellationToken::new()).await.unwrap();

    assert_eq!(outcome, RunOutcome::Completed);
    assert_eq!(
        *seen.lock().unwrap(),
        vec![0, 1, 2, 3, 4],
        "all batches in order"
    );
    assert_eq!(closes.load(Ordering::SeqCst), 1, "sink closed exactly once");
}

#[tokio::test]
async fn cancellation_drains_in_flight_and_closes_sink_once() {
    let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
    let closes = Arc::new(AtomicUsize::new(0));
    let token = CancellationToken::new();

    let mut registry = Registry::new();
    // An unbounded source: never returns None, so only cancellation can stop it.
    let sent = Arc::new(AtomicUsize::new(0));
    let s = sent.clone();
    registry.register_source(
        "infinite",
        Box::new(move |_| {
            Ok(Box::new(CountSource {
                next: 0,
                count: i32::MAX,
                sent: s.clone(),
                on_send: None,
            }) as Box<dyn Source>)
        }),
    );
    let seen_b = seen.clone();
    let closes_b = closes.clone();
    registry.register_sink(
        "record",
        Box::new(move |_| {
            Ok(Box::new(RecordingSink {
                seen: seen_b.clone(),
                closes: closes_b.clone(),
                gate: None,
            }) as Box<dyn Sink>)
        }),
    );

    let config = PipelineConfig::from_value(json!({
        "input": { "type": "infinite" },
        "pipeline": { "processors": [] },
        "output": { "type": "record" },
    }))
    .unwrap();

    let pipeline = Pipeline::build(&registry, &config).unwrap();
    let cancel_token = token.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(20)).await;
        cancel_token.cancel();
    });

    let outcome = timeout(Duration::from_secs(5), pipeline.run(token))
        .await
        .expect("cancelled run terminates promptly")
        .unwrap();

    assert_eq!(outcome, RunOutcome::Cancelled);
    assert_eq!(
        closes.load(Ordering::SeqCst),
        1,
        "sink closed exactly once on cancel"
    );
}

#[tokio::test]
async fn bounded_channel_blocks_fast_source_against_slow_sink() {
    let sent = Arc::new(AtomicUsize::new(0));
    let on_send = Arc::new(Notify::new());
    let gate = Arc::new(Notify::new());
    let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
    let closes = Arc::new(AtomicUsize::new(0));

    let mut registry = Registry::new();
    let s = sent.clone();
    let pinged = on_send.clone();
    registry.register_source(
        "fast",
        Box::new(move |_| {
            Ok(Box::new(CountSource {
                next: 0,
                count: 1000,
                sent: s.clone(),
                on_send: Some(pinged.clone()),
            }) as Box<dyn Source>)
        }),
    );
    let seen_b = seen.clone();
    let closes_b = closes.clone();
    let gate_b = gate.clone();
    registry.register_sink(
        "gated",
        Box::new(move |_| {
            Ok(Box::new(RecordingSink {
                seen: seen_b.clone(),
                closes: closes_b.clone(),
                gate: Some(gate_b.clone()),
            }) as Box<dyn Sink>)
        }),
    );

    // Buffer capacity 4 with a sink gated shut: the source can fill the channel
    // plus the one batch the consumer pulled and is blocked writing, then must
    // stop. It must NOT race ahead to all 1000.
    let config = PipelineConfig::from_value(json!({
        "input": { "type": "fast" },
        "pipeline": { "processors": [], "buffer_capacity": 4 },
        "output": { "type": "gated" },
    }))
    .unwrap();

    let pipeline = Pipeline::build(&registry, &config).unwrap();
    let token = CancellationToken::new();
    let run = tokio::spawn(pipeline.run(token.clone()));

    // Let the source push until it blocks on the full channel.
    on_send.notified().await;
    tokio::time::sleep(Duration::from_millis(50)).await;
    let blocked_at = sent.load(Ordering::SeqCst);
    assert!(
        blocked_at <= 6,
        "a bounded channel (cap 4) must stall the fast source near capacity, got {blocked_at}"
    );

    // Release the sink so the run can finish, then confirm it completed cleanly.
    let releaser = gate.clone();
    tokio::spawn(async move {
        loop {
            releaser.notify_one();
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    });
    let outcome = timeout(Duration::from_secs(10), run)
        .await
        .expect("run finishes once the sink drains")
        .unwrap()
        .unwrap();
    assert_eq!(outcome, RunOutcome::Completed);
    assert_eq!(
        seen.lock().unwrap().len(),
        1000,
        "every batch eventually written"
    );
}

#[tokio::test]
async fn registry_rejects_unknown_node_type() {
    let registry = Registry::new();
    let config = PipelineConfig::from_value(json!({
        "input": { "type": "nope" },
        "pipeline": { "processors": [] },
        "output": { "type": "record" },
    }))
    .unwrap();

    let err = match Pipeline::build(&registry, &config) {
        Ok(_) => panic!("expected build to reject the unknown type"),
        Err(e) => e,
    };
    match err {
        EngineError::Build(msg) => {
            assert!(msg.contains("source"), "names the kind: {msg}");
            assert!(msg.contains("nope"), "names the offending type: {msg}");
        }
        other => panic!("expected a Build error, got {other:?}"),
    }
}

/// A source that fails on its first read, to prove an error propagates and the
/// sink is still closed.
struct FailingSource;

#[async_trait]
impl Source for FailingSource {
    async fn read(&mut self) -> EngineResult<Option<RecordBatch>> {
        Err(EngineError::Source("boom".into()))
    }
}

#[tokio::test]
async fn source_error_propagates_and_sink_is_closed() {
    let closes = Arc::new(AtomicUsize::new(0));
    let mut registry = Registry::new();
    registry.register_source(
        "failing",
        Box::new(|_| Ok(Box::new(FailingSource) as Box<dyn Source>)),
    );
    let closes_b = closes.clone();
    registry.register_sink(
        "record",
        Box::new(move |_| {
            Ok(Box::new(RecordingSink {
                seen: Arc::new(std::sync::Mutex::new(Vec::new())),
                closes: closes_b.clone(),
                gate: None,
            }) as Box<dyn Sink>)
        }),
    );

    let config = PipelineConfig::from_value(json!({
        "input": { "type": "failing" },
        "pipeline": { "processors": [] },
        "output": { "type": "record" },
    }))
    .unwrap();

    let pipeline = Pipeline::build(&registry, &config).unwrap();
    let err = pipeline.run(CancellationToken::new()).await.unwrap_err();
    assert!(matches!(err, EngineError::Source(_)), "source error wins");
    assert_eq!(
        closes.load(Ordering::SeqCst),
        1,
        "sink still closed on error"
    );
}
