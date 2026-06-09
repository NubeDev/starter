//! Row and input shapes for the flows store.

use serde_json::Value;
use uuid::Uuid;

/// A saved flow as stored: the three config blobs plus its enabled flag. The
/// configs are opaque `Value`s here — the FlowManager validates them when it
/// builds the stream, the store only persists them.
#[derive(Debug, Clone)]
pub struct FlowRecord {
    pub id: Uuid,
    pub tenant_id: String,
    pub name: String,
    pub input: Value,
    pub pipeline: Value,
    pub output: Value,
    pub enabled: bool,
}

/// A new flow to insert.
#[derive(Debug, Clone)]
pub struct NewFlow {
    pub name: String,
    pub input: Value,
    pub pipeline: Value,
    pub output: Value,
    pub enabled: bool,
}

/// A partial update; `None` fields are left unchanged.
#[derive(Debug, Clone, Default)]
pub struct FlowPatch {
    pub name: Option<String>,
    pub input: Option<Value>,
    pub pipeline: Option<Value>,
    pub output: Option<Value>,
    pub enabled: Option<bool>,
}
