//! Compose the axum router. The consumer's notes routes ride alongside
//! starter's `/health`, `/metrics`, `/openapi.json`, `/auth/claim`,
//! and `/mcp`. All of them are protected by the same
//! `TokenAuthenticator` so a single bearer works across surfaces.

use std::sync::Arc;

use axum::Router;
use prometheus::Registry;
use starter_auth_token::routes::ClaimState;
use starter_auth_token::store::SqliteClaimStore;
use starter_auth_token::TokenAuthenticator;
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

// `NotesApi` is the openapi document. `nest` would let us add a
// prefix or compose multiple sub-docs, but with one consumer surface
// the direct re-use is clearer.

pub struct Built {
    pub router: Router,
    pub authenticator: Arc<dyn Authenticator>,
    pub store: Arc<NoteStore>,
}

pub fn build(pool: Pool, registry: Arc<Registry>, metrics: Arc<StandardMetrics>) -> Built {
    let claim_state: ClaimState = Arc::new(SqliteClaimStore::new(pool.clone()));
    let authenticator: Arc<dyn Authenticator> =
        Arc::new(TokenAuthenticator::new(SqliteClaimStore::new(pool.clone())));

    let note_store = Arc::new(NoteStore::new(pool.sqlx().clone()));

    // /auth/claim — starter-shipped.
    let claim_router = starter_auth_token::routes::claim_router::<AppState>(claim_state);

    // /notes/* — consumer-owned, guarded by bearer.
    let (events, _) = tokio::sync::broadcast::channel(64);
    let notes = notes_router::<AppState>(NotesState {
        store: note_store.clone(),
        events,
    });
    // `with_principal` is generic over the Authenticator type; pass a
    // sized newtype rather than the `Arc<dyn ...>` we hold so the
    // bound is satisfied. The trait object lives on for gRPC reuse.
    let auth_for_http = Arc::new(BoxedAuthenticator(authenticator.clone()));
    let notes = with_principal(notes, auth_for_http.clone());

    // /mcp — starter-mcp dispatch with the consumer's tool registered.
    let tools = Arc::new(ToolRegistry::new().register(NoteSearchTool { store: note_store.clone() }));
    let mcp = mcp_router::<AppState>(tools, McpHttpOptions::new().with_auth(auth_for_http));

    let router = ServerBuilder::<AppState>::new(AppState)
        .merge_router(claim_router)
        .merge_router(notes)
        .merge_router(mcp)
        .with_openapi(AppApi::openapi())
        .with_metrics(registry, metrics)
        .build();

    Built { router, authenticator, store: note_store }
}

/// Newtype wrapping `Arc<dyn Authenticator>` so it can be passed to
/// `with_principal` / `McpHttpOptions::with_auth`, both of which take
/// `Arc<A: Authenticator + Sized>`.
struct BoxedAuthenticator(Arc<dyn Authenticator>);

#[async_trait::async_trait]
impl Authenticator for BoxedAuthenticator {
    async fn verify(
        &self,
        credential: &str,
    ) -> starter_spi::Result<starter_spi::auth::Principal> {
        self.0.verify(credential).await
    }
}
