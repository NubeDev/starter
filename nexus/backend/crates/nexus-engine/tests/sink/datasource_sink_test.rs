//! The `datasource` sink: batching drives the writer, close flushes the tail,
//! and the Parquet `file` kind produces a file DataFusion can read back.

use std::sync::Arc;

use async_trait::async_trait;
use datafusion::arrow::array::{Int64Array, RecordBatch};
use datafusion::arrow::datatypes::{DataType, Field, Schema};
use datafusion::prelude::SessionContext;
use nexus_engine::core::{EngineResult, Sink};
use nexus_engine::sink::datasource::{DatasourceSink, Writer};
use serde_json::json;
use std::sync::Mutex;
use std::time::Duration;

/// A writer that records each batch's row count, so a test can assert the sink's
/// batching policy without a live datasource.
#[derive(Clone, Default)]
struct RecordingWriter {
    batches: Arc<Mutex<Vec<usize>>>,
}

#[async_trait]
impl Writer for RecordingWriter {
    async fn write_batch(&mut self, batch: &RecordBatch) -> EngineResult<()> {
        self.batches.lock().unwrap().push(batch.num_rows());
        Ok(())
    }

    async fn flush(&mut self) -> EngineResult<()> {
        Ok(())
    }
}

/// A one-column Int64 batch of `n` rows.
fn batch(n: i64) -> RecordBatch {
    let schema = Arc::new(Schema::new(vec![Field::new("v", DataType::Int64, false)]));
    let col = Arc::new(Int64Array::from_iter_values(0..n));
    RecordBatch::try_new(schema, vec![col]).unwrap()
}

#[tokio::test]
async fn batches_by_row_count_then_flushes_the_tail_on_close() {
    let writer = RecordingWriter::default();
    let seen = writer.batches.clone();
    // Flush every 4 rows; a long timer so only the count path fires here.
    let mut sink = DatasourceSink::with_writer(4, Duration::from_secs(60), Box::new(writer));

    sink.write(&batch(3)).await.unwrap();
    assert!(seen.lock().unwrap().is_empty(), "3 < 4 rows: not yet flushed");
    sink.write(&batch(2)).await.unwrap();
    // 3 + 2 = 5 >= 4 → one combined flush of 5 rows.
    assert_eq!(*seen.lock().unwrap(), vec![5]);

    // A trailing sub-threshold batch flushes on close, never lost.
    sink.write(&batch(1)).await.unwrap();
    sink.close().await.unwrap();
    assert_eq!(*seen.lock().unwrap(), vec![5, 1]);
}

#[tokio::test]
async fn file_kind_writes_readable_parquet() {
    let dir = std::env::temp_dir().join(format!(
        "nexus-rw04-parquet-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4()
    ));
    let cfg = json!({
        "kind": "file",
        "dir": dir.to_str().unwrap(),
        "prefix": "readings",
        "batch_rows": 100,
    });
    let mut sink = DatasourceSink::from_config(&cfg).unwrap();
    sink.write(&batch(10)).await.unwrap();
    // close finalizes the open part-file's footer.
    sink.close().await.unwrap();

    // Read every part-file back through DataFusion and count the rows.
    let ctx = SessionContext::new();
    let glob = format!("{}/*.parquet", dir.to_str().unwrap());
    let df = ctx
        .read_parquet(glob, Default::default())
        .await
        .expect("the sink wrote a readable parquet dataset");
    let rows: usize = df
        .collect()
        .await
        .unwrap()
        .iter()
        .map(|b| b.num_rows())
        .sum();
    assert_eq!(rows, 10, "all written rows are readable back");

    let _ = std::fs::remove_dir_all(&dir);
}
