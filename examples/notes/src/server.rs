//! Compose the axum router. The consumer's notes routes ride alongside
//! starter's `/health`, `/metrics`, `/openapi.json`, `/auth/claim`,
//! and `/mcp`. All of them are protected by the same
//! `TokenAuthenticator` so a single bearer works across surfaces.
//!
//! Extension hosting (per `DOCS/extensions/sessions/SCOPE.md` Phase 1):
//! the same `ServerBuilder` also mounts the `starter-ext-server` admin
//! slice (`/extensions/*`), the REST adapter for `contributes.rest[]` +
//! auto-mounted `POST /tools/<id>`, and the MCP adapter that registers
//! extension-contributed tools into the same `ToolRegistry` the
//! consumer's `NoteSearchTool` lives in. The extension gRPC backplane
//! is returned alongside the router so `main.rs` can register it on the
//! same tonic `Server` as `NoteService` (R4).
//!
//! `EXTENSIONS_DIR` overrides the default `./extensions/` scan root
//! (R2: an empty / missing directory is a no-op load, not an error).

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use prometheus::Registry;
use starter_auth_token::routes::ClaimState;
use starter_auth_token::store::SqliteClaimStore;
use starter_auth_token::TokenAuthenticator;
use starter_ext_grpc::proto::extension_grpc_server::ExtensionGrpcServer;
use starter_ext_grpc::{
    build_grpc_methods, extension_grpc_server, ExtensionGrpcService, NotWiredGrpcDispatcher,
    DEFAULT_REQUEST_TIMEOUT,
};
use starter_ext_host::{ExtensionRegistry, Loader};
use starter_ext_mcp::register_tools;
use starter_ext_sdk::builtin::BuiltinTable;
use starter_ext_server::{
    rest_router, router_with_auth, ExtensionAdmin, InMemoryEnablementStore, NotWiredDispatcher,
    RestRouterOptions,
};
use starter_mcp::{mcp_router, McpHttpOptions, ToolRegistry};
use starter_observability::metrics::StandardMetrics;
use starter_server::auth::with_principal;
use starter_server::ServerBuilder;
use starter_spi::auth::Authenticator;
use starter_store_sqlite::Pool;
use utoipa::OpenApi;

use crate::domain::NoteStore;
use crate::mcp::NoteSearchTool;
use crate::rest::{notes_router, NotesApi as AppApi, NotesState};

#[derive(Clone)]
pub struct AppState;

pub struct Built {
    pub router: Router,
    pub authenticator: Arc<dyn Authenticator>,
    pub store: Arc<NoteStore>,
    /// Tonic server for the extension gRPC backplane. Register on the
    /// same `tonic::transport::Server` as the consumer's `NoteService`
    /// (R4 — shared port, separate service definitions).
    pub extension_grpc: ExtensionGrpcServer<ExtensionGrpcService>,
}

pub fn build(pool: Pool, registry: Arc<Registry>, metrics: Arc<StandardMetrics>) -> Built {
    let claim_state: ClaimState = Arc::new(SqliteClaimStore::new(pool.clone()));
    let authenticator: Arc<dyn Authenticator> =
        Arc::new(TokenAuthenticator::new(SqliteClaimStore::new(pool.clone())));

    let note_store = Arc::new(NoteStore::new(pool.sqlx().clone()));

    // ---------------------------------------------------------------
    // Extension load (R2: empty / missing dir is a valid empty load).
    // ---------------------------------------------------------------
    let ext_dir = std::env::var_os("EXTENSIONS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("./extensions"));
    let candidates = Loader::scan(&ext_dir).validate_all();
    let mut ext_registry = ExtensionRegistry::new();
    let outcome = Loader::commit(candidates, &mut ext_registry);
    ext_registry.seal();
    let ext_registry = Arc::new(ext_registry);
    tracing::info!(
        validated = outcome.validated,
        failed = outcome.failed,
        dir = %ext_dir.display(),
        "extensions loaded",
    );

    // /auth/claim — starter-shipped.
    let claim_router = starter_auth_token::routes::claim_router::<AppState>(claim_state);

    // /notes/* — consumer-owned, guarded by bearer.
    let (events, _) = tokio::sync::broadcast::channel(64);
    let notes = notes_router::<AppState>(NotesState {
        store: note_store.clone(),
        events,
    });
    let auth_for_http = Arc::new(BoxedAuthenticator(authenticator.clone()));
    let notes = with_principal(notes, auth_for_http.clone());

    // /mcp — starter-mcp dispatch with the consumer's NoteSearchTool +
    // every builtin-flavour tool contributed by loaded extensions.
    let tools = ToolRegistry::new().register(NoteSearchTool {
        store: note_store.clone(),
    });
    // No builtin extensions ship in this example. The empty
    // BuiltinTable means process / wasm extensions still load, their
    // tool entries simply count as `tools_skipped_non_builtin`.
    let builtins = Arc::new(BuiltinTable::new());
    let (tools, mcp_outcome, mcp_result) =
        register_tools(&ext_registry, &builtins, tools);
    if let Err(err) = mcp_result {
        tracing::warn!(error = %err, "some extension tool bindings failed to wire");
    }
    tracing::info!(
        seen = mcp_outcome.extensions_seen,
        registered = mcp_outcome.tools_registered,
        skipped_non_builtin = mcp_outcome.tools_skipped_non_builtin,
        "extension MCP tools registered",
    );
    let mcp = mcp_router::<AppState>(
        Arc::new(tools),
        McpHttpOptions::new().with_auth(auth_for_http.clone()),
    );

    // /extensions/* — admin slice (gated Role::Admin by
    // `router_with_auth`; UI bundle path is intentionally unauthed).
    let admin = ExtensionAdmin::builder(ext_registry.clone())
        .with_enablement_store(Arc::new(InMemoryEnablementStore::default()))
        .build();
    let admin_router =
        router_with_auth::<AppState, _>(admin, Arc::new(BoxedAuthenticator(authenticator.clone())));

    // Extension-contributed REST routes + auto-mounted POST /tools/<id>.
    // Non-builtin records dispatch through NotWiredDispatcher — they
    // return 501 until the consumer wires a per-flavour dispatcher.
    let rest_dispatcher: Arc<dyn starter_ext_server::RestDispatcher> =
        Arc::new(NotWiredDispatcher);
    let ext_rest = match rest_router::<AppState>(
        ext_registry.clone(),
        rest_dispatcher,
        RestRouterOptions::default(),
    ) {
        Ok(r) => r,
        Err(err) => {
            // A collision / bad manifest at this stage is a deploy-time
            // mistake — surface it loudly. The server still starts
            // without the extension REST surface so admin can fix it.
            tracing::error!(error = %err, "extension REST router build failed; skipping");
            Router::new()
        }
    };

    let router = ServerBuilder::<AppState>::new(AppState)
        .merge_router(claim_router)
        .merge_router(notes)
        .merge_router(mcp)
        .merge_router(admin_router)
        .merge_router(ext_rest)
        .with_openapi(AppApi::openapi())
        .with_metrics(registry, metrics)
        .build();

    // Extension gRPC backplane (R4). Same tonic Server, separate
    // service from NoteService.
    let grpc_methods = build_grpc_methods(&ext_registry).unwrap_or_else(|err| {
        tracing::error!(error = %err, "extension gRPC build failed; backplane empty");
        Vec::new()
    });
    let extension_grpc = extension_grpc_server(
        grpc_methods,
        Arc::new(NotWiredGrpcDispatcher),
        Duration::from_secs(DEFAULT_REQUEST_TIMEOUT.as_secs()),
    );

    Built {
        router,
        authenticator,
        store: note_store,
        extension_grpc,
    }
}

/// Newtype wrapping `Arc<dyn Authenticator>` so it can be passed to
/// `with_principal` / `McpHttpOptions::with_auth` /
/// `router_with_auth`, all of which take `Arc<A: Authenticator + Sized>`.
struct BoxedAuthenticator(Arc<dyn Authenticator>);

#[async_trait::async_trait]
impl Authenticator for BoxedAuthenticator {
    async fn verify(&self, credential: &str) -> starter_spi::Result<starter_spi::auth::Principal> {
        self.0.verify(credential).await
    }
}
