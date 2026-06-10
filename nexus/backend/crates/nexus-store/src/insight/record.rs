//! Insight records and their create/update inputs.

use serde_json::Value;
use uuid::Uuid;

/// A stored insight: a named, tenant-scoped post-query transform script.
#[derive(Debug, Clone)]
pub struct InsightRecord {
    pub id: Uuid,
    pub tenant_id: String,
    pub name: String,
    pub script: String,
    /// Advisory JSON-Schema for the script's params (UI only).
    pub params_schema: Option<Value>,
}

/// Input to create an insight.
#[derive(Debug, Clone)]
pub struct NewInsight {
    pub name: String,
    pub script: String,
    pub params_schema: Option<Value>,
}

/// Partial update. `None` leaves a field untouched (COALESCE in the store).
#[derive(Debug, Clone, Default)]
pub struct InsightPatch {
    pub name: Option<String>,
    pub script: Option<String>,
    pub params_schema: Option<Value>,
}
