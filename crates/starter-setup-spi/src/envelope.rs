//! YAML envelope (DOCS §6). One file is one template — an envelope whose
//! `flow:` key holds a `FlowBody`.
//!
//! **The import path is NOT `parse_flow_file`.** That parser expects a
//! top-level `flow_id` and returns the whole body; our file is an
//! envelope with the flow body *nested* under `flow:`. Here we:
//!
//! 1. `serde_yaml` → [`TemplateEnvelope`],
//! 2. take the nested `flow` value and deserialize **only that** into a
//!    `FlowBody` (injecting `flow_id` from the envelope `id` when the
//!    nested body omits it, matching the §6 example), and
//! 3. hand the `FlowBody` to the caller to validate via the flow layer's
//!    body-level resolver (done in `starter-setup`, which has the
//!    node-kind registry).

use serde::{Deserialize, Serialize};
use starter_flow::definition::body::FlowBody;

use crate::error::SetupError;
use crate::model::{
    InputBinding, OutputBinding, SemVer, Template, TemplateAccess, TemplateId, TemplateSource,
};

/// The on-disk / on-wire envelope. Mirrors the §6 YAML exactly: scalar
/// metadata, an `input_schema`, the bindings, the `access` block, and a
/// nested `flow` value that becomes a `FlowBody`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TemplateEnvelope {
    /// Reverse-DNS template id.
    pub id: String,
    /// Semantic version string (`"1.2.0"`).
    pub version: SemVer,
    /// Display name.
    pub display_name: String,
    /// Description.
    #[serde(default)]
    pub description: String,
    /// Icon name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// Nav category.
    #[serde(default)]
    pub category: String,
    /// JSON-Schema launcher form.
    #[serde(default = "empty_schema")]
    pub input_schema: serde_json::Value,
    /// Form-field → entry-slot bindings.
    #[serde(default)]
    pub input_bindings: Vec<InputBinding>,
    /// Terminal-slot → result-field bindings.
    #[serde(default)]
    pub output_bindings: Vec<OutputBinding>,
    /// Access block (allowed teams, run role). Tenant is supplied by the
    /// import call site, not the file.
    #[serde(default)]
    pub access: TemplateAccess,
    /// The nested flow body — deserialized into a `FlowBody` (DOCS §6).
    pub flow: serde_yaml::Value,
}

fn empty_schema() -> serde_json::Value {
    serde_json::json!({ "type": "object" })
}

impl TemplateEnvelope {
    /// Parse a raw YAML document into an envelope.
    pub fn from_yaml(yaml: &str) -> Result<Self, SetupError> {
        serde_yaml::from_str(yaml).map_err(|e| SetupError::InvalidYaml(e.to_string()))
    }

    /// Deserialize the nested `flow` value into a `FlowBody`, injecting
    /// `flow_id` from the envelope `id` when the nested body omits it (the
    /// §6 example body carries no `flow_id`). Does **not** validate node
    /// kinds — that is the caller's resolver step.
    pub fn flow_body(&self) -> Result<FlowBody, SetupError> {
        // Re-encode the nested flow value as JSON so we can inject the
        // flow_id uniformly and deserialize through serde_json (FlowBody's
        // canonical wire form).
        let mut flow_json: serde_json::Value = serde_json::to_value(&self.flow)
            .map_err(|e| SetupError::InvalidYaml(format!("flow body: {e}")))?;
        let obj = flow_json
            .as_object_mut()
            .ok_or_else(|| SetupError::InvalidYaml("flow: must be a mapping".into()))?;
        obj.entry("flow_id")
            .or_insert_with(|| serde_json::Value::String(self.id.clone()));
        serde_json::from_value(flow_json).map_err(|e| SetupError::InvalidBody(e.to_string()))
    }

    /// Build a [`Template`] from this envelope, binding it to `tenant_id`
    /// (`None` => the `__global__` extension catalog) and recording
    /// `source`. The `flow_body` is parsed but **not** node-validated here.
    pub fn into_template(
        self,
        tenant_id: Option<String>,
        source: TemplateSource,
    ) -> Result<Template, SetupError> {
        let flow_body = self.flow_body()?;
        let mut access = self.access;
        access.tenant_id = tenant_id;
        Ok(Template {
            id: TemplateId(self.id),
            version: self.version,
            display_name: self.display_name,
            description: self.description,
            icon: self.icon,
            category: self.category,
            input_schema: self.input_schema,
            flow_body,
            input_bindings: self.input_bindings,
            output_bindings: self.output_bindings,
            access,
            source,
        })
    }
}

impl Template {
    /// Serialize a stored template back to an envelope YAML document
    /// (DOCS §6 export). The builder "Save" and a git-committed YAML
    /// produce byte-identical stored definitions.
    pub fn to_envelope_yaml(&self) -> Result<String, SetupError> {
        // Render the flow body as a YAML value, dropping the injected
        // flow_id so export round-trips with the §6 nested form.
        let mut flow_value: serde_yaml::Value = serde_yaml::to_value(&self.flow_body)
            .map_err(|e| SetupError::InvalidYaml(e.to_string()))?;
        if let serde_yaml::Value::Mapping(map) = &mut flow_value {
            map.remove(serde_yaml::Value::String("flow_id".into()));
        }
        let envelope = TemplateEnvelope {
            id: self.id.0.clone(),
            version: self.version,
            display_name: self.display_name.clone(),
            description: self.description.clone(),
            icon: self.icon.clone(),
            category: self.category.clone(),
            input_schema: self.input_schema.clone(),
            input_bindings: self.input_bindings.clone(),
            output_bindings: self.output_bindings.clone(),
            access: self.access.clone(),
            flow: flow_value,
        };
        serde_yaml::to_string(&envelope).map_err(|e| SetupError::InvalidYaml(e.to_string()))
    }
}
