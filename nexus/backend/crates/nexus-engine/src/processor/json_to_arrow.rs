//! The native `json_to_arrow` processor: parse a JSON carrier batch into a
//! typed Arrow batch with a stable, stream-wide schema.
//!
//! A native JSON source emits one Utf8 column of raw documents (see
//! [`crate::arrow_json::JSON_VALUE_FIELD`]); this processor turns each document
//! into typed columns the `sql` processor can query.
//!
//! Schema stability (roadmap §6): ArkFlow re-inferred a schema per batch, which
//! lets column types drift mid-stream — an accident, not a contract. This
//! implementation locks the schema once: a `schema` declared in config wins;
//! otherwise the first non-empty batch's inferred schema becomes the stream
//! schema. Every later batch is parsed against that fixed schema, so a document
//! whose shape no longer fits surfaces as a processor error rather than a silent
//! type change downstream.

use std::io::Cursor;
use std::sync::{Arc, Mutex};

use datafusion::arrow::array::RecordBatch;
use datafusion::arrow::datatypes::SchemaRef;
use datafusion::arrow::json::reader::{infer_json_schema, ReaderBuilder};
use serde_json::Value;

use super::declared_schema;
use crate::arrow_json::json_carrier_docs;
use crate::core::{EngineError, EngineResult, Processor};

/// Parses JSON carrier batches into typed Arrow batches against a schema fixed
/// for the life of the stream.
pub struct JsonToArrow {
    /// A schema declared in flow config, if any. When set it always wins, so a
    /// warehouse sink gets a stable schema even before the first batch arrives.
    declared: Option<SchemaRef>,
    /// The schema locked in from the first non-empty batch when none was
    /// declared. `None` until the first batch is seen.
    inferred: Mutex<Option<SchemaRef>>,
}

impl JsonToArrow {
    /// Build from the node config. An optional `schema` field is an Arrow schema
    /// in serde-JSON form (`{ "fields": [...] }`); when absent the schema is
    /// inferred from the first batch. Returns [`EngineError::Build`] on a
    /// malformed declared schema.
    pub fn from_config(config: &Value) -> EngineResult<Self> {
        Ok(Self {
            declared: declared_schema::parse(config)?,
            inferred: Mutex::new(None),
        })
    }

    /// Resolve the schema to parse `docs` against: the declared schema, else the
    /// already-locked inferred schema, else infer from `docs` and lock it. The
    /// returned schema is used for every column of the output batch, so all
    /// batches in one stream share it.
    fn stream_schema(&self, docs: &[String]) -> EngineResult<SchemaRef> {
        if let Some(schema) = &self.declared {
            return Ok(schema.clone());
        }
        let mut guard = self.inferred.lock().expect("json_to_arrow schema lock");
        if let Some(schema) = guard.as_ref() {
            return Ok(schema.clone());
        }
        let schema = infer_schema(docs)?;
        *guard = Some(schema.clone());
        Ok(schema)
    }
}

#[async_trait::async_trait]
impl Processor for JsonToArrow {
    async fn process(&self, batch: RecordBatch) -> EngineResult<Vec<RecordBatch>> {
        let docs = json_carrier_docs(&batch).map_err(EngineError::Processor)?;
        if docs.is_empty() {
            return Ok(Vec::new());
        }
        let schema = self.stream_schema(&docs)?;
        let parsed = parse_docs(&docs, schema)?;
        Ok(vec![parsed])
    }
}

/// Infer a schema from the documents, reading all of them so a field that is
/// null in the first row but typed in a later row still gets a type. An empty
/// or all-null stream has no inferable types and is a processor error rather
/// than a silent empty schema that would drop every column.
fn infer_schema(docs: &[String]) -> EngineResult<SchemaRef> {
    let joined = docs.join("\n");
    let mut cursor = Cursor::new(joined.as_bytes());
    let (schema, _read) = infer_json_schema(&mut cursor, None)
        .map_err(|e| EngineError::Processor(format!("json_to_arrow schema inference: {e}")))?;
    if schema.fields().is_empty() {
        return Err(EngineError::Processor(
            "json_to_arrow could not infer any column from the batch".into(),
        ));
    }
    Ok(Arc::new(schema))
}

/// Parse the documents against `schema`. A document that does not coerce to the
/// fixed schema (a column whose type drifted) fails here, honouring the
/// schema-stability contract.
fn parse_docs(docs: &[String], schema: SchemaRef) -> EngineResult<RecordBatch> {
    let joined = docs.join("\n");
    let mut decoder = ReaderBuilder::new(schema.clone())
        .build_decoder()
        .map_err(|e| EngineError::Processor(format!("json_to_arrow decoder: {e}")))?;
    decoder
        .decode(joined.as_bytes())
        .map_err(|e| EngineError::Processor(format!("json_to_arrow decode: {e}")))?;
    match decoder
        .flush()
        .map_err(|e| EngineError::Processor(format!("json_to_arrow flush: {e}")))?
    {
        Some(batch) => Ok(batch),
        // A non-empty input that flushes nothing means every document failed to
        // match the locked schema.
        None => Ok(RecordBatch::new_empty(schema)),
    }
}
