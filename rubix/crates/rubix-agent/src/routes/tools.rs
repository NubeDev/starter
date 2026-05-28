//! `POST /api/v1/tools/{tool_id}` — REST dispatcher for the
//! tools the rubix-agent advertises.
//!
//! LAYER: transport (REST). Extract → call domain → shape DTO → return.
//! The handler is deliberately tiny: it must not grow domain logic.
//! Anything beyond extract / dispatch / shape belongs in
//! [`rubix_tools`]. See
//! [docs/design/tools/](../../../docs/design/tools/README.md) and
//! [docs/design/i18n-prefs/](../../../docs/design/i18n-prefs/README.md)
//! for the Diagnostic-rendering posture.
//!
//! Locale handling: an [`AcceptLanguageLayer`] (from
//! `starter-i18n`) populates a [`LocaleCtx`] request extension.
//! When the caller passes `?render=server` the handler walks the
//! tool's `summary` field through
//! [`MessageBundle::render_diagnostic`] and stashes the rendered
//! string alongside the raw Diagnostic; without the query the wire
//! shape is the raw Diagnostic JSON (REST clients render
//! client-side — the documented default for non-MCP transports).
//!
//! The same `probe()` the CLI sibling consumes in-process is
//! reached here through `Tool::invoke` on the registered tool, so
//! both surfaces share one code path.

use std::collections::HashMap;
use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Extension, Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use starter_ext_spi::identity::CallerIdentity;
use starter_i18n::bundle::MessageBundle;
use starter_i18n::middleware::{accept_language_layer, LocaleCtx};
use starter_spi::auth::Principal;
use starter_spi::error::Error;
use starter_spi::i18n::{Diagnostic, LanguageTag};
use starter_spi::tool::Tool;

use crate::boot::mcp::prefs_from_locale;

/// State threaded through the tools router: a name → tool lookup
/// plus the bundle the renderer reads catalogues from.
#[derive(Clone)]
pub struct ToolsState {
    tools: Arc<HashMap<String, Arc<dyn Tool>>>,
    bundle: Arc<MessageBundle>,
}

impl ToolsState {
    /// Build a [`ToolsState`] from the boot-time tool registry. The
    /// `definition().name` of each tool is its dispatch id at
    /// `/api/v1/tools/{id}`.
    pub fn new(tools: Vec<Arc<dyn Tool>>, bundle: Arc<MessageBundle>) -> Self {
        let map: HashMap<String, Arc<dyn Tool>> = tools
            .into_iter()
            .map(|t| (t.definition().name, t))
            .collect();
        Self {
            tools: Arc::new(map),
            bundle,
        }
    }
}

/// `?render=` query string. The only currently-accepted value is
/// `"server"`; anything else (including absent) keeps the default
/// off-the-wire raw-Diagnostic shape.
#[derive(Debug, Default, Deserialize)]
pub struct RenderQuery {
    #[serde(default)]
    render: Option<String>,
}

/// Build the tools registrar. Mounts `POST /api/v1/tools/{tool_id}`
/// and wraps it with [`accept_language_layer`] so the handler can
/// read the negotiated language from the [`LocaleCtx`] extension.
pub fn registrar(state: ToolsState) -> crate::routes::RouteRegistrar {
    use crate::routes::{RouteMeta, RouteRegistrar};
    use axum::http::Method;
    let layer = accept_language_layer(state.bundle.clone());
    RouteRegistrar::new()
        .mount(
            Method::POST,
            "/api/v1/tools/{tool_id}",
            post(dispatch).with_state(state),
            RouteMeta::new()
                .describe("Dispatch a registered tool by id with a JSON body.")
                .tag("system"),
        )
        .map_router(|r| r.layer(layer))
}

/// Backwards-compatible alias for tests / existing call sites.
pub fn router(state: ToolsState) -> Router {
    registrar(state).into_router()
}

/// Handler — kept at ≤20 lines. Any growth here is a smell:
/// domain logic belongs in `rubix-tools` (push into `probe()`),
/// shaping logic belongs in [`shape_response`].
#[utoipa::path(
    post,
    path = "/api/v1/tools/{tool_id}",
    tag = "system",
    params(
        ("tool_id" = String, Path, description = "Registered tool id (e.g. `rubix.system.disk`, `rubix.user.create`, `rubix.flow.deploy`, `rubix.undo.last`)."),
        ("render" = Option<String>, Query, description = "Pass `server` to ask the agent to render the `summary` Diagnostic against the negotiated locale and return it as `rendered_summary` alongside the raw structured form."),
    ),
    responses(
        (status = 200, description = "Tool invocation succeeded; body is the tool's structured response."),
        (status = 400, description = "Invalid tool input."),
        (status = 401, description = "Unauthenticated (cookie session or API token required when the agent boots with `RUBIX_DATABASE_URL`)."),
        (status = 403, description = "Forbidden by the authz engine."),
        (status = 404, description = "Unknown tool id."),
        (status = 409, description = "Conflict — invariant violated."),
        (status = 500, description = "Tool execution failed."),
    ),
)]
pub(crate) async fn dispatch(
    State(state): State<ToolsState>,
    Path(tool_id): Path<String>,
    Query(q): Query<RenderQuery>,
    Extension(locale): Extension<LocaleCtx>,
    principal: Option<Extension<Principal>>,
    Json(input): Json<Value>,
) -> Response {
    let Some(tool) = state.tools.get(&tool_id).cloned() else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "unknown tool", "tool_id": tool_id})),
        )
            .into_response();
    };
    let result = match principal.map(|Extension(p)| p) {
        Some(p) => {
            let caller = CallerIdentity {
                tenant_id: p.tenant_id.clone(),
                user_id: Some(p.subject.clone()),
                roles: vec![format!("{:?}", p.role)],
                request_id: String::new(),
            };
            // Bind two task-locals for the dispatch:
            //   - `caller_local`: extension-backed `Tool` impls
            //     (`ProcessExtensionToolBinding`) upgrade to
            //     `SupervisorHandle::call_as` instead of an
            //     unscoped `call`.
            //   - `actor_local`: the `UndoDispatcher` wrappers
            //     read this to stamp the `actor` field on the
            //     reversible changelog row. `LocalActor` (in
            //     `rubix_tools::undo::dispatch`) reads the same
            //     task-local so an unwrapped tool dispatched
            //     directly (no undo) still attributes correctly.
            // Native rubix tools that need neither ignore both.
            let actor = starter_spi::changelog::Actor::User {
                subject: p.subject.clone(),
            };
            starter_ext_supervisor::caller_local::scope(
                caller,
                starter_undo::actor_local::scope(actor, tool.invoke(input)),
            )
            .await
        }
        None => tool.invoke(input).await,
    };
    shape_response(
        result,
        &state.bundle,
        locale.language().clone(),
        q.render.as_deref(),
    )
}

/// Map a tool result (`Result<Value, Error>`) onto a REST
/// response. Status comes from the [`Error`] variant; body is JSON
/// in every case. When `render == Some("server")` and the success
/// payload carries a `summary` [`Diagnostic`], the diagnostic is
/// rendered against the caller's locale and stashed as
/// `rendered_summary` alongside the raw structured form.
fn shape_response(
    result: Result<Value, Error>,
    bundle: &MessageBundle,
    language: LanguageTag,
    render: Option<&str>,
) -> Response {
    match result {
        Ok(mut body) => {
            if render == Some("server") {
                let lang = language;
                if let Some(rendered) = render_summary(bundle, &lang, &body) {
                    if let Value::Object(map) = &mut body {
                        map.insert("rendered_summary".into(), Value::String(rendered));
                    }
                }
            }
            (StatusCode::OK, Json(body)).into_response()
        }
        Err(e) => {
            // The `Unavailable` arm is special-cased before the
            // generic mapper so we can build a structured 503 body
            // (with a restart hint when the subject is an extension
            // id). All other variants fall through to the status
            // table below.
            if let Error::Unavailable {
                code,
                subject,
                message,
            } = &e
            {
                return unavailable_response(code, subject.as_deref(), message);
            }
            let status = match &e {
                Error::NotFound { .. } => StatusCode::NOT_FOUND,
                Error::Invalid { .. } => StatusCode::BAD_REQUEST,
                Error::Unauthenticated => StatusCode::UNAUTHORIZED,
                Error::Forbidden => StatusCode::FORBIDDEN,
                Error::Conflict { .. } => StatusCode::CONFLICT,
                Error::Internal { .. } => StatusCode::INTERNAL_SERVER_ERROR,
                // `Error` is `#[non_exhaustive]`; new variants
                // default to "internal" so the handler keeps
                // compiling. Add an explicit arm when one matters.
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            (status, Json(json!({"error": e.to_string()}))).into_response()
        }
    }
}

/// Build the structured 503 body returned for [`Error::Unavailable`].
///
/// When `code` matches the well-known supervisor sentinel and the
/// subject is non-empty, the body carries a `restart` hint pointing
/// at the matching admin endpoint so a UI can render an actionable
/// "Restart extension" affordance without parsing the message text.
/// Other unavailable kinds (no subject, unknown code) get the same
/// status but no hint — the wire stays additive.
fn unavailable_response(code: &str, subject: Option<&str>, message: &str) -> Response {
    let mut body = json!({
        "error":   message,
        "code":    code,
        "subject": subject,
    });
    if code == "extension.supervisor_unavailable" {
        if let Some(id) = subject.filter(|s| !s.is_empty()) {
            body["restart"] = json!({
                "method": "POST",
                "path":   format!("/extensions/{id}/restart"),
            });
        }
    }
    (StatusCode::SERVICE_UNAVAILABLE, Json(body)).into_response()
}

/// Pull `summary` from a tool result body, deserialise as
/// [`Diagnostic`], and render through
/// [`MessageBundle::render_diagnostic`] against the locale's
/// resolved preferences. Returns `None` if the body has no
/// `summary` field or it isn't a Diagnostic — the caller falls
/// through to the raw-JSON path.
fn render_summary(bundle: &MessageBundle, lang: &LanguageTag, body: &Value) -> Option<String> {
    let summary = body.get("summary")?.clone();
    let diag: Diagnostic = serde_json::from_value(summary).ok()?;
    let prefs = prefs_from_locale(lang);
    Some(bundle.render_diagnostic(lang, &diag, &prefs))
}

#[cfg(test)]
mod tests {
    use super::*;
    use starter_spi::i18n::MessageKey;

    fn bundle() -> Arc<MessageBundle> {
        Arc::new(rubix_spi::i18n::rubix_bundle().expect("rubix bundle parses"))
    }

    #[test]
    fn shape_response_maps_invalid_to_400() {
        let err = Error::Invalid {
            message: "bad mount".into(),
        };
        let resp = shape_response(Err(err), &bundle(), en(), None);
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn shape_response_maps_internal_to_500() {
        let boxed: Box<dyn std::error::Error + Send + Sync> = "boom".into();
        let err = Error::Internal { source: boxed };
        let resp = shape_response(Err(err), &bundle(), en(), None);
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn shape_response_maps_unavailable_to_503_with_restart_hint() {
        // Simulates the wire shape rubix-agent's tools router emits
        // after `ProcessExtensionToolBinding` translates a supervisor-
        // death `Transport` error. The frontend pattern-matches the
        // `code` field and uses `restart.path` to wire a single-click
        // recovery button — the wire shape is the contract.
        let err = Error::unavailable_subject(
            "extension.supervisor_unavailable",
            "com.rubix.example",
            "supervisor task is no longer running",
        );
        let resp = shape_response(Err(err), &bundle(), en(), None);
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
        let body = axum::body::to_bytes(resp.into_body(), 64 * 1024)
            .await
            .expect("body bytes");
        let v: Value = serde_json::from_slice(&body).expect("body json");
        assert_eq!(v["code"], "extension.supervisor_unavailable");
        assert_eq!(v["subject"], "com.rubix.example");
        assert_eq!(v["error"], "supervisor task is no longer running");
        assert_eq!(v["restart"]["method"], "POST");
        assert_eq!(
            v["restart"]["path"],
            "/extensions/com.rubix.example/restart",
        );
    }

    #[tokio::test]
    async fn shape_response_unavailable_without_subject_omits_restart_hint() {
        // Unavailable errors with no subject (or unknown code) still
        // return 503 but skip the restart hint — the caller has no
        // id to act on.
        let err = Error::unavailable("warehouse.unavailable", "connection lost");
        let resp = shape_response(Err(err), &bundle(), en(), None);
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
        let body = axum::body::to_bytes(resp.into_body(), 64 * 1024)
            .await
            .expect("body bytes");
        let v: Value = serde_json::from_slice(&body).expect("body json");
        assert_eq!(v["code"], "warehouse.unavailable");
        assert!(v.get("restart").is_none(), "no restart hint: {v}");
    }

    fn en() -> LanguageTag {
        LanguageTag::parse("en").expect("'en' parses")
    }

    #[test]
    fn render_summary_uses_caller_language() {
        let diag = Diagnostic::new(
            MessageKey::parse("rubix.system.disk.warn").expect("hard-coded key parses"),
        );
        let body = json!({"summary": diag});
        let en = render_summary(
            &bundle(),
            &LanguageTag::parse("en").expect("'en' parses"),
            &body,
        )
        .expect("renders");
        let es = render_summary(
            &bundle(),
            &LanguageTag::parse("es").expect("'es' parses"),
            &body,
        )
        .expect("renders");
        assert!(en.starts_with("Disk"), "EN got {en}");
        assert!(es.starts_with("El disco"), "ES got {es}");
    }
}
