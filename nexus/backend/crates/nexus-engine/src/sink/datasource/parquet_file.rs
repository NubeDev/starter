//! The `file` `DatasourceWriter`: append batches to rotating Parquet part-files.
//!
//! Parquet is columnar and only pays off with substantial row groups, so this
//! writer rotates a part-file on SIZE or ROWS — never on the sink's batch timer.
//! Timer-flushed tiny row groups are exactly what makes a Parquet dataset slow to
//! read, so the sink's `batch_ms` flush calls [`DatasourceWriter::flush`] (a
//! no-op here unless a threshold was crossed), while the part-file rolls over only
//! when it reaches the size or row threshold. `close`/cancel finalizes the open
//! part-file so no rows are stranded in an unclosed footer.
//!
//! Local filesystem is the v1 target: each part-file is a real file under the
//! configured directory, written through `AsyncArrowWriter`. An S3/object-store
//! backend is a later concern behind the crate's `s3` feature; the local path
//! pulls no object-store client.

use std::path::PathBuf;

use async_trait::async_trait;
use datafusion::arrow::array::RecordBatch;
use datafusion::arrow::datatypes::SchemaRef;
use datafusion::parquet::arrow::AsyncArrowWriter;
use tokio::fs::File;

use super::writer::DatasourceWriter;
use crate::core::{EngineError, EngineResult};

/// Rotate a part-file once it reaches ~64MB. Below this, Parquet row groups are
/// too small to compress or scan efficiently — the small-files problem the spec
/// guards against.
const ROTATE_BYTES: usize = 64 * 1024 * 1024;

/// Writes batches into rotating Parquet part-files under a directory, rolling a
/// new file on size or row threshold.
pub struct ParquetFileWriter {
    dir: PathBuf,
    /// Base name for part-files: `<prefix>-<n>.parquet`.
    prefix: String,
    rotate_rows: usize,
    /// The open writer + its schema + the rows it holds, or `None` before the
    /// first batch establishes the stream schema.
    open: Option<OpenPart>,
    /// Monotonic part counter for unique file names within one run.
    part: usize,
}

/// An in-progress part-file: the async writer, the schema it was opened with, and
/// the rows written to it so far (for the row-rotation threshold).
struct OpenPart {
    writer: AsyncArrowWriter<File>,
    schema: SchemaRef,
    rows: usize,
}

impl ParquetFileWriter {
    /// Build a writer rooted at `dir`, naming part-files with `prefix` and
    /// rotating after `rotate_rows` rows (or [`ROTATE_BYTES`], whichever first).
    /// The directory is created on the first write, not here, so building stays
    /// side-effect-free like every other node.
    pub fn new(dir: PathBuf, prefix: String, rotate_rows: usize) -> Self {
        Self {
            dir,
            prefix,
            rotate_rows: rotate_rows.max(1),
            open: None,
            part: 0,
        }
    }

    /// Open a fresh part-file with `schema`, creating the directory if needed.
    async fn open_part(&mut self, schema: SchemaRef) -> EngineResult<()> {
        tokio::fs::create_dir_all(&self.dir)
            .await
            .map_err(|e| EngineError::Sink(format!("file sink mkdir failed: {e}")))?;
        let path = self
            .dir
            .join(format!("{}-{}.parquet", self.prefix, self.part));
        self.part += 1;
        let file = File::create(&path)
            .await
            .map_err(|e| EngineError::Sink(format!("file sink create {path:?} failed: {e}")))?;
        let writer = AsyncArrowWriter::try_new(file, schema.clone(), None)
            .map_err(|e| EngineError::Sink(format!("parquet writer init failed: {e}")))?;
        self.open = Some(OpenPart {
            writer,
            schema,
            rows: 0,
        });
        Ok(())
    }

    /// Close the open part-file, finalizing its footer. A no-op when no part is
    /// open.
    async fn close_part(&mut self) -> EngineResult<()> {
        if let Some(part) = self.open.take() {
            part.writer
                .close()
                .await
                .map_err(|e| EngineError::Sink(format!("parquet close failed: {e}")))?;
        }
        Ok(())
    }
}

#[async_trait]
impl DatasourceWriter for ParquetFileWriter {
    async fn write_batch(&mut self, batch: &RecordBatch) -> EngineResult<()> {
        if batch.num_rows() == 0 {
            return Ok(());
        }
        // A schema change mid-stream cannot share a Parquet file; roll over so
        // each part-file is internally consistent. The engine's schema-stability
        // contract makes this rare, but the writer must not corrupt a footer.
        let schema_changed = self
            .open
            .as_ref()
            .is_some_and(|p| p.schema != batch.schema());
        if schema_changed {
            self.close_part().await?;
        }
        if self.open.is_none() {
            self.open_part(batch.schema()).await?;
        }

        let part = self.open.as_mut().expect("part opened above");
        part.writer
            .write(batch)
            .await
            .map_err(|e| EngineError::Sink(format!("parquet write failed: {e}")))?;
        part.rows += batch.num_rows();

        // Rotate on size or rows — whichever crosses first. `in_progress_size`
        // reflects buffered-but-unflushed bytes plus flushed groups, a good proxy
        // for the eventual file size.
        let too_big = part.writer.in_progress_size() + part.writer.bytes_written() >= ROTATE_BYTES;
        let too_many = part.rows >= self.rotate_rows;
        if too_big || too_many {
            self.close_part().await?;
        }
        Ok(())
    }

    async fn flush(&mut self) -> EngineResult<()> {
        // Finalize whatever part is open so end-of-run / cancellation leaves a
        // readable file. The next batch (if any) opens a new part.
        self.close_part().await
    }
}
