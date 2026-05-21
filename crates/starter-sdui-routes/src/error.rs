//! Wire-level error mapping for the three SDUI routes.
//!
//! Every DoS-limit violation surfaces as `413 Payload Too Large`
//! with a JSON body shaped:
//!
//! ```json
//! { "error": "<message>", "what": "page_state_bytes" }
//! ```
//!
//! The `what` field is the stable identifier — clients (and the
//! integration tests in `tests/limits_413.rs`) branch on it. The
//! `error` string is human-readable and may change; the `what` tag
//! is part of the wire contract.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

/// Stable `what:` tags surfaced on `413 Payload Too Large`.
///
/// Per R8 in DOCS/frontend/sdui/SCOPE.md, each row in the limits
/// table has one tag here. Tests pin every variant; the *limit
/// value* lives in [`crate::limits`] alongside the enforcement
/// helper.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WhatTag {
    /// `page_state` byte cap exceeded (R8: 64 KiB).
    PageStateBytes,
    /// Serialised render tree exceeded the cap (R8: 2 MiB).
    RenderTreeBytes,
    /// Tree node count exceeded the cap (R8: 2000).
    TreeNodes,
    /// Tree depth exceeded the cap (R8: 32).
    TreeDepth,
    /// Too many distinct component variant types in one tree (R8: 60).
    ComponentTypes,
    /// Action handler exceeded the per-fire timeout (R8: 5 s).
    HandlerTimeout,
    /// Table page size exceeded the cap (R8: 500 rows).
    TableRowsPerPage,
}

impl WhatTag {
    /// Stable wire string. Match the variant names; serde uses
    /// `snake_case` so the JSON value is e.g. `"page_state_bytes"`.
    pub fn as_str(self) -> &'static str {
        match self {
            WhatTag::PageStateBytes => "page_state_bytes",
            WhatTag::RenderTreeBytes => "render_tree_bytes",
            WhatTag::TreeNodes => "tree_nodes",
            WhatTag::TreeDepth => "tree_depth",
            WhatTag::ComponentTypes => "component_types",
            WhatTag::HandlerTimeout => "handler_timeout",
            WhatTag::TableRowsPerPage => "table_rows_per_page",
        }
    }
}

/// Errors the three SDUI routes can return.
#[derive(Debug, thiserror::Error)]
pub enum SduiError {
    /// A DoS limit was exceeded. Surfaces as 413 with a stable
    /// [`WhatTag`].
    #[error("payload too large: {}", what.as_str())]
    PayloadTooLarge {
        /// Stable identifier the client branches on.
        what: WhatTag,
        /// Human-readable detail, e.g. `"5120 bytes > 64 KiB cap"`.
        detail: String,
    },

    /// The named page could not be found by the host's
    /// [`crate::PageProvider`]. Surfaces as 404 with a
    /// `diagnostics`-shaped body so the client can render it
    /// inline.
    #[error("page not found: {page_ref}")]
    PageNotFound {
        /// The page reference the resolver was asked to fetch.
        page_ref: String,
    },

    /// The action's handler is not registered. Per R5 this returns
    /// 404 with a `diagnostics`-shaped body (`code:
    /// handler_not_found`) — the test in
    /// `tests/action_not_found.rs` pins both pieces.
    #[error("handler not found: {handler}")]
    HandlerNotFound {
        /// Handler name the action request asked for.
        handler: String,
    },

    /// The request body was malformed, the bindings failed to
    /// parse, or the query string was invalid. Surfaces as 400.
    #[error("bad request: {0}")]
    BadRequest(String),

    /// An internal error escaped the resolver — handler panic,
    /// graph-read failure, etc. Surfaces as 500.
    #[error("internal error: {0}")]
    Internal(String),
}

/// JSON body for a `413` response.
#[derive(Debug, Serialize)]
struct PayloadTooLargeBody<'a> {
    error: &'a str,
    /// Stable `what:` tag per R8.
    what: &'static str,
}

/// JSON body for a `404` on `/action` — shaped like
/// `ActionResponse::Diagnostics` so the client can render it in
/// place.
#[derive(Debug, Serialize)]
struct DiagnosticsBody<'a> {
    /// Always `"diagnostics"` — keeps the union discriminant
    /// consistent with the success path.
    #[serde(rename = "type")]
    ty: &'static str,
    items: Vec<DiagnosticItem<'a>>,
}

#[derive(Debug, Serialize)]
struct DiagnosticItem<'a> {
    severity: &'static str,
    code: &'static str,
    message: String,
    field: Option<&'a str>,
}

impl IntoResponse for SduiError {
    fn into_response(self) -> Response {
        match self {
            SduiError::PayloadTooLarge { what, detail } => {
                let body = PayloadTooLargeBody {
                    error: &detail,
                    what: what.as_str(),
                };
                (StatusCode::PAYLOAD_TOO_LARGE, Json(body)).into_response()
            }
            SduiError::PageNotFound { page_ref } => {
                let body = DiagnosticsBody {
                    ty: "diagnostics",
                    items: vec![DiagnosticItem {
                        severity: "error",
                        code: "page_not_found",
                        message: format!("page `{page_ref}` is not registered"),
                        field: None,
                    }],
                };
                (StatusCode::NOT_FOUND, Json(body)).into_response()
            }
            SduiError::HandlerNotFound { handler } => {
                let body = DiagnosticsBody {
                    ty: "diagnostics",
                    items: vec![DiagnosticItem {
                        severity: "error",
                        code: "handler_not_found",
                        message: format!("handler `{handler}` is not registered"),
                        field: None,
                    }],
                };
                (StatusCode::NOT_FOUND, Json(body)).into_response()
            }
            SduiError::BadRequest(detail) => {
                let body = serde_json::json!({ "error": detail });
                (StatusCode::BAD_REQUEST, Json(body)).into_response()
            }
            SduiError::Internal(detail) => {
                let body = serde_json::json!({ "error": detail });
                (StatusCode::INTERNAL_SERVER_ERROR, Json(body)).into_response()
            }
        }
    }
}
