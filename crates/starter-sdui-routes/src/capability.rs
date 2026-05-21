//! Client capability handshake — per SCOPE.md § R2 + R7.
//!
//! Two pieces:
//!
//! - **R2 — IR version handshake.** The client advertises the IR
//!   versions it understands. The server clamps emission to the
//!   highest mutually-supported version; the React renderer
//!   independently refuses to project a tree whose `ir_version`
//!   exceeds [`SUPPORTED_IR_VERSION`].
//! - **R7 — `custom` renderer-id filter.** The client advertises
//!   which `renderer_id` values its registry has implementations
//!   for. The server rewrites unknown ids to
//!   [`starter_ui_ir::Component::Dangling`] before emission so the
//!   page degrades gracefully.
//!
//! ## Threat model (R7)
//!
//! A malicious or curious client can lie about supported
//! renderer-ids — to **harvest schemas** by advertising every
//! plausible id and inspecting the returned `props`, or to **probe
//! for features** ("does this server know about `com.acme.secret`?"):
//!
//! 1. **`renderer_id` is treated as public.** Its appearance in a
//!    capability response is not a secret. If the *existence* of an
//!    id leaks deployment information (e.g. "this deployment has
//!    the internal-admin floorplan widget"), gate the **deployment**
//!    — never try to hide the id from a capability check.
//! 2. **`custom.props` are scoped to the renderer's contract, not
//!    the user's permissions.** A handler emitting `{ type:
//!    "custom", props: { sensitive } }` is responsible for ensuring
//!    those props are appropriate for the principal the resolve was
//!    issued against. The capability filter is a *vocabulary* check
//!    ("does this client know how to render this id"), not an
//!    *authorisation* check ("is this user allowed to see this
//!    data"). Conflating the two is a bug — auth runs at the handler
//!    boundary per R5 and at the resolve boundary, both before any
//!    `custom` node is ever constructed.

use serde::{Deserialize, Serialize};
use starter_ui_ir::Component;

/// The highest IR version this routes crate emits. Bump in lock-
/// step with [`starter_ui_ir::IR_VERSION`]; the renderer carries
/// its own copy in `@nube/starter-sdui-react`.
pub const SUPPORTED_IR_VERSION: u32 = starter_ui_ir::IR_VERSION;

/// What a client can declare about its capabilities — sent in the
/// request body for `/resolve` and `/action`, normally embedded in
/// the `context` block by the React provider.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClientCapabilities {
    /// IR versions this client can render. Empty list ≡ "trust
    /// the server", which is the same as advertising
    /// `[SUPPORTED_IR_VERSION]`.
    #[serde(default)]
    pub ir_versions: Vec<u32>,
    /// `renderer_id`s this client's registry has implementations
    /// for. Empty list ≡ "no custom renderers wired" — any
    /// `Component::Custom` in the resolved tree is rewritten to
    /// [`Component::Dangling`].
    #[serde(default)]
    pub custom_renderers: Vec<String>,
}

/// Filter applied to a resolved tree before emission. Constructed
/// from a [`ClientCapabilities`] block.
pub struct CapabilityFilter<'a> {
    caps: &'a ClientCapabilities,
}

impl<'a> CapabilityFilter<'a> {
    /// New filter scoped to one client's capabilities.
    pub fn new(caps: &'a ClientCapabilities) -> Self {
        Self { caps }
    }

    /// `true` when the server's emitted IR version is in the
    /// client's supported set. An empty list means "trust the
    /// server" — see [`ClientCapabilities::ir_versions`].
    pub fn accepts_ir_version(&self, version: u32) -> bool {
        self.caps.ir_versions.is_empty() || self.caps.ir_versions.contains(&version)
    }

    /// Walk `node` in place and rewrite any
    /// [`Component::Custom`] whose `renderer_id` is not in the
    /// client's advertised set to a [`Component::Dangling`] stub.
    /// The rest of the tree is unchanged.
    pub fn rewrite_unknown_custom(&self, node: &mut Component) {
        match node {
            Component::Custom { id, renderer_id, .. } => {
                if !self.caps.custom_renderers.is_empty()
                    && !self.caps.custom_renderers.iter().any(|s| s == renderer_id)
                {
                    let dangling_id = id.clone().unwrap_or_else(|| renderer_id.clone());
                    *node = Component::Dangling { id: dangling_id };
                }
            }
            Component::Page { children, .. }
            | Component::Row { children, .. }
            | Component::Col { children, .. }
            | Component::Grid { children, .. } => {
                for c in children {
                    self.rewrite_unknown_custom(c);
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use starter_ui_ir::ComponentTree;

    #[test]
    fn unknown_custom_rewrites_to_dangling() {
        let caps = ClientCapabilities {
            ir_versions: vec![SUPPORTED_IR_VERSION],
            custom_renderers: vec!["com.acme.known".into()],
        };
        let filter = CapabilityFilter::new(&caps);
        let mut tree = ComponentTree::new(Component::Page {
            id: "p".into(),
            title: None,
            header_actions: vec![],
            children: vec![Component::Custom {
                id: Some("c1".into()),
                renderer_id: "com.acme.unknown".into(),
                props: None,
                subscribe: vec![],
            }],
            style: None,
            default_row_gap: None,
            default_column_gap: None,
            default_page_padding: None,
            default_max_width: None,
        });
        filter.rewrite_unknown_custom(&mut tree.root);
        if let Component::Page { children, .. } = &tree.root {
            match &children[0] {
                Component::Dangling { id } => assert_eq!(id, "c1"),
                other => panic!("expected Dangling, got {other:?}"),
            }
        } else {
            panic!("page root");
        }
    }

    #[test]
    fn known_custom_passes_through() {
        let caps = ClientCapabilities {
            ir_versions: vec![],
            custom_renderers: vec!["com.acme.floorplan".into()],
        };
        let filter = CapabilityFilter::new(&caps);
        let mut node = Component::Custom {
            id: Some("fp".into()),
            renderer_id: "com.acme.floorplan".into(),
            props: None,
            subscribe: vec![],
        };
        filter.rewrite_unknown_custom(&mut node);
        assert!(matches!(node, Component::Custom { .. }));
    }

    #[test]
    fn empty_list_means_trust_server() {
        // No custom_renderers declared — treat the client as having
        // every id wired. R7: capability filter is vocabulary, not
        // auth; refusing here would be the wrong default.
        let caps = ClientCapabilities::default();
        let filter = CapabilityFilter::new(&caps);
        let mut node = Component::Custom {
            id: None,
            renderer_id: "anything".into(),
            props: None,
            subscribe: vec![],
        };
        filter.rewrite_unknown_custom(&mut node);
        assert!(matches!(node, Component::Custom { .. }));
    }

    #[test]
    fn ir_version_handshake() {
        let caps = ClientCapabilities {
            ir_versions: vec![4],
            custom_renderers: vec![],
        };
        let filter = CapabilityFilter::new(&caps);
        assert!(filter.accepts_ir_version(4));
        assert!(!filter.accepts_ir_version(5));
    }
}
