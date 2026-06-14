//! YAML import/export + binding validation (P0, DOCS §6/§9).
//!
//! Import path (DOCS §6 — **not** `parse_flow_file`):
//! `serde_yaml` → [`TemplateEnvelope`] → nested `flow` → `FlowBody` →
//! node-kind validation via the flow layer's body-level resolver. This
//! module owns the envelope→template step plus the §9 rule that template
//! `input_bindings` may never target a reserved trusted-identity slot.

use std::sync::Arc;

use starter_flow::definition::resolver::TopologyResolver;
use starter_flow::registry::NodeKindRegistry;
use starter_flow_spi::flow::FlowId;
use starter_setup_spi::envelope::TemplateEnvelope;
use starter_setup_spi::error::{SetupError, SetupResult};
use starter_setup_spi::model::{Template, TemplateSource};
use starter_setup_spi::reserved;

/// Split a `node.slot` reference into `(node, slot)` on the **last** dot
/// (node ids are reverse-DNS, so the node/slot separator is the final
/// dot — mirrors the resolver's `parse_endpoint`).
pub fn slot_node(reference: &str) -> Option<(&str, &str)> {
    reference.rsplit_once('.')
}

/// Validate a template's bindings (DOCS §9): no `input_binding` may
/// target a reserved trusted-identity slot, since those are seeded from
/// the verified `Principal` and must never be settable from client form
/// input. Also rejects structurally malformed `node.slot` references.
pub fn validate_bindings(template: &Template) -> SetupResult<()> {
    for b in &template.input_bindings {
        let (_, slot) = slot_node(&b.slot).ok_or_else(|| {
            SetupError::InvalidBinding(format!("malformed slot reference: {}", b.slot))
        })?;
        if reserved::is_reserved(slot) {
            return Err(SetupError::InvalidBinding(format!(
                "input_binding for field '{}' targets reserved trusted-identity slot '{}' \
                 (identity is host-bound, never client-supplied — DOCS §9)",
                b.field, slot
            )));
        }
    }
    for b in &template.output_bindings {
        if slot_node(&b.slot).is_none() {
            return Err(SetupError::InvalidBinding(format!(
                "malformed output slot reference: {}",
                b.slot
            )));
        }
    }
    Ok(())
}

/// Validate the template's flow body against the registered node kinds,
/// reusing the flow layer's body-level resolver (DOCS §6 — the same
/// validation `flow-watch` uses, applied at the body level rather than
/// the file level). Does **not** persist anything.
pub async fn validate_flow_body(
    template: &Template,
    kinds: &NodeKindRegistry,
) -> SetupResult<()> {
    let flow_id = FlowId::new(template.id.0.clone())
        .map_err(|e| SetupError::InvalidBody(format!("template id is not a valid flow id: {e}")))?;
    TopologyResolver::resolve_body(&template.flow_body, &flow_id, kinds)
        .await
        .map(|_| ())
        .map_err(|e| SetupError::InvalidBody(e.to_string()))
}

/// Import a raw YAML envelope into a fully-validated [`Template`]
/// (DOCS §6): parse → nest the flow body → validate node kinds → validate
/// bindings. `tenant_id = None` imports into the `__global__` extension
/// catalog. The result is ready for `TemplateStore::put`.
pub async fn import_template_yaml(
    yaml: &str,
    tenant_id: Option<String>,
    source: TemplateSource,
    kinds: &Arc<NodeKindRegistry>,
) -> SetupResult<Template> {
    let envelope = TemplateEnvelope::from_yaml(yaml)?;
    let template = envelope.into_template(tenant_id, source)?;
    validate_flow_body(&template, kinds).await?;
    validate_bindings(&template)?;
    Ok(template)
}

/// Export a stored template back to envelope YAML (DOCS §6).
pub fn export_template_yaml(template: &Template) -> SetupResult<String> {
    template.to_envelope_yaml()
}
