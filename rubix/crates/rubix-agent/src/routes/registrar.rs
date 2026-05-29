//! Route registrar — single chokepoint for mounting any
//! rubix-agent-owned HTTP route.
//!
//! Wraps an [`axum::Router`] alongside a parallel
//! [`Vec<RouteEntry>`] catalog so that the live HTTP surface and
//! any projection of it (admin introspection, OpenAPI document)
//! cannot drift. Every rubix-agent route MUST be mounted through
//! [`RouteRegistrar::route`]; a workspace test
//! ([`tests/route_registrar_discipline_test.rs`](../../tests/route_registrar_discipline_test.rs))
//! fails if a raw `axum::Router::route` call leaks into
//! `crates/rubix-agent/src/` outside this file.
//!
//! Upstream routers (`starter-auth-users`, `starter-ext-server`,
//! `starter-warehouse-explorer`, `starter-server` / `starter-sdui-routes`)
//! own their own OpenAPI surface and are merged via
//! [`RouteRegistrar::merge_external`] — they appear in the live
//! router but not in the catalog projection.
//!
//! See [docs/design/admin/](../../../../docs/design/admin/README.md)
//! §"Route catalog" for the contract.//!
//! Naming note: the mount method is [`RouteRegistrar::mount`]
//! (not `route`) so the workspace discipline test can grep for
//! `.route(` and ban it everywhere outside this file without
//! false positives on the registrar's own call site.
use axum::http::Method;
use axum::routing::MethodRouter;
use axum::Router;
use serde::Serialize;
use serde_json::{json, Map, Value};

/// One row in the route catalog.
///
/// Schema fields are `Option<serde_json::Value>` rather than a
/// typed JSON Schema crate so per-verb builders can either
/// hand-author a small inline schema (the common case for query-
/// parameter handlers) or surface one derived from `schemars` /
/// `utoipa` when the request/response types are already
/// `JsonSchema`/`ToSchema`.
#[derive(Debug, Clone, Serialize)]
pub struct RouteEntry {
    pub method: String,
    pub path: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub request_schema: Option<Value>,
    pub response_schema: Option<Value>,
}

/// Per-route metadata recorded in the catalog at mount time.
#[derive(Debug, Default, Clone)]
pub struct RouteMeta {
    description: Option<String>,
    tags: Vec<String>,
    request_schema: Option<Value>,
    response_schema: Option<Value>,
}

impl RouteMeta {
    pub const fn new() -> Self {
        Self {
            description: None,
            tags: Vec::new(),
            request_schema: None,
            response_schema: None,
        }
    }

    pub fn describe(mut self, d: impl Into<String>) -> Self {
        self.description = Some(d.into());
        self
    }

    pub fn tag(mut self, t: impl Into<String>) -> Self {
        self.tags.push(t.into());
        self
    }

    pub fn request_schema(mut self, schema: Value) -> Self {
        self.request_schema = Some(schema);
        self
    }

    pub fn response_schema(mut self, schema: Value) -> Self {
        self.response_schema = Some(schema);
        self
    }
}

/// Registrar — every `route(...)` call appends to both the live
/// router and the catalog. Per-verb builders return a
/// `RouteRegistrar`; `main.rs` merges them all and projects the
/// final catalog into `/api/v1/admin/openapi.json`.
pub struct RouteRegistrar {
    router: Router,
    catalog: Vec<RouteEntry>,
}

impl Default for RouteRegistrar {
    fn default() -> Self {
        Self::new()
    }
}

impl RouteRegistrar {
    pub fn new() -> Self {
        Self {
            router: Router::new(),
            catalog: Vec::new(),
        }
    }

    /// Mount one (method, path) pair. The single chokepoint.
    ///
    /// `mr` is a state-bound `MethodRouter<()>` — per-verb
    /// builders are expected to call `get(handler).with_state(...)`
    /// (or similar) before passing it in, so the registrar itself
    /// is state-agnostic.
    pub fn mount(
        mut self,
        method: Method,
        path: &str,
        mr: MethodRouter<()>,
        meta: RouteMeta,
    ) -> Self {
        self.catalog.push(RouteEntry {
            method: method.as_str().to_ascii_uppercase(),
            path: path.to_owned(),
            description: meta.description,
            tags: meta.tags,
            request_schema: meta.request_schema,
            response_schema: meta.response_schema,
        });
        self.router = self.router.route(path, mr);
        self
    }

    /// Merge another registrar — router + catalog.
    pub fn merge(mut self, other: RouteRegistrar) -> Self {
        self.router = self.router.merge(other.router);
        self.catalog.extend(other.catalog);
        self
    }

    /// Merge an upstream `axum::Router` whose routes are not part
    /// of the rubix-agent catalog. Used for `starter-auth-users`,
    /// `starter-ext-server`, `starter-warehouse-explorer`,
    /// `starter-sdui-routes`, and the `starter-mcp` surface —
    /// each owns its own OpenAPI projection.
    pub fn merge_external(mut self, router: Router) -> Self {
        self.router = self.router.merge(router);
        self
    }

    /// Replace the inner router via a transformer. Used for
    /// middleware combinators (`with_principal`, `with_role`,
    /// `changelog_layer`, `gate_tools`) that take the router by
    /// value. Catalog is untouched.
    pub fn map_router<F>(mut self, f: F) -> Self
    where
        F: FnOnce(Router) -> Router,
    {
        self.router = f(self.router);
        self
    }

    pub fn catalog(&self) -> &[RouteEntry] {
        &self.catalog
    }

    pub fn into_router(self) -> Router {
        self.router
    }

    pub fn into_parts(self) -> (Router, Vec<RouteEntry>) {
        (self.router, self.catalog)
    }
}

/// Minimal `info` block carried into the OpenAPI projection.
#[derive(Debug, Clone)]
pub struct OpenApiInfo {
    pub title: String,
    pub version: String,
    pub description: Option<String>,
}

/// Project a route catalog into an OpenAPI 3.0.3 document.
///
/// Backs `GET /api/v1/admin/openapi.json`. Deliberately separate
/// from the utoipa-derived `/openapi.json` document (which merges
/// upstream `starter-auth-users` paths) — the admin variant is
/// strictly the projection of routes that flowed through the
/// registrar, so a route appearing here is a route the live
/// router serves.
pub fn catalog_to_openapi(catalog: &[RouteEntry], info: OpenApiInfo) -> Value {
    let mut paths: Map<String, Value> = Map::new();
    for entry in catalog {
        let path_item = paths
            .entry(entry.path.clone())
            .or_insert_with(|| Value::Object(Map::new()));
        let path_obj = path_item
            .as_object_mut()
            .expect("path_item is always inserted as Object");
        path_obj.insert(entry.method.to_ascii_lowercase(), build_operation(entry));
    }
    let mut info_obj = Map::new();
    info_obj.insert("title".into(), json!(info.title));
    info_obj.insert("version".into(), json!(info.version));
    if let Some(d) = info.description {
        info_obj.insert("description".into(), json!(d));
    }
    json!({
        "openapi": "3.0.3",
        "info": info_obj,
        "paths": paths,
    })
}

fn build_operation(entry: &RouteEntry) -> Value {
    let mut op = Map::new();
    if let Some(d) = &entry.description {
        let summary = d.lines().next().unwrap_or(d).trim();
        op.insert("summary".into(), json!(summary));
        op.insert("description".into(), json!(d));
    }
    if !entry.tags.is_empty() {
        op.insert("tags".into(), json!(entry.tags));
    }
    if let Some(schema) = &entry.request_schema {
        op.insert(
            "requestBody".into(),
            json!({
                "required": true,
                "content": { "application/json": { "schema": schema } }
            }),
        );
    }
    let response = match &entry.response_schema {
        Some(schema) => json!({
            "description": "OK",
            "content": { "application/json": { "schema": schema } }
        }),
        None => json!({ "description": "OK" }),
    };
    let mut responses = Map::new();
    responses.insert("200".into(), response);
    op.insert("responses".into(), json!(responses));
    op.insert(
        "operationId".into(),
        json!(operation_id(&entry.method, &entry.path)),
    );
    Value::Object(op)
}

fn operation_id(method: &str, path: &str) -> String {
    let mut id = method.to_ascii_lowercase();
    for ch in path.chars() {
        if ch.is_ascii_alphanumeric() {
            id.push(ch);
        } else if ch == '_' {
            id.push('_');
        } else if ch == '{' || ch == '}' {
            // skip path param braces
        } else {
            id.push('_');
        }
    }
    // collapse runs of '_'
    let mut out = String::with_capacity(id.len());
    let mut prev_us = false;
    for ch in id.chars() {
        if ch == '_' {
            if !prev_us {
                out.push(ch);
            }
            prev_us = true;
        } else {
            out.push(ch);
            prev_us = false;
        }
    }
    out.trim_matches('_').to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::routing::get;

    async fn handler() -> &'static str {
        "ok"
    }

    #[test]
    fn route_records_catalog_entry() {
        let reg = RouteRegistrar::new().mount(
            Method::GET,
            "/x",
            get(handler).with_state(()),
            RouteMeta::new().describe("xx").tag("t"),
        );
        let cat = reg.catalog();
        assert_eq!(cat.len(), 1);
        assert_eq!(cat[0].method, "GET");
        assert_eq!(cat[0].path, "/x");
        assert_eq!(cat[0].description.as_deref(), Some("xx"));
        assert_eq!(cat[0].tags, vec!["t".to_owned()]);
    }

    #[test]
    fn merge_concatenates_catalogs() {
        let a = RouteRegistrar::new().mount(
            Method::GET,
            "/a",
            get(handler).with_state(()),
            RouteMeta::new(),
        );
        let b = RouteRegistrar::new().mount(
            Method::GET,
            "/b",
            get(handler).with_state(()),
            RouteMeta::new(),
        );
        let merged = a.merge(b);
        assert_eq!(merged.catalog().len(), 2);
    }

    #[test]
    fn openapi_projects_one_path_per_entry() {
        let reg = RouteRegistrar::new()
            .mount(
                Method::GET,
                "/x",
                get(handler).with_state(()),
                RouteMeta::new(),
            )
            .mount(
                Method::POST,
                "/y/{id}",
                axum::routing::post(handler).with_state(()),
                RouteMeta::new()
                    .describe("create y")
                    .request_schema(json!({"type":"object"})),
            );
        let doc = catalog_to_openapi(
            reg.catalog(),
            OpenApiInfo {
                title: "t".into(),
                version: "0".into(),
                description: None,
            },
        );
        assert_eq!(doc["openapi"], "3.0.3");
        assert!(doc["paths"]["/x"]["get"].is_object());
        assert!(doc["paths"]["/y/{id}"]["post"]["requestBody"].is_object());
        assert_eq!(doc["paths"]["/y/{id}"]["post"]["summary"], "create y");
    }

    #[test]
    fn operation_id_is_method_plus_path_slug() {
        assert_eq!(
            operation_id("GET", "/api/v1/admin/registry"),
            "get_api_v1_admin_registry"
        );
        assert_eq!(
            operation_id("POST", "/api/v1/admin/registry/tools/{id}/invoke"),
            "post_api_v1_admin_registry_tools_id_invoke"
        );
    }
}
