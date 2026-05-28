//! Admin OpenAPI projection (`GET /api/v1/admin/openapi.json`)
//! — verifies the projection is a faithful reflection of the
//! [`RouteRegistrar`] catalog.
//!
//! Builds a small registrar with two known routes, projects them
//! through [`rubix_agent::routes::catalog_to_openapi`], and
//! asserts the resulting document has the expected paths,
//! methods, descriptions, and tags.
//!
//! The full live document is assembled in `main.rs` from every
//! per-verb registrar; this test exercises the projection
//! contract (not the full graph) so it stays hermetic — no DB,
//! no warehouse, no MCP wiring.

use axum::http::Method;
use axum::routing::{get, post};
use rubix_agent::routes::{catalog_to_openapi, OpenApiInfo, RouteMeta, RouteRegistrar};
use serde_json::{json, Value};

async fn ok() -> &'static str {
    "ok"
}

#[test]
fn projection_includes_every_mounted_path() {
    let reg = RouteRegistrar::new()
        .mount(
            Method::GET,
            "/api/v1/admin/registry",
            get(ok).with_state(()),
            RouteMeta::new()
                .describe("Multiplexed registry snapshot.")
                .tag("admin"),
        )
        .mount(
            Method::POST,
            "/api/v1/admin/registry/tools/{id}/invoke",
            post(ok).with_state(()),
            RouteMeta::new()
                .describe("Synchronously dispatch a tool.")
                .tag("admin")
                .request_schema(json!({
                    "type": "object",
                    "required": ["tenant"],
                    "properties": { "tenant": { "type": "string" } }
                })),
        );

    let doc = catalog_to_openapi(
        reg.catalog(),
        OpenApiInfo {
            title: "rubix-agent (admin projection)".into(),
            version: "test".into(),
            description: Some("projection test".into()),
        },
    );

    assert_eq!(doc["openapi"], "3.0.3");
    assert_eq!(doc["info"]["title"], "rubix-agent (admin projection)");
    assert_eq!(doc["info"]["version"], "test");

    let get_op = &doc["paths"]["/api/v1/admin/registry"]["get"];
    assert!(get_op.is_object(), "GET registry path projected; doc={doc}");
    assert_eq!(get_op["summary"], "Multiplexed registry snapshot.");
    assert_eq!(get_op["tags"], json!(["admin"]));

    let post_op = &doc["paths"]["/api/v1/admin/registry/tools/{id}/invoke"]["post"];
    assert!(
        post_op.is_object(),
        "POST invoke path projected; doc={doc}"
    );
    let body_schema =
        &post_op["requestBody"]["content"]["application/json"]["schema"];
    assert_eq!(body_schema["required"], json!(["tenant"]));
}

#[test]
fn projection_emits_unique_operation_ids() {
    let reg = RouteRegistrar::new()
        .mount(
            Method::GET,
            "/api/v1/admin/overview",
            get(ok).with_state(()),
            RouteMeta::new(),
        )
        .mount(
            Method::GET,
            "/api/v1/admin/registry/tools/{id}",
            get(ok).with_state(()),
            RouteMeta::new(),
        );
    let doc = catalog_to_openapi(
        reg.catalog(),
        OpenApiInfo {
            title: "t".into(),
            version: "0".into(),
            description: None,
        },
    );
    let mut ids: Vec<String> = Vec::new();
    let paths = doc["paths"].as_object().expect("paths object");
    for (_path, methods) in paths {
        for (_method, op) in methods.as_object().expect("path item is object") {
            if let Some(id) = op.get("operationId").and_then(Value::as_str) {
                ids.push(id.to_owned());
            }
        }
    }
    let mut sorted = ids.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(
        ids.len(),
        sorted.len(),
        "operationIds must be unique; got {ids:?}"
    );
}
