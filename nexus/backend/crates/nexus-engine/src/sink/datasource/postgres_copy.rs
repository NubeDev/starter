//! The Postgres `DatasourceWriter`: bulk-load each batch with `COPY FROM STDIN`.
//!
//! COPY is the right bulk primitive — one streamed transfer per batch instead of
//! one round-trip per row (`pg_insert`'s old shape). It also covers Timescale,
//! which is Postgres on the wire. We use sqlx's `PgCopyIn` (the workspace already
//! ships sqlx; tokio-postgres' `BinaryCopyInWriter` would be a second client
//! stack, which roadmap §8 forbids), and own the text-format encoding: text COPY
//! carries every JSON scalar the Arrow→JSON bridge produces via its text
//! representation, with non-scalars round-tripping as JSON text into a text/jsonb
//! column — the same lossless fallback the row-insert path used.
//!
//! Every row in a batch must expose the same column set (the engine's schema
//! stability contract guarantees this); the column order is taken from the first
//! row and every value is validated as an identifier before it reaches SQL text.

use async_trait::async_trait;
use datafusion::arrow::array::RecordBatch;
use serde_json::{Map, Value};
use sqlx::postgres::PgPoolCopyExt;
use sqlx::PgPool;

use super::identifier::validate_identifier;
use super::writer::DatasourceWriter;
use crate::arrow_json::batch_to_rows;
use crate::core::{EngineError, EngineResult};

/// Writes batches into one Postgres table via `COPY ... FROM STDIN`.
pub struct PostgresCopyWriter {
    pool: PgPool,
    /// Validated, unqualified table name. Validated once at build time so the
    /// per-batch path never re-checks it.
    table: String,
}

impl PostgresCopyWriter {
    /// Build from an open pool and a target table. The table name is validated
    /// here (allowlisted identifier shape) so an invalid name fails the flow at
    /// build time, not mid-stream.
    pub fn new(pool: PgPool, table: &str) -> EngineResult<Self> {
        validate_identifier(table)?;
        Ok(Self {
            pool,
            table: table.to_string(),
        })
    }
}

#[async_trait]
impl DatasourceWriter for PostgresCopyWriter {
    async fn write_batch(&mut self, batch: &RecordBatch) -> EngineResult<()> {
        if batch.num_rows() == 0 {
            return Ok(());
        }
        let rows = batch_to_rows(batch).map_err(EngineError::Sink)?.rows;
        let objects = as_objects(&rows)?;
        // Column order is the first row's key order; every row carries the same
        // keys under the schema-stability contract, so one column list is correct
        // for the whole batch.
        let columns: Vec<&String> = objects[0].keys().collect();
        for col in &columns {
            validate_identifier(col)?;
        }
        let column_list = columns
            .iter()
            .map(|c| format!("\"{c}\""))
            .collect::<Vec<_>>()
            .join(", ");

        let stmt = format!(
            "COPY \"{}\" ({column_list}) FROM STDIN WITH (FORMAT text)",
            self.table
        );
        let mut copy = self
            .pool
            .copy_in_raw(&stmt)
            .await
            .map_err(|e| EngineError::Sink(format!("postgres COPY begin failed: {e}")))?;

        let payload = encode_text(&objects, &columns);
        if let Err(e) = copy.send(payload.as_bytes()).await {
            // Abort releases the connection's COPY state so the pool stays usable.
            let _ = copy.abort(format!("send failed: {e}")).await;
            return Err(EngineError::Sink(format!("postgres COPY send failed: {e}")));
        }
        copy.finish()
            .await
            .map_err(|e| EngineError::Sink(format!("postgres COPY finish failed: {e}")))?;
        Ok(())
    }

    async fn flush(&mut self) -> EngineResult<()> {
        // Each `write_batch` is a complete COPY transaction; nothing is held back.
        Ok(())
    }
}

/// Narrow the JSON rows to objects, rejecting a non-object row (a sink fed
/// non-tabular data) rather than silently dropping it.
fn as_objects(rows: &[Value]) -> EngineResult<Vec<&Map<String, Value>>> {
    rows.iter()
        .map(|r| {
            r.as_object()
                .ok_or_else(|| EngineError::Sink("datasource sink expects object rows".into()))
        })
        .collect()
}

/// Encode rows as Postgres COPY text: tab-separated columns, newline-terminated
/// rows, with the four text-format escapes applied to every field. A missing key
/// in a later row is written as `\N` (NULL) so a heterogeneous-but-coercible
/// batch still loads.
fn encode_text(objects: &[&Map<String, Value>], columns: &[&String]) -> String {
    // Pre-size by columns so a wide batch does not repeatedly reallocate.
    let mut out = String::with_capacity(objects.len() * columns.len() * 8);
    for obj in objects {
        for (i, col) in columns.iter().enumerate() {
            if i > 0 {
                out.push('\t');
            }
            out.push_str(&encode_field(obj.get(*col).unwrap_or(&Value::Null)));
        }
        out.push('\n');
    }
    out
}

/// Render one JSON value as a COPY text field. `null` becomes the unquoted `\N`
/// sentinel; scalars use their text form; arrays/objects serialize as JSON text
/// (the lossless fallback for a jsonb/text column).
fn encode_field(value: &Value) -> String {
    match value {
        Value::Null => "\\N".to_string(),
        Value::Bool(b) => {
            if *b {
                "t".to_string()
            } else {
                "f".to_string()
            }
        }
        Value::Number(n) => n.to_string(),
        Value::String(s) => escape(s),
        other => escape(&other.to_string()),
    }
}

/// Apply the COPY text-format escapes: backslash, tab, newline, carriage return.
/// Anything else passes through verbatim. Without this a value containing a tab
/// or newline would corrupt the column/row framing.
fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn obj(v: Value) -> Map<String, Value> {
        v.as_object().unwrap().clone()
    }

    #[test]
    fn encodes_scalars_and_nulls() {
        let row = obj(json!({ "a": 1, "b": "x", "c": null, "d": true }));
        let cols: Vec<String> = vec!["a".into(), "b".into(), "c".into(), "d".into()];
        let refs: Vec<&String> = cols.iter().collect();
        let line = encode_text(&[&row], &refs);
        assert_eq!(line, "1\tx\t\\N\tt\n");
    }

    #[test]
    fn escapes_tab_newline_backslash() {
        assert_eq!(escape("a\tb"), "a\\tb");
        assert_eq!(escape("a\nb"), "a\\nb");
        assert_eq!(escape("a\\b"), "a\\\\b");
    }

    #[test]
    fn nested_value_round_trips_as_json_text() {
        let f = encode_field(&json!({ "k": 1 }));
        assert_eq!(f, "{\"k\":1}");
    }
}
