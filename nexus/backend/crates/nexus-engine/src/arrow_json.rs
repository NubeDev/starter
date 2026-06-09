//! Convert an Arrow `RecordBatch` into JSON rows plus a coarse column schema.
//!
//! The collector buffers Arrow batches; this is where they become the
//! `Vec<serde_json::Value>` rows and `ColumnSchema` list the REST layer returns.
//! The column types are deliberately coarse — the frontend renders cells, not
//! the full Arrow type lattice.

use datafusion::arrow::array::RecordBatch;
use datafusion::arrow::datatypes::DataType;
use datafusion::arrow::json::ArrayWriter;
use nexus_spi::dto::query::{ColumnSchema, ResultColumnType};
use serde_json::Value;

/// One batch converted to JSON: the rows and their serialized byte size (used
/// for the byte cap and the response stats).
pub struct JsonBatch {
    pub rows: Vec<Value>,
    pub bytes: u64,
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
