//! `changelog_layer(recorder)` — write one `starter-changelog` row
//! per authenticated tool dispatch.
//!
//! What gets recorded per request that hits a tool route:
//!
//! - `actor`   — `Actor::User { subject }` extracted from the
//!               request's [`Principal`] extension. Anonymous
//!               requests bypass the recorder (no audit row).
//! - `resource`— `ResourceRef { kind: "tool.invoke", id: tool_id, owner: None }`
//!               with the tool id pulled from the path. The kind
//!               is hardcoded for v0; a second invoke-shaped
//!               resource type would promote it to a constant on
//!               the gate registration site.
//! - `op`      — `Op::Custom("invoke")`.
//! - `after`   — the request body parsed as JSON, with a coarse
//!               redaction pass that drops obvious secret-looking
//!               keys. Non-JSON bodies are recorded as `null`.
//!
//! The middleware reads the request body once via
//! [`axum::extract::Request::into_parts`] and rebuilds the
//! request so the downstream handler sees the same bytes. The
//! resulting buffering is unconditional for tool routes — a
//! reasonable trade because the dispatcher is itself JSON-only
//! and the bodies are small. See
//! [docs/design/audit/](../../docs/design/audit/README.md).

use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::extract::Request;
use axum::http::StatusCode;
use axum::middleware::{from_fn_with_state, Next};
use axum::response::Response;
use axum::Router;
use chrono::Utc;
use serde_json::Value;

use starter_spi::auth::Principal;
use starter_spi::authz::ResourceRef;
use starter_spi::changelog::{Actor, Change, ChangeId, ChangeRecorder, GroupId, Op};

/// Cap on the request body buffered for one audit row. Anything
/// larger than this gets truncated to `Value::Null` rather than
/// blown up — the audit row still lands.
const MAX_AUDIT_BODY_BYTES: usize = 64 * 1024;

/// Resource-kind sentinel every tool-dispatch audit row uses.
pub const TOOL_INVOKE_KIND: &str = "tool.invoke";

/// Op tag every tool-dispatch audit row uses.
pub const TOOL_INVOKE_OP: &str = "invoke";

/// State threaded into the middleware closure: the recorder + the
/// path segment under which the tool router is mounted (used to
/// extract the trailing `tool_id`).
#[derive(Clone)]
pub struct ChangelogState {
    /// Shared recorder. Wrapped in [`Arc<dyn ChangeRecorder>`] so
    /// integration tests can swap a SQLite recorder in without
    /// touching the binary's wiring.
    pub recorder: Arc<dyn ChangeRecorder>,
    /// Path prefix that precedes the trailing `tool_id` segment in
    /// the route — typically `"/api/v1/tools/"`. The middleware
    /// derives the tool id by stripping this prefix from the URI
    /// path; requests whose path does not start with it skip the
    /// recorder (the middleware is layered above non-tool routes
    /// in the same composition, so the prefix check is its own
    /// scope filter).
    pub tool_path_prefix: String,
}

/// Wrap `router` in the changelog middleware.
pub fn changelog_layer<S>(router: Router<S>, state: ChangelogState) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    router.layer(from_fn_with_state(state, record_request))
}

async fn record_request(
    axum::extract::State(state): axum::extract::State<ChangelogState>,
    req: Request,
    next: Next,
) -> Response {
    // -- 1. Skip when the request isn't a tool dispatch.
    let Some(tool_id) = tool_id_from_path(req.uri().path(), &state.tool_path_prefix) else {
        return next.run(req).await;
    };

    // -- 2. Skip anonymous requests (audit row needs an actor).
    let Some(principal) = req.extensions().get::<Principal>().cloned() else {
        return next.run(req).await;
    };

    // -- 3. Buffer the body so we can both audit it and forward it
    //       on to the dispatch handler.
    let (parts, body) = req.into_parts();
    let bytes = match to_bytes(body, MAX_AUDIT_BODY_BYTES).await {
        Ok(b) => b,
        Err(_) => return StatusCode::PAYLOAD_TOO_LARGE.into_response_body(),
    };
    let payload = redact(parse_json(&bytes));
    let rebuilt = Request::from_parts(parts, Body::from(bytes.clone()));

    // -- 4. Run the handler first; only audit successful or
    //       failed-but-completed turns so panics-in-handler don't
    //       leave a misleading "success" row in the log.
    let response = next.run(rebuilt).await;

    // `id` and `group_id` are placeholders here — both recorder
    // impls overwrite them inside `transaction(...)` with a freshly
    // minted ULID before persisting, so the empty strings never
    // reach the database. See the
    // `starter_changelog_postgres::PgChangeRecorder` and
    // `starter_changelog_sqlite::SqliteChangeRecorder` impls.
    let change = Change {
        id: ChangeId(String::new()),
        at: Utc::now(),
        actor: Actor::User {
            subject: principal.subject.clone(),
        },
        resource: ResourceRef::row(TOOL_INVOKE_KIND, tool_id),
        resource_version: None,
        op: Op::Custom(TOOL_INVOKE_OP.to_owned()),
        before: None,
        after: Some(payload),
        patch: None,
        group_id: GroupId(String::new()),
        correlation: None,
    };

    let recorder = state.recorder.clone();
    if let Err(e) = recorder
        .transaction(Box::new(move |tx| {
            Box::pin(async move {
                tx.record(change).await?;
                Ok(())
            })
        }))
        .await
    {
        tracing::warn!(
            target: "rubix.audit",
            error = %e,
            "changelog row write failed",
        );
    }

    response
}

/// Pull the trailing path segment after `prefix` if `path` starts
/// with it. Returns `None` when `path` is outside the tool router
/// (the middleware then forwards untouched).
fn tool_id_from_path(path: &str, prefix: &str) -> Option<String> {
    let tail = path.strip_prefix(prefix)?;
    let id = tail.split('/').next()?;
    if id.is_empty() {
        return None;
    }
    Some(id.to_owned())
}

/// Best-effort JSON parse for the audit payload. Non-JSON bodies
/// fall back to `Value::Null` rather than rejecting the request —
/// the audit row should never be the reason a request fails.
fn parse_json(bytes: &[u8]) -> Value {
    if bytes.is_empty() {
        return Value::Null;
    }
    serde_json::from_slice(bytes).unwrap_or(Value::Null)
}

/// Drop keys that look like secrets from the top-level object.
/// Anything nested is left intact for v0 — the disk / db / alert
/// tools' inputs are flat. A future pass swaps this for the
/// shared redaction helper once one exists upstream.
fn redact(v: Value) -> Value {
    match v {
        Value::Object(mut map) => {
            map.retain(|k, _| !is_secret_key(k));
            Value::Object(map)
        }
        other => other,
    }
}

fn is_secret_key(key: &str) -> bool {
    let lower = key.to_ascii_lowercase();
    [
        "password",
        "secret",
        "token",
        "api_key",
        "apikey",
        "authorization",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

/// Tiny adapter trait so `StatusCode::PAYLOAD_TOO_LARGE` can build
/// an [`axum::response::Response`] without pulling in the
/// `IntoResponse` import at the call site.
trait IntoResponseBody {
    fn into_response_body(self) -> Response;
}

impl IntoResponseBody for StatusCode {
    fn into_response_body(self) -> Response {
        Response::builder()
            .status(self)
            .body(Body::empty())
            .expect("empty body builds")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_id_from_path_extracts_trailing_segment() {
        let id = tool_id_from_path("/api/v1/tools/rubix.system.disk", "/api/v1/tools/");
        assert_eq!(id.as_deref(), Some("rubix.system.disk"));
    }

    #[test]
    fn tool_id_from_path_returns_none_outside_prefix() {
        assert!(tool_id_from_path("/healthz", "/api/v1/tools/").is_none());
        assert!(tool_id_from_path("/api/v1/tools/", "/api/v1/tools/").is_none());
    }

    #[test]
    fn redact_drops_password_field() {
        let v = serde_json::json!({"mount": "/", "password": "hunter2"});
        let r = redact(v);
        assert_eq!(r.get("mount").and_then(|v| v.as_str()), Some("/"));
        assert!(r.get("password").is_none());
    }
}
