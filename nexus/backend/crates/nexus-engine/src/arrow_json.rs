//! Convert an Arrow `RecordBatch` into JSON rows plus a coarse column schema.
//!
//! The collector buffers Arrow batches; this is where they become the
//! `Vec<serde_json::Value>` rows and `ColumnSchema` list the REST layer returns.
//! The column types are deliberately coarse — the frontend renders cells, not
//! the full Arrow type lattice.

use std::sync::Arc;

use datafusion::arrow::array::{Array, RecordBatch, StringArray};
use datafusion::arrow::datatypes::{DataType, Field, Schema};
use datafusion::arrow::json::ArrayWriter;
use nexus_spi::dto::query::{ColumnSchema, ResultColumnType};
use serde_json::Value;

/// The column name a native JSON source uses to carry each raw JSON document
/// into the `json_to_arrow` processor.
///
/// A native source that emits JSON (memory/generate/http_poll/simulator) cannot
/// hand a typed Arrow batch downstream — it does not know the schema. Instead it
/// emits a single Utf8 column of JSON-document strings under this name, and
/// [`super::processor::json_to_arrow`] infers a schema and parses it into a typed
/// batch. The single-value-field convention keeps a stored flow that pipes a
/// JSON source into `json_to_arrow` working unchanged.
pub const JSON_VALUE_FIELD: &str = "__value__";

/// One batch converted to JSON: the rows and their serialized byte size (used
/// for the byte cap and the response stats).
pub struct JsonBatch {
    pub rows: Vec<Value>,
    pub bytes: u64,
}

/// Wrap a slice of raw JSON-document strings as a one-column carrier batch under
/// [`JSON_VALUE_FIELD`]. The native JSON sources emit this; `json_to_arrow`
/// unwraps it. An empty slice yields an empty batch with the carrier schema.
pub fn json_carrier_batch(docs: &[String]) -> RecordBatch {
    let schema = Arc::new(Schema::new(vec![Field::new(
        JSON_VALUE_FIELD,
        DataType::Utf8,
        false,
    )]));
    let column = Arc::new(StringArray::from_iter_values(docs.iter().map(String::as_str)));
    // try_new only fails on a column/length mismatch, which a single column built
    // from the same slice cannot hit.
    RecordBatch::try_new(schema, vec![column])
        .expect("single-column carrier batch is always well-formed")
}

/// Read the JSON-document strings out of a [`json_carrier_batch`]. Returns an
/// error if the batch is not a single `__value__` Utf8 column, which means a
/// non-JSON source was wired into `json_to_arrow`.
pub fn json_carrier_docs(batch: &RecordBatch) -> Result<Vec<String>, String> {
    let column = batch
        .column_by_name(JSON_VALUE_FIELD)
        .ok_or_else(|| format!("json_to_arrow expects a {JSON_VALUE_FIELD} column"))?;
    let strings = column
        .as_any()
        .downcast_ref::<StringArray>()
        .ok_or_else(|| format!("json_to_arrow {JSON_VALUE_FIELD} column must be Utf8"))?;
    Ok((0..strings.len())
        .map(|i| strings.value(i).to_string())
        .collect())
}

/// Serialize a `RecordBatch` to a JSON array of row objects. Returns the rows
/// and the byte size of the serialization so the caller can account for caps
/// without re-serializing.
pub fn batch_to_rows(batch: &RecordBatch) -> Result<JsonBatch, String> {
    let mut buf = Vec::new();
    let mut writer = ArrayWriter::new(&mut buf);
    writer
        .write(batch)
        .map_err(|e| format!("arrow→json write failed: {e}"))?;
    writer
        .finish()
        .map_err(|e| format!("arrow→json finish failed: {e}"))?;

    let bytes = buf.len() as u64;
    let rows: Vec<Value> =
        serde_json::from_slice(&buf).map_err(|e| format!("arrow→json parse failed: {e}"))?;
    Ok(JsonBatch { rows, bytes })
}

/// Derive the coarse column schema from a batch's Arrow schema.
pub fn columns_of(batch: &RecordBatch) -> Vec<ColumnSchema> {
    batch
        .schema()
        .fields()
        .iter()
        .map(|f| ColumnSchema {
            name: f.name().clone(),
            column_type: coarse_type(f.data_type()),
        })
        .collect()
}

/// Map an Arrow `DataType` to the coarse type the frontend renders against.
fn coarse_type(dt: &DataType) -> ResultColumnType {
    match dt {
        DataType::Boolean => ResultColumnType::Bool,
        DataType::Int8
        | DataType::Int16
        | DataType::Int32
        | DataType::Int64
        | DataType::UInt8
        | DataType::UInt16
        | DataType::UInt32
        | DataType::UInt64 => ResultColumnType::Int,
        DataType::Float16 | DataType::Float32 | DataType::Float64 => ResultColumnType::Float,
        DataType::Utf8 | DataType::LargeUtf8 => ResultColumnType::String,
        DataType::Date32
        | DataType::Date64
        | DataType::Time32(_)
        | DataType::Time64(_)
        | DataType::Timestamp(_, _) => ResultColumnType::Timestamp,
        _ => ResultColumnType::Other,
    }
}
