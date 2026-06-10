//! `GET /api/v1/flows/node-types` — the flow-builder palette.
//!
//! LAYER: transport (REST). Extract → call domain → shape DTO → return.
//! No SQL, no business predicates, no cross-resource walks here.
//! See docs/design/layering/.

use axum::Json;
use nexus_spi::dto::flow::{NodeCategory, NodeType, NodeTypeList};

/// Map the engine's category to the wire discriminant.
fn category(c: nexus_engine::NodeCategory) -> NodeCategory {
    match c {
        nexus_engine::NodeCategory::Input => NodeCategory::Input,
        nexus_engine::NodeCategory::Processor => NodeCategory::Processor,
        nexus_engine::NodeCategory::Output => NodeCategory::Output,
    }
}

/// Return every registered node, described. A static catalogue sourced from the
/// engine registry's self-description — no datasource work, no per-tenant state.
#[utoipa::path(
    get,
    path = "/api/v1/flows/node-types",
    tag = "flows",
    operation_id = "list_node_types",
    responses((status = 200, description = "Registered flow node types", body = NodeTypeList)),
)]
pub async fn list_node_types() -> Json<NodeTypeList> {
    let node_types = nexus_engine::describe()
        .into_iter()
        .map(|d| NodeType {
            kind: d.kind.to_string(),
            category: category(d.category),
            label: d.label.to_string(),
            description: d.description.to_string(),
            config_schema: d.config_schema,
        })
        .collect();
    Json(NodeTypeList { node_types })
}
