//! The native pipeline: build nodes from a [`PipelineConfig`] via a [`Registry`]
//! and run them as `source → bounded channel → processors → sink`.
//!
//! Run semantics (the frozen contract, roadmap §6):
//! - The source runs in its own task and pushes batches into a bounded
//!   `tokio::mpsc` channel. The channel bound is the backpressure mechanism: a
//!   fast source blocks on a full channel until the sink drains it.
//! - The consumer loop pulls each batch, runs it through the processor chain in
//!   order (each processor may fan out to several batches), and writes the
//!   results to the sink in arrival order.
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

use datafusion::arrow::array::RecordBatch;

/// A built, runnable pipeline. Construct with [`Pipeline::build`], then drive it
/// once with [`Pipeline::run`]. A pipeline is single-use: `run` consumes the
/// source and sink.
pub struct Pipeline {
    source: Box<dyn Source>,
    processors: Vec<Box<dyn Processor>>,
    sink: Box<dyn Sink>,
    buffer_capacity: usize,
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
        })
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
            processors,
            mut sink,
            buffer_capacity,
        } = self;

        let (tx, rx) = mpsc::channel::<RecordBatch>(buffer_capacity);
        let source_task = tokio::spawn(drive_source(source, tx, token.clone()));

        // Consume until the source channel closes (source done or dropped on
        // cancel). The consumer never reads the token directly: it stops when
        // the channel closes, which the source task guarantees on cancel by
        // dropping `tx`. This drains in-flight batches before closing the sink.
        let consume = consume(rx, &processors, sink.as_mut()).await;

        // The source task carries whether it ended by cancel or by source error.
        let source_result = source_task.await.map_err(|e| {
            EngineError::Source(format!("source task panicked or was aborted: {e}"))
        })?;

        let close = sink.close().await;
        finish(consume, source_result, close, &token)
    }
}

/// Source task body: read batches and forward them until the source is
/// exhausted, the token fires, or the consumer drops the receiver. Dropping `tx`
/// on return is what signals the consumer to stop.
async fn drive_source(
    mut source: Box<dyn Source>,
    tx: mpsc::Sender<RecordBatch>,
    token: CancellationToken,
) -> SourceEnd {
    loop {
        let batch = tokio::select! {
            biased;
            _ = token.cancelled() => return SourceEnd::Cancelled,
            read = source.read() => read,
        };
        match batch {
            Ok(Some(batch)) => {
                // A send error means the consumer is gone (its loop ended on a
                // processor/sink error); stop quietly and let that error win.
                if tx.send(batch).await.is_err() {
                    return SourceEnd::Done;
                }
            }
            Ok(None) => return SourceEnd::Done,
            Err(e) => return SourceEnd::Failed(e),
        }
    }
}

/// Consumer loop: apply the processor chain to each received batch and write the
/// results to the sink in order. Returns on channel close or first node error.
async fn consume(
    mut rx: mpsc::Receiver<RecordBatch>,
    processors: &[Box<dyn Processor>],
    sink: &mut dyn Sink,
) -> EngineResult<()> {
    while let Some(batch) = rx.recv().await {
        for out in process_chain(processors, batch).await? {
            sink.write(&out).await?;
        }
    }
    Ok(())
}

/// Run one batch through every processor in order. Each processor may fan a
/// batch out to several; those all feed the next processor, preserving order.
async fn process_chain(
    processors: &[Box<dyn Processor>],
    batch: RecordBatch,
) -> EngineResult<Vec<RecordBatch>> {
    let mut current = vec![batch];
    for processor in processors {
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
