//! Turn a slice of JSON row objects into one typed Arrow `RecordBatch`.
//!
//! The Postgres federation provider pulls each remote row as a JSON object
//! (`to_jsonb(t)`), which sidesteps a per-Postgres-type → Arrow mapping table:
//! Arrow's own JSON reader infers the schema and parses the values. The schema
//! is inferred from the rows actually fetched, so it reflects the data this scan
//! sees — federation re-fetches per scan, so there is no cross-scan drift to
//! guard against here (unlike the streaming `json_to_arrow` processor).

use std::io::Cursor;
use std::sync::Arc;

use datafusion::arrow::array::RecordBatch;
use datafusion::arrow::datatypes::{Schema, SchemaRef};
use datafusion::arrow::json::reader::{infer_json_schema, ReaderBuilder};
use serde_json::Value;

use crate::core::{EngineError, EngineResult};

/// Infer a schema from `rows` and parse them into one batch. An empty input
/// yields an empty batch with no columns — a valid (if degenerate) table that a
/// join treats as zero rows. Rows must be JSON objects; a non-object row is a
/// provider error (the remote query did not return tabular JSON).
pub fn json_rows_to_batch(rows: &[Value]) -> EngineResult<RecordBatch> {
    if rows.is_empty() {
        return Ok(RecordBatch::new_empty(Arc::new(Schema::empty())));
    }
    let docs = encode_ndjson(rows)?;
    let schema = infer(&docs)?;
    parse(&docs, schema)
}

/// Render the rows as newline-delimited JSON for Arrow's reader, rejecting any
/// non-object row up front so the failure names the cause.
fn encode_ndjson(rows: &[Value]) -> EngineResult<String> {
    let mut out = String::new();
    for row in rows {
        if !row.is_object() {
            return Err(EngineError::Source(
                "federation postgres provider expects object rows".into(),
            ));
        }
        out.push_str(&row.to_string());
        out.push('\n');
    }
    Ok(out)
}

/// Infer the batch schema from all rows so a field null in the first row but
/// typed later still gets a column.
fn infer(ndjson: &str) -> EngineResult<SchemaRef> {
    let mut cursor = Cursor::new(ndjson.as_bytes());
    let (schema, _read) = infer_json_schema(&mut cursor, None)
        .map_err(|e| EngineError::Source(format!("federation schema inference: {e}")))?;
    Ok(Arc::new(schema))
}

/// Parse the NDJSON against the inferred schema into one batch.
fn parse(ndjson: &str, schema: SchemaRef) -> EngineResult<RecordBatch> {
    let mut decoder = ReaderBuilder::new(schema.clone())
        .build_decoder()
        .map_err(|e| EngineError::Source(format!("federation decoder: {e}")))?;
    decoder
        .decode(ndjson.as_bytes())
        .map_err(|e| EngineError::Source(format!("federation decode: {e}")))?;
    match decoder
        .flush()
        .map_err(|e| EngineError::Source(format!("federation flush: {e}")))?
    {
        Some(batch) => Ok(batch),
        None => Ok(RecordBatch::new_empty(schema)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn empty_rows_yield_empty_batch() {
        let batch = json_rows_to_batch(&[]).unwrap();
        assert_eq!(batch.num_rows(), 0);
        assert_eq!(batch.num_columns(), 0);
    }

    #[test]
    fn objects_become_typed_columns() {
        let rows = vec![
            json!({ "id": 1, "name": "a" }),
            json!({ "id": 2, "name": "b" }),
        ];
        let batch = json_rows_to_batch(&rows).unwrap();
        assert_eq!(batch.num_rows(), 2);
        assert_eq!(batch.num_columns(), 2);
    }

    #[test]
    fn non_object_row_is_an_error() {
        let rows = vec![json!(42)];
        assert!(json_rows_to_batch(&rows).is_err());
    }
}
