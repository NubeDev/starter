//! Per-method dispatch. Parses a frame, routes to the right
//! handler, returns a `Response` (or `None` for a notification).

use std::sync::Arc;

use crate::protocol::{Request, Response, RpcError};
use crate::registry::ToolRegistry;

/// Dispatch one frame. Returns `None` for valid notifications;
/// otherwise a response (success or error).
pub async fn dispatch(_registry: &Arc<ToolRegistry>, raw: &str) -> Option<Response> {
    let request: Request = match serde_json::from_str(raw) {
        Ok(req) => req,
        Err(e) => {
            return Some(Response::err(
                serde_json::Value::Null,
                RpcError::invalid_params(e.to_string()),
            ));
        }
    };

    let id = match request.id.clone() {
        Some(id) => id,
        None => return None, // notification
    };

    // TODO(ap): implement tools/list, tools/call dispatch against
    // the registry. Stub returns method_not_found for now so the
    // shape is locked.
    Some(Response::err(
        id,
        RpcError::method_not_found(&request.method),
    ))
}
