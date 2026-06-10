//! Per-node debug tap decorators.
//!
//! Each tap wraps one real node ([`Source`], [`Processor`], or [`Sink`]) and,
//! when its flow's [`FlowDebugChannel`] is enabled, publishes a per-node counter
//! tick and a bounded sample of the rows crossing that node's boundary. When
//! debug is off the tap is effectively free: one relaxed atomic load per call,
//! then straight through to the inner node — no row conversion, no allocation.
//!
//! The taps are installed for **every** run (so debug can be toggled mid-run),
//! but the cheap gate keeps an undebugged flow paying almost nothing. Counters
//! accumulate across the run regardless of the gate so that the first tick after
//! enabling reports the true totals.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use datafusion::arrow::array::RecordBatch;
use nexus_spi::dto::flow::{FlowDebugEvent, NodeCounters, NodeRole};

use super::channel::FlowDebugChannel;
use crate::arrow_json::batch_to_rows;
use crate::core::{EngineResult, Pipeline, Processor, Sink, Source};

/// Wrap every node of a built `pipeline` in a debug tap bound to `channel`,
/// assigning the canonical node index — `0` to the source, `1..=N` to the
/// processors in order, `N+1` to the sink — and return the re-assembled
/// pipeline. The taps are always installed; they publish only while `channel`
/// is enabled (see [`FlowDebugChannel`]).
pub fn with_debug(pipeline: Pipeline, channel: FlowDebugChannel) -> Pipeline {
    let (source, processors, sink, buffer_capacity, max_batch_rows) = pipeline.into_parts();

    let source: Box<dyn Source> = Box::new(DebugSource::new(source, 0, channel.clone()));
    let processors: Vec<Box<dyn Processor>> = processors
        .into_iter()
        .enumerate()
        .map(|(i, p)| {
            let idx = (i + 1) as u32;
            Box::new(DebugProcessor::new(p, idx, channel.clone())) as Box<dyn Processor>
        })
        .collect();
    let sink_index = (processors.len() + 1) as u32;
    let sink: Box<dyn Sink> = Box::new(DebugSink::new(sink, sink_index, channel));

    Pipeline::from_parts(source, processors, sink, buffer_capacity, max_batch_rows)
}

/// Running totals one tap accumulates, shared as relaxed atomics so a counter
/// tick reads a consistent-enough snapshot without a lock on the hot path.
#[derive(Default)]
struct Totals {
    rows_in: AtomicU64,
    rows_out: AtomicU64,
    batches: AtomicU64,
}

/// The shared per-node tap state: which node this is, its channel, and its
/// running totals.
struct Tap {
    node_index: u32,
    role: NodeRole,
    channel: FlowDebugChannel,
    totals: Totals,
}

impl Tap {
    fn new(node_index: u32, role: NodeRole, channel: FlowDebugChannel) -> Self {
        Self {
            node_index,
            role,
            channel,
            totals: Totals::default(),
        }
    }

    /// Record a batch crossing the node and, when debug is on, publish a counter
    /// tick and a sampled-rows event. `rows_in`/`rows_out` differ for a processor
    /// that filters or fans out; `batch` is the representative batch to sample.
    fn observe(&self, rows_in: usize, rows_out: usize, sample_from: &RecordBatch) {
        // Always accumulate — cheap, and keeps totals truthful across an
        // enable-mid-run.
        self.totals.rows_in.fetch_add(rows_in as u64, Ordering::Relaxed);
        let rows_out_total = self
            .totals
            .rows_out
            .fetch_add(rows_out as u64, Ordering::Relaxed)
            + rows_out as u64;
        let batches = self.totals.batches.fetch_add(1, Ordering::Relaxed) + 1;
        let rows_in_total = self.totals.rows_in.load(Ordering::Relaxed);

        if !self.channel.is_enabled() {
            return;
        }

        let counters = NodeCounters {
            node_index: self.node_index,
            role: self.role,
            rows_in: rows_in_total,
            rows_out: rows_out_total,
            batches,
        };
        self.channel.publish(FlowDebugEvent::Counters {
            seq: self.channel.next_seq(),
            counters,
        });

        // Sample a bounded slice of the boundary rows. Conversion failures are
        // swallowed — a debug sample must never affect the run.
        let cap = self.channel.sample_rows();
        if cap > 0 && sample_from.num_rows() > 0 {
            let slice = if sample_from.num_rows() > cap {
                sample_from.slice(0, cap)
            } else {
                sample_from.clone()
            };
            if let Ok(json) = batch_to_rows(&slice) {
                self.channel.publish(FlowDebugEvent::Sample {
                    seq: self.channel.next_seq(),
                    node_index: self.node_index,
                    role: self.role,
                    node_id: None,
                    rows: json.rows,
                });
            }
        }
    }
}

/// A [`Source`] wrapped to publish per-node debug for the rows it reads.
pub struct DebugSource {
    inner: Box<dyn Source>,
    tap: Arc<Tap>,
}

impl DebugSource {
    pub fn new(inner: Box<dyn Source>, node_index: u32, channel: FlowDebugChannel) -> Self {
        Self {
            inner,
            tap: Arc::new(Tap::new(node_index, NodeRole::Source, channel)),
        }
    }
}

#[async_trait]
impl Source for DebugSource {
    async fn read(&mut self) -> EngineResult<Option<RecordBatch>> {
        let batch = self.inner.read().await?;
        if let Some(b) = &batch {
            let n = b.num_rows();
            self.tap.observe(n, n, b);
        }
        Ok(batch)
    }

    async fn commit(&mut self) -> EngineResult<()> {
        self.inner.commit().await
    }
}

/// A [`Processor`] wrapped to publish per-node debug for its input/output rows.
pub struct DebugProcessor {
    inner: Box<dyn Processor>,
    tap: Arc<Tap>,
}

impl DebugProcessor {
    pub fn new(inner: Box<dyn Processor>, node_index: u32, channel: FlowDebugChannel) -> Self {
        Self {
            inner,
            tap: Arc::new(Tap::new(node_index, NodeRole::Processor, channel)),
        }
    }
}

#[async_trait]
impl Processor for DebugProcessor {
    async fn process(&mut self, batch: RecordBatch) -> EngineResult<Vec<RecordBatch>> {
        let rows_in = batch.num_rows();
        let out = self.inner.process(batch).await?;
        let rows_out: usize = out.iter().map(RecordBatch::num_rows).sum();
        // Sample the first non-empty output batch so the value view shows what the
        // processor actually produced (post-filter, post-transform).
        if let Some(sample) = out.iter().find(|b| b.num_rows() > 0) {
            self.tap.observe(rows_in, rows_out, sample);
        } else if let Some(empty) = out.first() {
            // Fan-out-to-nothing (e.g. a filter dropping everything): still tick
            // the counter so the UI sees rows_in climb with rows_out flat.
            self.tap.observe(rows_in, rows_out, empty);
        }
        Ok(out)
    }
}

/// A [`Sink`] wrapped to publish per-node debug for the rows reaching the writer.
pub struct DebugSink {
    inner: Box<dyn Sink>,
    tap: Arc<Tap>,
}

impl DebugSink {
    pub fn new(inner: Box<dyn Sink>, node_index: u32, channel: FlowDebugChannel) -> Self {
        Self {
            inner,
            tap: Arc::new(Tap::new(node_index, NodeRole::Sink, channel)),
        }
    }
}

#[async_trait]
impl Sink for DebugSink {
    async fn write(&mut self, batch: &RecordBatch) -> EngineResult<()> {
        let n = batch.num_rows();
        // Observe before the write so a failing sink still shows the rows that
        // reached its boundary.
        self.tap.observe(n, n, batch);
        self.inner.write(batch).await
    }

    async fn close(&mut self) -> EngineResult<()> {
        self.inner.close().await
    }
}
