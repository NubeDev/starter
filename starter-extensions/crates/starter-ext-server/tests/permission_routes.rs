//! Phase 7d / SCOPE-EXT R15 smoke tests for the REST adapter's
//! per-entry `auth.permission` field.
//!
//! Three cases:
//!
//! * [`per_entry_permission_applied`] — reader principal with an
//!   explicit `(weather, read)` grant gets 200 on the read route
//!   and 403 on the refresh route; granting `(weather, refresh)`
//!   makes the refresh route 200 too.
//! * [`unknown_resource_is_build_error`] — a manifest with
//!   `permission: { resource: doesnt_exist }` causes
//!   `rest_router::build` to return `RestBuildError::UnknownResource`
//!   so the broken extension refuses to mount.
//! * [`role_and_permission_compose_correctly`] — a request that
//!   fails the outer `require_role` gate gets 403 from `with_role`
//!   and the permission middleware never runs (no
//!   `engine.check` invocation).
//!
//! The audit consequence ("role-deny never reaches the
//! permission gate, dashboards must exclude pre-role rejections")
//! is asserted by the test's `engine_calls` counter staying at
//! zero for the role-deny case.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use axum::{Extension, Router};
use starter_ext_host::{ExtensionRegistry, Loader};
use starter_ext_sdk::builtin::{BuiltinEntry, BuiltinTable};
use starter_ext_server::{rest_router, BuiltinRestDispatcher, RestBuildError, RestRouterOptions};
use starter_spi::auth::{Principal, Role};
use starter_spi::authz::{
    Decision, Ownership, PolicyEngine, ResourceRef, ResourceRegistry, ResourceSpec,
};
use tempfile::tempdir;
use tower::ServiceExt;

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

const WEATHER_BUNDLE: &str = r#"
v: 1
id: com.acme.weather
version: 0.1.0
display_name: "Weather"
runtime: { kind: builtin, crate_name: weather }
contributes:
  rest:
    - id: com.acme.weather.forecast
      method: GET
      path: /weather/forecast
      description_file: docs/forecast.md
      auth:
        permission: { resource: weather, action: read }
    - id: com.acme.weather.refresh
      method: POST
      path: /weather/refresh
      description_file: docs/refresh.md
      auth:
        permission: { resource: weather, action: refresh }
"#;

const ADMIN_ONLY_BUNDLE: &str = r#"
v: 1
id: com.acme.admin
version: 0.1.0
display_name: "Admin"
runtime: { kind: builtin, crate_name: admin }
contributes:
  rest:
    - id: com.acme.admin.tools
      method: GET
      path: /admin/tools
      description_file: docs/tools.md
      auth:
        require_role: admin
        permission: { resource: weather, action: read }
"#;

const BAD_RESOURCE_BUNDLE: &str = r#"
v: 1
id: com.acme.bad
version: 0.1.0
display_name: "Bad"
runtime: { kind: builtin, crate_name: bad }
contributes:
  rest:
    - id: com.acme.bad.endpoint
      method: GET
      path: /bad/endpoint
      description_file: docs/endpoint.md
      auth:
        permission: { resource: doesnt_exist, action: read }
"#;

fn write_file(root: &std::path::Path, rel: &str, body: &[u8]) {
    let p = root.join(rel);
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(p, body).unwrap();
}

fn write_bundle(root: &std::path::Path, dir_name: &str, yaml: &str, docs: &[&str]) {
    let dir = root.join(dir_name);
    std::fs::create_dir_all(&dir).unwrap();
    write_file(&dir, "block.yaml", yaml.as_bytes());
    for d in docs {
        write_file(&dir, d, b"# doc");
    }
}

fn load_registry(root: &std::path::Path) -> Arc<ExtensionRegistry> {
    let recs = Loader::scan(root).validate_all();
    let mut reg = ExtensionRegistry::new();
    Loader::commit(recs, &mut reg);
    reg.seal();
    Arc::new(reg)
}

fn weather_builtins() -> Arc<BuiltinTable> {
    let mut table = BuiltinTable::new();
    let ext_id = starter_ext_spi::ExtensionId::new("com.acme.weather").unwrap();
    table.insert(
        ext_id,
        BuiltinEntry::new(
            &["com.acme.weather.forecast", "com.acme.weather.refresh"],
            |_id, _ctx, _params| Ok(serde_json::json!({ "ok": true })),
        ),
    );
    let ext_id = starter_ext_spi::ExtensionId::new("com.acme.admin").unwrap();
    table.insert(
        ext_id,
        BuiltinEntry::new(&["com.acme.admin.tools"], |_id, _ctx, _params| {
            Ok(serde_json::json!({ "ok": true }))
        }),
    );
    Arc::new(table)
}

/// Minimal `ResourceRegistry` for the tests. Just enough for
/// `apply_gate` to call `lookup("weather")` successfully.
#[derive(Default)]
struct TestRegistry {
    inner: std::sync::RwLock<HashMap<String, ResourceSpec>>,
}

impl TestRegistry {
    fn with_weather() -> Arc<dyn ResourceRegistry> {
        let me = Self::default();
        me.inner.write().unwrap().insert(
            "weather".to_string(),
            ResourceSpec::from_static(
                "weather",
                &["read", "refresh"],
                Ownership::None,
                "Weather",
                "Test resource.",
            ),
        );
        Arc::new(me)
    }
}

impl ResourceRegistry for TestRegistry {
    fn register(&self, spec: ResourceSpec) {
        self.inner.write().unwrap().insert(spec.kind.clone(), spec);
    }
    fn known(&self) -> Vec<ResourceSpec> {
        self.inner.read().unwrap().values().cloned().collect()
    }
    fn lookup(&self, kind: &str) -> Option<ResourceSpec> {
        self.inner.read().unwrap().get(kind).cloned()
    }
}

/// Stub engine: counts `check` calls, allows whatever
/// `(action, kind)` pair is listed in `grants`. Lets tests prove
/// the permission middleware ran (or didn't) and what it asked.
struct StubEngine {
    grants: HashSet<(String, String)>,
    calls: Arc<AtomicUsize>,
}

impl StubEngine {
    fn new(grants: &[(&str, &str)], calls: Arc<AtomicUsize>) -> Arc<Self> {
        Arc::new(Self {
            grants: grants
                .iter()
                .map(|(k, a)| (k.to_string(), a.to_string()))
                .collect(),
            calls,
        })
    }
}

#[async_trait]
impl PolicyEngine for StubEngine {
    async fn check(&self, _p: &Principal, action: &str, obj: &ResourceRef) -> Decision {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self
            .grants
            .contains(&(obj.kind.to_string(), action.to_string()))
        {
            Decision::allow()
        } else {
            Decision::deny("no_matching_rule")
        }
    }
}

fn reader_principal() -> Principal {
    Principal {
        subject: "alice".to_string(),
        role: Role::Reader,
        scopes: vec![],
        tenant_id: None,
        teams: vec![],
        extra: serde_json::Value::Null,
    }
}

/// Mount the extension router under a thin layer that injects the
/// engine + the test principal into the request extensions. Models
/// what `with_principal` + `Extension(engine)` do in a real host.
fn mount<S: Clone + Send + Sync + 'static>(
    inner: Router<S>,
    engine: Arc<dyn PolicyEngine>,
    principal: Principal,
) -> Router<S> {
    inner.layer(Extension(engine)).layer(Extension(principal))
}

// ---------------------------------------------------------------------------
// per-entry-permission-applied
// ---------------------------------------------------------------------------

#[tokio::test]
async fn per_entry_permission_applied() {
    let tmp = tempdir().unwrap();
    write_bundle(
        tmp.path(),
        "com.acme.weather",
        WEATHER_BUNDLE,
        &["docs/forecast.md", "docs/refresh.md"],
    );
    let registry = load_registry(tmp.path());
    let dispatcher = Arc::new(BuiltinRestDispatcher::new(
        weather_builtins(),
        registry.clone(),
    ));
    let res_registry = TestRegistry::with_weather();
    let rest: Router<()> = rest_router(
        registry,
        dispatcher,
        RestRouterOptions {
            path_prefix: None,
            resource_registry: Some(res_registry),
            metrics: None,
        },
    )
    .expect("router builds");

    // Reader with `(weather, read)` only.
    let calls = Arc::new(AtomicUsize::new(0));
    let engine = StubEngine::new(&[("weather", "read")], calls.clone());
    let app = mount(rest.clone(), engine, reader_principal());

    // 200 on the read route.
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/weather/forecast")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 403 on the refresh route (no grant).
    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/weather/refresh")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    assert!(calls.load(Ordering::SeqCst) >= 2);

    // Grant `(weather, refresh)`; both routes succeed.
    let calls = Arc::new(AtomicUsize::new(0));
    let engine = StubEngine::new(
        &[("weather", "read"), ("weather", "refresh")],
        calls.clone(),
    );
    let app = mount(rest, engine, reader_principal());
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/weather/refresh")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/weather/forecast")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

// ---------------------------------------------------------------------------
// unknown-resource-is-build-error
// ---------------------------------------------------------------------------

#[tokio::test]
async fn unknown_resource_is_build_error() {
    let tmp = tempdir().unwrap();
    write_bundle(
        tmp.path(),
        "com.acme.bad",
        BAD_RESOURCE_BUNDLE,
        &["docs/endpoint.md"],
    );
    let registry = load_registry(tmp.path());
    let dispatcher = Arc::new(BuiltinRestDispatcher::new(
        weather_builtins(),
        registry.clone(),
    ));
    let res_registry = TestRegistry::with_weather();
    let err = rest_router::<()>(
        registry,
        dispatcher,
        RestRouterOptions {
            path_prefix: None,
            resource_registry: Some(res_registry),
            metrics: None,
        },
    )
    .expect_err("unknown resource must surface as build error");
    match err {
        RestBuildError::UnknownResource { entry, resource } => {
            assert!(entry.contains("com.acme.bad"), "entry: {entry}");
            assert_eq!(resource, "doesnt_exist");
        }
        other => panic!("expected UnknownResource, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// role+permission compose correctly
// ---------------------------------------------------------------------------

#[tokio::test]
async fn role_and_permission_compose_correctly() {
    let tmp = tempdir().unwrap();
    write_bundle(
        tmp.path(),
        "com.acme.admin",
        ADMIN_ONLY_BUNDLE,
        &["docs/tools.md"],
    );
    let registry = load_registry(tmp.path());
    let dispatcher = Arc::new(BuiltinRestDispatcher::new(
        weather_builtins(),
        registry.clone(),
    ));
    let res_registry = TestRegistry::with_weather();
    let rest: Router<()> = rest_router(
        registry,
        dispatcher,
        RestRouterOptions {
            path_prefix: None,
            resource_registry: Some(res_registry),
            metrics: None,
        },
    )
    .expect("router builds");

    // Reader hits the admin-gated route. `with_role` (outer) rejects
    // before `with_permission` (inner) ever runs — the audit
    // consequence the SCOPE calls out. The engine's call counter
    // stays at zero.
    let calls = Arc::new(AtomicUsize::new(0));
    let engine = StubEngine::new(&[("weather", "read")], calls.clone());
    let app = mount(rest, engine, reader_principal());
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/admin/tools")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    assert_eq!(
        calls.load(Ordering::SeqCst),
        0,
        "policy engine must not be consulted when require_role fails first \
         (audit consequence: no permission-deny entry for pre-role rejections)"
    );
}
