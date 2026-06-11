//! Materialise an extension's contributed flow node-kinds into the host's
//! [`NodeKindRegistry`] (FLOW-NODES slice B).
//!
//! An extension's `block.yaml` `contributes.nodes[]` declares flow node-kinds
//! (e.g. `com.acme.devices.device_create`). The host does NOT execute their
//! bodies — it bridges each kind to the extension's supervised child over
//! `flow.node.invoke` via a [`ProcessNodeProxy`]. This keeps the host generic:
//! it hardcodes no extension; any process-flavour bundle that contributes nodes
//! gets them wired the same way, and the node runs in the child where the
//! author's code lives.
//!
//! This is the missing seam the Setup/Automation Builder needed: a setup
//! template references contributed node-kinds, and without this bridge those
//! kinds resolve to nothing at run time. The setup `RunService`'s engine shares
//! the very registry this function populates.

use std::sync::Arc;

use starter_ext_flow::ProcessNodeProxy;
use starter_ext_spi::Manifest;
use starter_ext_supervisor::SupervisorHandle;
use starter_flow::registry::NodeKindRegistry;
use starter_flow_spi::node::KindId;

/// Register every `contributes.nodes[]` kind from `manifest` into `registry`,
/// each bridged to `supervisor` (the extension's running child) by a
/// [`ProcessNodeProxy`]. The proxy forwards `NodeBehavior::invoke` to the
/// child's `flow.node.invoke` handler.
///
/// Best-effort per kind: a node whose id is not a syntactically valid
/// [`KindId`], or whose registration is refused (reserved namespace / duplicate
/// id — the namespace validator should already have caught a non-descendant id
/// at manifest validation), is logged and skipped so one bad node never blocks
/// the rest of the extension or the host boot. Returns the count registered.
pub async fn register_contributed_nodes(
    manifest: &Manifest,
    supervisor: &SupervisorHandle,
    registry: &Arc<NodeKindRegistry>,
) -> usize {
    let ext_id = manifest.id.as_str();
    let mut registered = 0usize;
    for node in &manifest.contributes.nodes {
        let kind = match KindId::new(node.kind.clone()) {
            Ok(k) => k,
            Err(e) => {
                tracing::warn!(
                    target: "nexus_api::extensions::nodes",
                    extension = %ext_id,
                    kind = %node.kind,
                    error = %e,
                    "skipping contributed node-kind with an invalid id"
                );
                continue;
            }
        };
        let proxy = ProcessNodeProxy::new(kind, supervisor.clone(), node.streaming);
        match registry.register(Arc::new(proxy)).await {
            Ok(()) => {
                registered += 1;
                tracing::debug!(
                    target: "nexus_api::extensions::nodes",
                    extension = %ext_id,
                    kind = %node.kind,
                    streaming = node.streaming,
                    "bridged contributed node-kind to its extension child"
                );
            }
            Err(e) => {
                tracing::warn!(
                    target: "nexus_api::extensions::nodes",
                    extension = %ext_id,
                    kind = %node.kind,
                    error = %e,
                    "skipping contributed node-kind the registry refused"
                );
            }
        }
    }
    registered
}
