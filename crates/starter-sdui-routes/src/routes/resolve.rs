//! `POST /api/v1/ui/resolve`.
//!
//! Body:
//!
//! ```json
//! {
//!   "page_ref": "...",
//!   "target_ref": "...",
//!   "stack": { "alias": "entity-id" },
//!   "page_state": { ... },
//!   "capabilities": { "ir_versions": [...], "custom_renderers": [...] }
//! }
//! ```
//!
//! Response:
//!
//! ```json
//! {
//!   "render": <ComponentTree>,
//!   "subscriptions": [{ "entity_id": "...", "slot": "..." }, ...]
//! }
//! ```
//!
//! Order of operations:
//!
//! 1. Enforce R8 `page_state` byte cap.
//! 2. Look the page up via [`crate::PageProvider`].
//! 3. Enforce R8 tree-shape caps on the unresolved layout.
//! 4. Substitute bindings against [`crate::SduiState::graph`].
//! 5. Apply the capability filter (R7) — unknown `renderer_id`s
//!    become `Dangling`.
//! 6. Enforce R8 serialised-bytes cap on the final tree.

use std::cell::RefCell;
use std::collections::HashMap;

use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use starter_ui_bindings::{
    substitute_tree, EntityGraph, EntityId, EvalContext, SlotAccess, Subject, SubscriptionPlan,
};
use starter_ui_ir::ComponentTree;

use crate::capability::{CapabilityFilter, ClientCapabilities};
use crate::error::SduiError;
use crate::limits;
use crate::state::SduiState;

/// Request body for `/resolve`.
#[derive(Debug, Clone, Deserialize)]
pub struct ResolveRequest {
    /// Page reference the [`crate::PageProvider`] knows how to
    /// look up.
    pub page_ref: String,
    /// Optional target the page is resolved against. `None`
    /// renders the page in target-less mode; bindings that touch
    /// `$target` then error.
    #[serde(default)]
    pub target_ref: Option<String>,
    /// Named stack frames for `$stack.alias` bindings.
    #[serde(default)]
    pub stack: HashMap<String, EntityId>,
    /// In-flight page state for `$page.field` bindings. Capped
    /// per R8.
    #[serde(default)]
    pub page_state: JsonValue,
    /// Principal claims for `$user.field` bindings. Normally
    /// populated server-side from the auth layer; accepted on the
    /// wire for tests / dev tooling.
    #[serde(default)]
    pub user: serde_json::Map<String, JsonValue>,
    /// Client capability handshake. Empty defaults pass everything
    /// through (R7: "trust the server").
    #[serde(default)]
    pub capabilities: ClientCapabilities,
}

/// Response body for `/resolve`.
#[derive(Debug, Clone, Serialize)]
pub struct ResolveResponse {
    /// The resolved tree with bindings substituted.
    pub render: ComponentTree,
    /// Per-resolve subscription plan — one [`Subject`] per
    /// `(entity_id, slot)` pair the resolver touched.
    pub subscriptions: Vec<Subject>,
}

/// Axum handler.
pub async fn handler(
    State(state): State<SduiState>,
    Json(req): Json<ResolveRequest>,
) -> Result<Json<ResolveResponse>, SduiError> {
    // 1. page_state byte cap (R8).
    limits::enforce_page_state_bytes(&req.page_state)?;

    // 2. Page lookup.
    let mut tree = state
        .pages
        .lookup_page(&req.page_ref)
        .await
        .ok_or_else(|| SduiError::PageNotFound {
            page_ref: req.page_ref.clone(),
        })?;

    // 3. Tree-shape caps on the unresolved layout — checking here
    // also bounds the binding-substitution walk that follows.
    limits::enforce_tree_shape(&tree)?;

    // 4. Binding substitution. Scoped so the non-Send `EvalContext`
    // (it holds `&RefCell<SlotAccess>` and a `&dyn MessageBag`) is
    // dropped before the next `.await` — keeping the handler future
    // Send for axum.
    let log: RefCell<Vec<SlotAccess>> = RefCell::new(Vec::new());
    let page_obj = req.page_state.as_object().cloned().unwrap_or_default();
    {
        let graph_ref: &(dyn EntityGraph + Send + Sync) = &*state.graph;
        let ctx = EvalContext {
            graph: graph_ref,
            target: req.target_ref.as_deref(),
            self_id: None,
            stack: &req.stack,
            user: &req.user,
            page: &page_obj,
            access_log: Some(&log),
            item: None,
            index: None,
            catalogue: &starter_ui_bindings::NullBag,
            locale: "en",
        };
        substitute_tree(&mut tree, &ctx)
            .map_err(|e| SduiError::BadRequest(format!("binding substitution failed: {e}")))?;
    }

    // 4b. Chart / KPI source resolver — turn `ChartSource::Static`
    // and `ChartSource::AnalyticsTemplate` into the server-emitted
    // `series` / `value` payloads the client renders verbatim.
    crate::chart_resolve::resolve_chart_sources(
        &mut tree,
        state.analytics.as_deref(),
    )
    .await;

    // 5. Capability filter (R7). Run after substitution so any
    // `custom` nodes synthesised by the binding pass are also
    // filtered.
    let filter = CapabilityFilter::new(&req.capabilities);
    filter.rewrite_unknown_custom(&mut tree.root);

    // R2: refuse to emit a tree whose ir_version the client cannot
    // render. Falls back to a structured bad-request — the client
    // is expected to handshake before issuing /resolve, so this is
    // a defensive check.
    if !filter.accepts_ir_version(tree.ir_version) {
        return Err(SduiError::BadRequest(format!(
            "client does not accept ir_version {} (advertised {:?})",
            tree.ir_version, req.capabilities.ir_versions,
        )));
    }

    // 6. Serialised-bytes cap (R8).
    let bytes = serde_json::to_vec(&tree)
        .map_err(|e| SduiError::Internal(format!("serialise tree: {e}")))?;
    limits::enforce_render_tree_bytes(bytes.len())?;

    let plan = SubscriptionPlan::from_log(log.into_inner());
    Ok(Json(ResolveResponse {
        render: tree,
        subscriptions: plan.subjects,
    }))
}
