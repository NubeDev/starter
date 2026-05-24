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
use starter_i18n::bundle::MessageBundle;
use starter_i18n::middleware::{accept_language_layer, LocaleCtx};
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

/// Build the tools router. Mounts `POST /api/v1/tools/{tool_id}`
/// and wraps it with [`accept_language_layer`] so the handler can
/// read the negotiated language from the [`LocaleCtx`] extension.
pub fn router(state: ToolsState) -> Router {
    let layer = accept_language_layer(state.bundle.clone());
    Router::new()
        .route("/api/v1/tools/{tool_id}", post(dispatch))
        .with_state(state)
        .layer(layer)
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
    Json(input): Json<Value>,
) -> Response {
    let Some(tool) = state.tools.get(&tool_id).cloned() else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "unknown tool", "tool_id": tool_id})),
        )
            .into_response();
    };
    let result = tool.invoke(input).await;
    shape_response(result, &state.bundle, locale.language().clone(), q.render.as_deref())
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

/// Pull `summary` from a tool result body, deserialise as
/// [`Diagnostic`], and render through
/// [`MessageBundle::render_diagnostic`] against the locale's
/// resolved preferences. Returns `None` if the body has no
/// `summary` field or it isn't a Diagnostic — the caller falls
/// through to the raw-JSON path.
fn render_summary(
    bundle: &MessageBundle,
    lang: &LanguageTag,
    body: &Value,
) -> Option<String> {
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
