//! `POST /api/v1/ui/action` wire types — `ActionRequest`,
//! `ActionContext`, `ActionResponse`.
//!
//! Defined here (in the zero-I/O `starter-ui-ir` crate) so consumers
//! that want only the wire contract — CLI pretty-printers, schema
//! consumers (notably `starter-ai-builder-prompt`), the future
//! Flutter codegen — can deserialise / serialise responses without
//! pulling the HTTP server. The transport adapter lives in
//! `starter-sdui-routes`.
//!
//! ## Divergence D1 from Rubix
//!
//! Rubix's `ActionResponse::FormErrors` variant — a per-field error
//! map — is **not ported**. Starter ships the wider
//! [`ActionResponse::Diagnostics`] variant only, carrying
//! `Vec<Diagnostic>` where each item declares a severity, a stable
//! code, a server-translated message, and an optional field anchor.
//!
//! The wire is closed: a payload tagged `"type": "form_errors"`
//! fails to deserialise. There is no back-compat (starter has not
//! shipped). See `DOCS/frontend/sdui/DIVERGENCE.md` D1 for the
//! rationale and migration note.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::diagnostic::Diagnostic;
use crate::{Component, ComponentTree};

/// Inbound body of `POST /api/v1/ui/action`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ActionRequest {
    /// Handler name registered in the server's `HandlerRegistry`.
    pub handler: String,
    /// Handler-specific arguments — opaque to the IR.
    #[serde(default)]
    pub args: JsonValue,
    /// Page-level context that originated the action.
    #[serde(default)]
    pub context: ActionContext,
}

/// Per-fire client-supplied context. `target` is a component id for
/// SDUI button actions and a node path for kind-action handlers; the
/// discriminator is the handler shape.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ActionContext {
    /// Component id (SDUI flow) **or** node path (kind-action flow)
    /// that originated the action.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    /// Ordered nav-node ids forming the breadcrumb stack.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stack: Vec<String>,
    /// Page-local state at the moment the action fired. Capped
    /// server-side per R8 (64 KiB).
    #[serde(default, skip_serializing_if = "JsonValue::is_null")]
    pub page_state: JsonValue,
    /// Opaque auth subject identifier threaded through for audit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth_subject: Option<String>,
}

/// All possible action outcomes — the response of
/// `POST /api/v1/ui/action`.
///
/// `#[serde(tag = "type")]` produces the discriminated-union shape
/// the TS client's zod schemas expect.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ActionResponse {
    /// Replace a single subtree in the current render tree.
    Patch {
        target_component_id: String,
        tree: ComponentTree,
    },
    /// Replace the full page render tree.
    FullRender { tree: ComponentTree },
    /// Client-side navigation.
    Navigate { to: NavigateTo },
    /// Show a transient notification.
    Toast {
        intent: ToastIntent,
        message: String,
    },
    /// Structured server→client messages — errors, warnings, infos —
    /// either field-anchored or global. **Replaces Rubix's
    /// `FormErrors`** (divergence D1). See [`Diagnostic`].
    Diagnostics { items: Vec<Diagnostic> },
    /// Trigger a file download from the given URL.
    Download { url: String },
    /// Long-running response — client subscribes to the given
    /// channel.
    Stream { channel: String },
    /// Open a modal dialog whose content is the supplied subtree.
    Dialog { tree: Component },
    /// Show a toast and tell the renderer to refresh table / KPI
    /// caches. Shorthand for the most common mutation-success
    /// pattern.
    ToastAndRefresh {
        intent: ToastIntent,
        message: String,
    },
    /// No-op — action succeeded but the UI does not need to change.
    None,
}

/// Target of [`ActionResponse::Navigate`].
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NavigateTo {
    pub target_ref: String,
}

/// Intent (severity) for a [`ActionResponse::Toast`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ToastIntent {
    Ok,
    Warn,
    Danger,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::Severity;
    use serde_json::json;

    #[test]
    fn diagnostics_variant_serialises_with_type_tag() {
        let resp = ActionResponse::Diagnostics {
            items: vec![
                Diagnostic::new(Severity::Error, "project.code.too_short", "Too short.")
                    .with_field("projectCode"),
                Diagnostic::new(Severity::Warning, "task.due.past", "Past due."),
            ],
        };
        let v = serde_json::to_value(&resp).unwrap();
        assert_eq!(v["type"], "diagnostics");
        let items = v["items"].as_array().unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0]["severity"], "error");
        assert_eq!(items[0]["field"], "projectCode");
        assert!(items[1].get("field").is_none(), "global item omits `field`");
    }

    #[test]
    fn form_errors_tag_is_rejected_at_the_wire() {
        // D1: starter has not shipped, so the Rubix-era variant is
        // a parse error, not a deprecated-but-accepted shape.
        let raw = json!({
            "type": "form_errors",
            "errors": { "projectCode": "Too short" },
        });
        let err = serde_json::from_value::<ActionResponse>(raw).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("form_errors") || msg.contains("unknown variant"),
            "expected a tagged-enum mismatch, got: {msg}",
        );
    }

    #[test]
    fn navigate_round_trip() {
        let r = ActionResponse::Navigate {
            to: NavigateTo {
                target_ref: "/devices/1".into(),
            },
        };
        let s = serde_json::to_string(&r).unwrap();
        let _back: ActionResponse = serde_json::from_str(&s).unwrap();
        assert!(s.contains("\"type\":\"navigate\""));
    }

    #[test]
    fn toast_and_refresh_round_trip() {
        let r = ActionResponse::ToastAndRefresh {
            intent: ToastIntent::Ok,
            message: "Saved.".into(),
        };
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(v["type"], "toast_and_refresh");
        assert_eq!(v["intent"], "ok");
    }

    #[test]
    fn action_request_defaults() {
        let raw = json!({ "handler": "do.thing" });
        let req: ActionRequest = serde_json::from_value(raw).unwrap();
        assert_eq!(req.handler, "do.thing");
        assert!(req.context.target.is_none());
        assert!(req.context.stack.is_empty());
    }
}
