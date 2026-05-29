//! [`rest_router`] — walks the registry, builds one merged
//! [`axum::Router`] from every `Validated` extension's
//! `contributes.rest[]` + `contributes.tools[]`.
//!
//! - Tool entries always mount at `POST /tools/<tool_id>`. The tool
//!   id is the manifest's full reverse-DNS id; the same id reaches MCP
//!   via the [`starter_ext_mcp`] adapter so an extension author writes
//!   one handler and gets two transports (R13).
//! - REST entries mount at the manifest's declared `method` + `path`.
//! - Path/method collisions across extensions are returned as
//!   [`RestBuildError::Collision`] *before* any router state is
//!   constructed — the operator sees one diagnostic with both ids
//!   instead of an Axum runtime panic.
//! - Each entry's `auth: { require_role, require_scope }` is wrapped
//!   around the entry's nested sub-router via
//!   [`super::auth::apply_gate`].
//!
//! All entries share one [`RestDispatcher`] supplied by the consumer;
//! the dispatcher decides per-extension how the call is fulfilled
//! (builtin direct, future process / wasm slices). The router holds an
//! `Arc<dyn RestDispatcher>` so cloning is cheap.

use std::collections::HashMap;
use std::sync::Arc;

use axum::routing::{delete, get, head, patch, post, put, MethodRouter};
use axum::Router;
use starter_ext_host::ExtensionRegistry;
use starter_ext_spi::{
    AuthGate, ContributeRest, ContributeTool, ExtensionId, LifecycleState, RestStreaming,
};
use starter_spi::authz::ResourceRegistry;

use super::auth::apply_gate;
use super::dispatcher::RestDispatcher;
use super::handler::{ndjson, non_streaming, sse, HandlerSpec};
use super::schema::SchemaCheck;

/// Build options. Kept as a struct so future knobs (`prefix:` to mount
/// every contributed route under `/ext/…`, a default `AuthGate`, …)
/// land without breaking existing callers.
#[derive(Default)]
pub struct RestRouterOptions {
    /// Optional prefix prepended to every contributed REST path. Tool
    /// routes (`POST /tools/<id>`) are not affected. Leave `None` for
    /// "mount at root" (the common case).
    pub path_prefix: Option<String>,
    /// Host `ResourceRegistry`, consulted at build time to validate
    /// every `ContributeRest.auth.permission.resource` against a
    /// registered kind (SCOPE-EXT R15 / Phase 7d). An unknown kind
    /// surfaces as [`RestBuildError::UnknownResource`] — symmetric
    /// with [`RestBuildError::UnknownRole`]. A manifest entry that
    /// declares a `permission` field without a registry wired in
    /// is rejected as `UnknownResource` for the same reason: the
    /// adapter cannot verify the kind exists, so the safe action
    /// is "refuse to mount" (loud failure beats silent skip).
    pub resource_registry: Option<Arc<dyn ResourceRegistry>>,
    /// Optional per-extension metrics registry. When wired, every
    /// dispatched REST request bumps `rest_requests_total` for its
    /// extension. Leave `None` (the default) for no metrics overhead.
    pub metrics: Option<starter_ext_metrics::MetricsRegistry>,
}

/// Errors raised at router build time.
#[derive(Debug, thiserror::Error)]
pub enum RestBuildError {
    /// Two contribute entries from (possibly different) extensions
    /// declared the same `(method, path)`. The adapter never silently
    /// shadows a route — both ids surface in the diagnostic.
    #[error("path collision on `{method} {path}` between {first:?} and {second:?}")]
    Collision {
        /// Method (uppercased).
        method: String,
        /// Path template.
        path: String,
        /// The id ("<extension>:<contribute_id>") that registered the
        /// route first.
        first: String,
        /// The id ("<extension>:<contribute_id>") that tried to
        /// register the colliding route.
        second: String,
    },

    /// A `require_role` value did not resolve to a known
    /// [`starter_spi::auth::Role`].
    #[error("entry {entry:?}: unknown role `{role}` (known: reader / writer / admin)")]
    UnknownRole {
        /// "<extension>:<contribute_id>"
        entry: String,
        /// The unrecognised role string.
        role: String,
    },

    /// A `permission.resource` value did not resolve to a kind
    /// registered in the host's
    /// [`starter_spi::authz::ResourceRegistry`]. Surfaces a typo
    /// in the manifest at deploy time rather than as a runtime
    /// 403. The broken extension refuses to mount; the rest of
    /// the host's extensions continue to load (the caller logs
    /// the error and skips this extension at the loader level).
    #[error(
        "entry {entry:?}: unknown permission resource `{resource}` \
         (register the kind on the host's ResourceRegistry, or remove \
         `auth.permission` from the manifest)"
    )]
    UnknownResource {
        /// "<extension>:<contribute_id>"
        entry: String,
        /// The unrecognised resource kind.
        resource: String,
    },

    /// A schema file referenced by a contribute entry could not be
    /// read. Loaders aggregate validation errors; the REST adapter
    /// returns at the first I/O failure so the caller sees one clear
    /// reason rather than a cascade.
    #[error("entry {entry:?}: reading schema file {path:?}: {source}")]
    SchemaIo {
        /// "<extension>:<contribute_id>"
        entry: String,
        /// Manifest-relative schema path.
        path: String,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// A schema file did not parse as JSON.
    #[error("entry {entry:?}: schema file {path:?} is not valid JSON: {source}")]
    SchemaJson {
        /// "<extension>:<contribute_id>"
        entry: String,
        /// Manifest-relative schema path.
        path: String,
        /// Underlying parse error.
        #[source]
        source: serde_json::Error,
    },

    /// The manifest's `method` field could not be mapped onto an HTTP
    /// method axum knows (`GET`, `POST`, `PUT`, `PATCH`, `DELETE`,
    /// `HEAD`).
    #[error("entry {entry:?}: unsupported HTTP method `{method}`")]
    UnsupportedMethod {
        /// "<extension>:<contribute_id>"
        entry: String,
        /// The unrecognised method string.
        method: String,
    },
}

/// Build the REST adapter router.
pub fn rest_router<S>(
    registry: Arc<ExtensionRegistry>,
    dispatcher: Arc<dyn RestDispatcher>,
    options: RestRouterOptions,
) -> Result<Router<S>, RestBuildError>
where
    S: Clone + Send + Sync + 'static,
{
    // First pass: collect every (method, path) the registry will mount.
    // Detect collisions before constructing any handler state so we can
    // return one diagnostic listing both ids without partial wiring.
    let mut planned: HashMap<(String, String), String> = HashMap::new();
    for record in registry.iter_validated() {
        let Some(manifest) = record.manifest.as_ref() else {
            continue;
        };
        if record.state != LifecycleState::Validated {
            continue;
        }
        let extension_id = match record.id.as_ref() {
            Some(id) => id,
            None => continue,
        };
        // Tools.
        for tool in &manifest.contributes.tools {
            let entry = format!("{}:{}", extension_id.as_str(), tool.id);
            let path = tool_path(&tool.id);
            check_collision(&mut planned, "POST", &path, &entry)?;
        }
        // REST entries.
        for rest in &manifest.contributes.rest {
            let entry = format!("{}:{}", extension_id.as_str(), rest.id);
            let method = rest.method.to_ascii_uppercase();
            let path = mount_path(&options.path_prefix, &rest.path);
            check_collision(&mut planned, &method, &path, &entry)?;
        }
    }

    // Second pass: build the router. Per-entry sub-routers carry their
    // own `HandlerSpec` as state so the outer router can stay generic
    // over the caller's app state `S`.
    let mut router: Router<S> = Router::new();
    for record in registry.iter_validated() {
        let Some(manifest) = record.manifest.as_ref() else {
            continue;
        };
        let Some(extension_id) = record.id.as_ref() else {
            continue;
        };
        let counters = options.metrics.as_ref().map(|m| m.counters(extension_id));
        for tool in &manifest.contributes.tools {
            router = mount_tool(
                router,
                dispatcher.clone(),
                extension_id,
                tool,
                &record.bundle_dir,
                options.resource_registry.as_deref(),
                counters.clone(),
            )?;
        }
        for rest in &manifest.contributes.rest {
            router = mount_rest(
                router,
                dispatcher.clone(),
                extension_id,
                rest,
                &record.bundle_dir,
                options.path_prefix.as_deref(),
                options.resource_registry.as_deref(),
                counters.clone(),
            )?;
        }
    }
    Ok(router)
}

// ---------------------------------------------------------------------------
// Mount helpers
// ---------------------------------------------------------------------------

fn mount_tool<S>(
    router: Router<S>,
    dispatcher: Arc<dyn RestDispatcher>,
    extension: &ExtensionId,
    tool: &ContributeTool,
    bundle_dir: &std::path::Path,
    resource_registry: Option<&dyn ResourceRegistry>,
    counters: Option<std::sync::Arc<starter_ext_metrics::Counters>>,
) -> Result<Router<S>, RestBuildError>
where
    S: Clone + Send + Sync + 'static,
{
    let entry_label = format!("{}:{}", extension.as_str(), tool.id);
    let schema = load_schema(bundle_dir, &tool.input_schema, &entry_label)?;
    let spec = HandlerSpec::new(dispatcher, extension.clone(), &tool.id, schema, counters);
    let path = tool_path(&tool.id);
    let sub: Router<S> = Router::new()
        .route(&path, post(non_streaming))
        .with_state(spec);
    let sub = apply_gate(sub, &tool.auth, &entry_label, resource_registry)?;
    Ok(router.merge(sub))
}

#[allow(clippy::too_many_arguments)]
fn mount_rest<S>(
    router: Router<S>,
    dispatcher: Arc<dyn RestDispatcher>,
    extension: &ExtensionId,
    rest: &ContributeRest,
    bundle_dir: &std::path::Path,
    path_prefix: Option<&str>,
    resource_registry: Option<&dyn ResourceRegistry>,
    counters: Option<std::sync::Arc<starter_ext_metrics::Counters>>,
) -> Result<Router<S>, RestBuildError>
where
    S: Clone + Send + Sync + 'static,
{
    let entry_label = format!("{}:{}", extension.as_str(), rest.id);
    let schema = match rest.request_schema.as_deref() {
        Some(rel) => load_schema(bundle_dir, rel, &entry_label)?,
        None => SchemaCheck::default(),
    };
    let spec = HandlerSpec::new(dispatcher, extension.clone(), &rest.id, schema, counters);
    let method = rest.method.to_ascii_uppercase();
    let method_router: MethodRouter<Arc<HandlerSpec>> = match (method.as_str(), rest.streaming) {
        ("GET", RestStreaming::Sse) => get(sse),
        ("GET", RestStreaming::Ndjson) => get(ndjson),
        ("GET", RestStreaming::None) => get(non_streaming),
        ("POST", RestStreaming::Sse) => post(sse),
        ("POST", RestStreaming::Ndjson) => post(ndjson),
        ("POST", RestStreaming::None) => post(non_streaming),
        ("PUT", RestStreaming::None) => put(non_streaming),
        ("PATCH", RestStreaming::None) => patch(non_streaming),
        ("DELETE", RestStreaming::None) => delete(non_streaming),
        ("HEAD", RestStreaming::None) => head(non_streaming),
        // Streaming on a non-GET/POST verb is unusual enough that we
        // refuse rather than silently downgrade. The error points at
        // the manifest field so the operator can pick a verb the
        // adapter handles.
        (other, _) => {
            return Err(RestBuildError::UnsupportedMethod {
                entry: entry_label,
                method: other.to_string(),
            });
        }
    };
    let path = mount_path(&path_prefix.map(str::to_string), &rest.path);
    let sub: Router<S> = Router::new().route(&path, method_router).with_state(spec);
    let sub = apply_gate(sub, &rest.auth, &entry_label, resource_registry)?;
    Ok(router.merge(sub))
}

fn check_collision(
    planned: &mut HashMap<(String, String), String>,
    method: &str,
    path: &str,
    entry: &str,
) -> Result<(), RestBuildError> {
    let key = (method.to_string(), path.to_string());
    if let Some(first) = planned.get(&key) {
        return Err(RestBuildError::Collision {
            method: method.to_string(),
            path: path.to_string(),
            first: first.clone(),
            second: entry.to_string(),
        });
    }
    planned.insert(key, entry.to_string());
    Ok(())
}

fn tool_path(tool_id: &str) -> String {
    format!("/tools/{tool_id}")
}

fn mount_path(prefix: &Option<String>, path: &str) -> String {
    let path = if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    };
    match prefix {
        Some(p) if !p.is_empty() => {
            let p = p.trim_end_matches('/');
            format!("{p}{path}")
        }
        _ => path,
    }
}

fn load_schema(
    bundle_dir: &std::path::Path,
    rel: &str,
    entry: &str,
) -> Result<SchemaCheck, RestBuildError> {
    let path = bundle_dir.join(rel);
    let bytes = std::fs::read(&path).map_err(|e| RestBuildError::SchemaIo {
        entry: entry.to_string(),
        path: rel.to_string(),
        source: e,
    })?;
    let value: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|e| RestBuildError::SchemaJson {
            entry: entry.to_string(),
            path: rel.to_string(),
            source: e,
        })?;
    Ok(SchemaCheck::from_value(&value))
}

// Auth gate borrowed from the manifest as-is. Kept as a function so the
// router code reads "wrap router in gate" without inlining the gate
// destructuring twice.
fn _gate_doc(_gate: &AuthGate) {}
