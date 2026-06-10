//! The native pipeline: build nodes from a [`PipelineConfig`] via a [`Registry`]
//! and run them as `source → bounded channel → processors → sink`.
//!
//! Run semantics (the frozen contract, roadmap §6):
//! - The source runs in its own task and pushes batches into a bounded
//!   `tokio::mpsc` channel. The channel bound is the backpressure mechanism: a
//!   fast source blocks on a full channel until the sink drains it.
//! - Oversized batches are sliced to `max_batch_rows` before the channel, so the
//!   channel depth bounds in-flight *rows*, not just batch count — a single fat
//!   batch can no longer defeat backpressure with green metrics.
//! - The consumer loop pulls each batch, runs it through the processor chain in
//!   order (each processor may fan out to several batches, themselves sliced),
//!   and writes the results to the sink in arrival order. After a source batch is
//!   fully written it acks the source so the source can `commit()` upstream
//!   delivery — the "no silent data loss" hook (default no-op; QoS sources
//!   override it).
//! - **Completion:** the source returns `None`, the channel closes, the consumer
//!   drains the remaining batches, the sink is closed once → [`RunOutcome::Completed`].
//! - **Cancellation:** `token.cancelled()` fires. The source task stops reading
//!   at its next `.await`, the in-flight batches already in the channel are
//!   drained, the sink is closed once → [`RunOutcome::Cancelled`].
//! - **Error:** any node returns `Err`. The run stops, the sink is still closed
//!   once (best-effort flush), and the original error propagates.

use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use super::config::PipelineConfig;
use super::error::{EngineError, EngineResult};
use super::node::{Processor, Sink, Source};
use super::outcome::RunOutcome;
use super::registry::Registry;
use super::slice::slice_to_max;

use datafusion::arrow::array::RecordBatch;

/// A built, runnable pipeline. Construct with [`Pipeline::build`], then drive it
/// once with [`Pipeline::run`]. A pipeline is single-use: `run` consumes the
/// source and sink.
pub struct Pipeline {
    source: Box<dyn Source>,
    processors: Vec<Box<dyn Processor>>,
    sink: Box<dyn Sink>,
    buffer_capacity: usize,
    max_batch_rows: usize,
}

impl Pipeline {
    /// Build every node from `config` using `registry`. Returns
    /// [`EngineError::Build`] if any node names an unregistered type or fails to
    /// construct from its config. No I/O happens here — building is pure setup.
    pub fn build(registry: &Registry, config: &PipelineConfig) -> EngineResult<Self> {
        let source = registry.build_source(&config.input.node_type, &config.input.config)?;
        let processors = config
            .processors
            .iter()
            .map(|p| registry.build_processor(&p.node_type, &p.config))
            .collect::<EngineResult<Vec<_>>>()?;
        let sink = registry.build_sink(&config.output.node_type, &config.output.config)?;
        Ok(Self {
            source,
            processors,
            sink,
            buffer_capacity: config.buffer_capacity,
            max_batch_rows: config.max_batch_rows,
        })
    }

    /// Assemble a pipeline from already-built nodes. The flow layer uses this to
    /// wrap each node from [`Pipeline::build`] in a debug tap before running,
    /// without `core` having to know about the flow debug machinery. `processors`
    /// run in the given order.
    pub fn from_parts(
        source: Box<dyn Source>,
        processors: Vec<Box<dyn Processor>>,
        sink: Box<dyn Sink>,
        buffer_capacity: usize,
        max_batch_rows: usize,
    ) -> Self {
        Self {
            source,
            processors,
            sink,
            buffer_capacity,
            max_batch_rows,
        }
    }

    /// Decompose a built pipeline back into its nodes and tuning, so a caller can
    /// wrap individual nodes and rebuild with [`Pipeline::from_parts`].
    #[allow(clippy::type_complexity)]
    pub fn into_parts(
        self,
    ) -> (
        Box<dyn Source>,
        Vec<Box<dyn Processor>>,
        Box<dyn Sink>,
        usize,
        usize,
    ) {
        (
            self.source,
            self.processors,
            self.sink,
            self.buffer_capacity,
            self.max_batch_rows,
        )
    }

    /// Run the pipeline to completion, cancellation, or first error.
    ///
    /// `close()` is called on the sink exactly once before returning, on every
    /// terminal path (completed, cancelled, error), so a sink can rely on it for
    /// a final flush. On error, the node error is returned after the close
    /// attempt; a close error only surfaces when the run itself succeeded.
    pub async fn run(self, token: CancellationToken) -> EngineResult<RunOutcome> {
        let Pipeline {
            source,
            mut processors,
            mut sink,
            buffer_capacity,
            max_batch_rows,
        } = self;

        let (tx, rx) = mpsc::channel::<RecordBatch>(buffer_capacity);
        // One ack per source batch flows back so the source can commit upstream
        // delivery after the sink has written it. Bounded to the channel depth so
        // an idle source-side commit never accumulates unboundedly.
        let (ack_tx, ack_rx) = mpsc::channel::<()>(buffer_capacity.max(1));
        let source_task = tokio::spawn(drive_source(
            source,
            tx,
            ack_rx,
            token.clone(),
            max_batch_rows,
        ));

        // Consume until the source channel closes (source done or dropped on
        // cancel). The consumer never reads the token directly: it stops when
        // the channel closes, which the source task guarantees on cancel by
        // dropping `tx`. This drains in-flight batches before closing the sink.
        let consume = consume(rx, ack_tx, &mut processors, sink.as_mut(), max_batch_rows).await;

        // The source task carries whether it ended by cancel or by source error.
        let source_result = source_task.await.map_err(|e| {
            EngineError::Source(format!("source task panicked or was aborted: {e}"))
        })?;

        let close = sink.close().await;
        finish(consume, source_result, close, &token)
    }
}

/// Source task body: read batches, slice oversized ones, and forward them until
/// the source is exhausted, the token fires, or the consumer drops the receiver.
/// Between reads it commits any batches the consumer has already acked, in order,
/// without blocking the read loop. Dropping `tx` on return is what signals the
/// consumer to stop; the final commit drain runs after the loop.
async fn drive_source(
    mut source: Box<dyn Source>,
    tx: mpsc::Sender<RecordBatch>,
    mut ack_rx: mpsc::Receiver<()>,
    token: CancellationToken,
    max_batch_rows: usize,
) -> SourceEnd {
    let end = loop {
        // Commit anything already written before reading more, so a QoS source
        // advances its upstream offset promptly without lockstepping the reads.
        if let Err(e) = drain_commits(&mut source, &mut ack_rx).await {
            break SourceEnd::Failed(e);
        }
        let batch = tokio::select! {
            biased;
            _ = token.cancelled() => break SourceEnd::Cancelled,
            read = source.read() => read,
        };
        match batch {
            Ok(Some(batch)) => {
                let mut blocked = false;
                for piece in slice_to_max(batch, max_batch_rows) {
                    // A send error means the consumer is gone (its loop ended on a
                    // processor/sink error); stop quietly and let that error win.
                    if tx.send(piece).await.is_err() {
                        blocked = true;
                        break;
                    }
                }
                if blocked {
                    break SourceEnd::Done;
                }
            }
            Ok(None) => break SourceEnd::Done,
            Err(e) => break SourceEnd::Failed(e),
        }
    };
    // Drain the acks the consumer fired for already-written batches before the
    // run ends, so a clean finish commits everything the sink accepted.
    drop(tx);
    while ack_rx.recv().await.is_some() {
        if source.commit().await.is_err() {
            break;
        }
    }
    end
}

/// Commit each batch the consumer has already acked, in FIFO order, taking only
/// acks ready *now* so the read loop is never stalled waiting for one.
async fn drain_commits(
    source: &mut Box<dyn Source>,
    ack_rx: &mut mpsc::Receiver<()>,
) -> EngineResult<()> {
    while ack_rx.try_recv().is_ok() {
        source.commit().await?;
    }
    Ok(())
}

/// Consumer loop: apply the processor chain to each received batch, write the
/// (sliced) results to the sink in order, then ack the source so it can commit
/// that batch's upstream delivery. Returns on channel close or first node error.
async fn consume(
    mut rx: mpsc::Receiver<RecordBatch>,
    ack_tx: mpsc::Sender<()>,
    processors: &mut [Box<dyn Processor>],
    sink: &mut dyn Sink,
    max_batch_rows: usize,
) -> EngineResult<()> {
    while let Some(batch) = rx.recv().await {
        for out in process_chain(processors, batch).await? {
            for piece in slice_to_max(out, max_batch_rows) {
                sink.write(&piece).await?;
            }
        }
        // The batch is durably with the sink; ack so the source may commit. A
        // closed ack channel means the source already ended — harmless to ignore.
        let _ = ack_tx.send(()).await;
    }
    Ok(())
}

/// Run one batch through every processor in order. Each processor may fan a
/// batch out to several; those all feed the next processor, preserving order.
async fn process_chain(
    processors: &mut [Box<dyn Processor>],
    batch: RecordBatch,
) -> EngineResult<Vec<RecordBatch>> {
    let mut current = vec![batch];
    for processor in processors.iter_mut() {
        let mut next = Vec::with_capacity(current.len());
        for batch in current {
            next.extend(processor.process(batch).await?);
        }
        current = next;
    }
    Ok(current)
}

/// How the source task ended, carried back so the run can tell a cancel from a
/// clean finish from a source error.
enum SourceEnd {
    /// Source returned `None`, or the consumer dropped the receiver.
    Done,
    /// The cancellation token fired before end-of-stream.
    Cancelled,
    /// The source raised an error.
    Failed(EngineError),
}

/// Collapse the three result strands into one outcome. A consumer/processor/sink
/// error wins over everything; then a source error; then cancellation; then a
/// close error; otherwise completed.
fn finish(
    consume: EngineResult<()>,
    source: SourceEnd,
    close: EngineResult<()>,
    token: &CancellationToken,
) -> EngineResult<RunOutcome> {
    consume?;
    if let SourceEnd::Failed(e) = source {
        return Err(e);
    }
    close?;
    if matches!(source, SourceEnd::Cancelled) || token.is_cancelled() {
        Ok(RunOutcome::Cancelled)
    } else {
        Ok(RunOutcome::Completed)
    }
}
