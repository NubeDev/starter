//! JSON rows ↔ Arrow `RecordBatch` at the insight boundary.
//!
//! The query path hands the insight stage its result as JSON row objects (the
//! shape the REST layer already returns). Arrow's own JSON reader infers a schema
//! and parses them into a typed batch — the same approach the federation
//! provider uses for `to_jsonb` rows — so the insight surface never needs a
//! per-type mapping table. On the way out, the transformed batches become JSON
//! rows again for the response.

use std::io::Cursor;
use std::sync::Arc;

use datafusion::arrow::array::RecordBatch;
use datafusion::arrow::datatypes::{Schema, SchemaRef};
use datafusion::arrow::json::reader::{infer_json_schema, ReaderBuilder};
use datafusion::arrow::json::ArrayWriter;
use serde_json::Value;

use super::frame::Frame;
use crate::error::{InsightError, InsightResult};

/// Parse JSON row objects into a [`Frame`]. An empty input is a valid empty
/// frame (no columns). A non-object row is a caller error — the insight stage is
/// fed tabular rows, never scalars.
pub fn rows_to_frame(rows: &[Value]) -> InsightResult<Frame> {
    if rows.is_empty() {
        return Ok(Frame::from_batches(vec![]));
    }
    let ndjson = encode_ndjson(rows)?;
    let schema = infer(&ndjson)?;
    let batch = parse(&ndjson, schema)?;
    Ok(Frame::from_batches(vec![batch]))
}

/// Serialize a frame's batches back to JSON row objects for the response.
pub fn batches_to_rows(batches: &[RecordBatch]) -> InsightResult<Vec<Value>> {
    let mut out = Vec::new();
    for batch in batches {
        if batch.num_rows() == 0 {
            continue;
        }
        let mut buf = Vec::new();
        let mut writer = ArrayWriter::new(&mut buf);
        writer
            .write(batch)
            .map_err(|e| InsightError::Engine(format!("arrow→json write: {e}")))?;
        writer
            .finish()
            .map_err(|e| InsightError::Engine(format!("arrow→json finish: {e}")))?;
        let rows: Vec<Value> = serde_json::from_slice(&buf)
            .map_err(|e| InsightError::Engine(format!("arrow→json parse: {e}")))?;
        out.extend(rows);
    }
    Ok(out)
}

/// Render rows as newline-delimited JSON, rejecting any non-object row up front
/// so the failure names its cause rather than producing a degenerate schema.
fn encode_ndjson(rows: &[Value]) -> InsightResult<String> {
    let mut out = String::new();
    for row in rows {
        if !row.is_object() {
            return Err(InsightError::Runtime(
                "insight input expects object rows".into(),
            ));
        }
        out.push_str(&row.to_string());
        out.push('\n');
    }
    Ok(out)
}

/// Infer the schema across all rows so a field null in the first row but typed
/// later still gets a column.
fn infer(ndjson: &str) -> InsightResult<SchemaRef> {
    let mut cursor = Cursor::new(ndjson.as_bytes());
    let (schema, _read) = infer_json_schema(&mut cursor, None)
        .map_err(|e| InsightError::Engine(format!("schema inference: {e}")))?;
    Ok(Arc::new(schema))
}

/// Parse the NDJSON against the inferred schema into one batch.
fn parse(ndjson: &str, schema: SchemaRef) -> InsightResult<RecordBatch> {
    let mut decoder = ReaderBuilder::new(schema.clone())
        .build_decoder()
        .map_err(|e| InsightError::Engine(format!("json decoder: {e}")))?;
    decoder
        .decode(ndjson.as_bytes())
        .map_err(|e| InsightError::Engine(format!("json decode: {e}")))?;
    match decoder
        .flush()
        .map_err(|e| InsightError::Engine(format!("json flush: {e}")))?
    {
        Some(batch) => Ok(batch),
        None => Ok(RecordBatch::new_empty(schema)),
    }
}

/// Build a one-column empty schema fallback so a fully-empty insight result is
/// still a well-formed (zero-row) frame rather than a panic.
pub(super) fn empty_schema() -> SchemaRef {
    Arc::new(Schema::empty())
}
