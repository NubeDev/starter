//! One row in the admin registry projection.
//!
//! Every kind — tool, node, rule, template, table, skill,
//! extension — projects to this shape. Per-kind extras ride in
//! [`RegistryItem::metadata`] as an opaque `serde_json::Value`
//! object so new keys are additive without a wire bump. See
//! [docs/design/admin/](../../../../docs/design/admin/README.md).

use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

use super::source::ItemSource;

/// One registry row in the canonical admin envelope.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RegistryItem {
    /// Stable item id. Tool ids, node kinds, rule ids, template
    /// names, table names, skill ids, extension ids.
    pub id: String,

    /// Short human label. Falls back to `id` when the registry
    /// entry has no separate display name.
    pub label: String,

    /// One-sentence description. Empty string when none declared.
    pub summary: String,

    /// Where the item came from. See [`ItemSource`].
    pub source: ItemSource,

    /// JSON Schema for the item's input where applicable, else
    /// `null`. Tools, templates and nodes can declare one; skills
    /// and extensions are inherently `null`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_schema: Option<Value>,

    /// JSON Schema for the item's output where applicable. Today
    /// most tools do not declare one; the field exists for
    /// forward-compatibility.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<Value>,

    /// Per-kind extra fields. Always a JSON object on the wire
    /// (empty `{}` when the projector emitted no extras) so
    /// clients can `metadata.tags?.length` without a null check.
    /// The exact keys per kind are documented in
    /// [docs/design/admin/](../../../../docs/design/admin/README.md#per-kind-metadata).
    #[serde(default)]
    pub metadata: Value,
}

impl RegistryItem {
    /// Build a minimal item with empty summary, no schemas, and an
    /// empty metadata object. Projectors layer extras on with the
    /// `with_*` helpers below.
    pub fn new(id: impl Into<String>, source: ItemSource) -> Self {
        let id = id.into();
        Self {
            label: id.clone(),
            id,
            summary: String::new(),
            source,
            input_schema: None,
            output_schema: None,
            metadata: Value::Object(serde_json::Map::new()),
        }
    }

    /// Override the human label.
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// Set the one-sentence summary.
    pub fn with_summary(mut self, summary: impl Into<String>) -> Self {
        self.summary = summary.into();
        self
    }

    /// Attach the input JSON Schema.
    pub fn with_input_schema(mut self, schema: Value) -> Self {
        self.input_schema = Some(schema);
        self
    }

    /// Attach the output JSON Schema.
    pub fn with_output_schema(mut self, schema: Value) -> Self {
        self.output_schema = Some(schema);
        self
    }

    /// Replace the metadata object. Panics in debug if `metadata`
    /// is not a JSON object — the wire contract is "metadata is
    /// always an object".
    pub fn with_metadata(mut self, metadata: Value) -> Self {
        debug_assert!(
            metadata.is_object(),
            "RegistryItem.metadata must be a JSON object"
        );
        self.metadata = metadata;
        self
    }
}
