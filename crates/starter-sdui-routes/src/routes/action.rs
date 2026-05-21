//! `POST /api/v1/ui/action`.
//!
//! Body:
//!
//! ```json
//! {
//!   "handler": "device.restart",
//!   "args":    { ... },
//!   "context": { "target": "...", "stack": [...], "page_state": {...} }
//! }
//! ```
//!
//! Response: the discriminated [`starter_ui_ir::ActionResponse`]
//! union per R5 — `patch | full_render | navigate | toast |
//! diagnostics | download | stream | none` (plus the two starter
//! shorthands `dialog` / `toast_and_refresh`).
//!
//! Per R5, an unregistered `handler` returns `404` with a body
//! shaped like `ActionResponse::Diagnostics` so the client can
//! render the error in place without parsing two response shapes.
//! The integration test in `tests/action_not_found.rs` pins the
//! status code, the `code: "handler_not_found"` field, and the
//! discriminator.

use axum::extract::State;
use axum::response::{IntoResponse, Response};
use axum::Json;
use starter_ui_ir::{ActionRequest, ActionResponse};

use crate::error::SduiError;
use crate::handler::{HandlerContext, Principal};
use crate::limits;
use crate::state::SduiState;

/// Re-export of the wire body so consumers wiring custom
/// dispatchers can deserialise the same shape.
pub type ActionBody = ActionRequest;

/// Axum handler.
pub async fn handler(
    State(state): State<SduiState>,
    Json(req): Json<ActionBody>,
) -> Response {
    // R8: page_state byte cap also applies to action requests —
    // the context block carries page_state, and a 64KiB cap
    // upstream of dispatch keeps the limit consistent with
    // /resolve.
    if let Err(e) = limits::enforce_page_state_bytes(&req.context.page_state) {
        return e.into_response();
    }

    let principal = Principal {
        subject: req.context.auth_subject.clone().unwrap_or_default(),
        ..Principal::default()
    };
    let ctx = HandlerContext {
        principal,
        name: req.handler.clone(),
        args: req.args.clone(),
        context: req.context.clone(),
    };

    match state.handlers.dispatch(ctx).await {
        Ok(Ok(response)) => Json(response).into_response(),
        Ok(Err(err)) => err.into_response(),
        Err(missing) => SduiError::HandlerNotFound {
            handler: missing.handler,
        }
        .into_response(),
    }
}

/// Convenience: build an `ActionResponse::Diagnostics` carrying a
/// single error item. Useful in handler closures that need to
/// short-circuit with a structured message.
pub fn diagnostics_error(code: &str, message: impl Into<String>) -> ActionResponse {
    ActionResponse::Diagnostics {
        items: vec![starter_ui_ir::Diagnostic::new(
            starter_ui_ir::Severity::Error,
            code,
            message,
        )],
    }
}
