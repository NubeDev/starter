//! End-to-end proof: every surface (REST, MCP, gRPC) round-trips
//! against a real server built from `starter-notes::server::build`,
//! authenticating with an `owner_token` minted by the same
//! `/auth/claim` flow `starter-auth-token` ships.

use std::sync::Arc;

use prometheus::Registry;
use serde_json::Value;
use starter_notes::{grpc as notes_grpc, migrations, server as notes_server};
use starter_observability::metrics::StandardMetrics;
use starter_server::testing::TestApp;
use starter_store_sqlite::{migrate, pool};
use tonic::transport::Channel;
use tonic::Request;

use notes_grpc::proto::note_service_client::NoteServiceClient;
use notes_grpc::proto::{GetRequest, ListRequest};

#[tokio::test]
async fn every_surface_round_trips() {
    let pool = pool::connect("sqlite::memory:").await.expect("connect");
    let mut chain = migrate(&pool);
    for source in migrations::sources() {
        chain = chain.with_source(source);
    }
    chain.run().await.expect("migrate");

    let claim_store = starter_auth_token::store::SqliteClaimStore::new(pool.clone());
    let pending = starter_auth_token::regenerate_claim_pending(&claim_store)
        .await
        .expect("seed pending");

    let registry = Arc::new(Registry::new());
    let metrics = Arc::new(StandardMetrics::register(&registry).expect("metrics"));
    let built = notes_server::build(pool, registry, metrics);

    // Snapshot store + authenticator before move; gRPC needs them too.
    let grpc_store = built.store.clone();
    let grpc_auth = built.authenticator.clone();

    let app = TestApp::spawn(built.router).await;
    let http = reqwest::Client::new();

    // 1. Claim → owner token.
    let claim: Value = http
        .post(format!("{}/auth/claim", app.base_url))
        .json(&serde_json::json!({ "token": pending.plaintext }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let owner = claim["owner_token"].as_str().expect("owner_token").to_string();
    assert!(!owner.is_empty());

    // 2. REST: POST /notes (consumer route, behind starter's auth layer).
    let created: Value = http
        .post(format!("{}/notes", app.base_url))
        .bearer_auth(&owner)
        .json(&serde_json::json!({ "body": "buy milk" }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(created["body"], "buy milk");
    let note_id = created["id"].as_str().expect("id").to_string();

    // 3. REST: GET /notes → contains our note.
    let listed: Value = http
        .get(format!("{}/notes", app.base_url))
        .bearer_auth(&owner)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(listed.as_array().expect("array").len(), 1);

    // 4. REST without bearer → 401 from starter's `with_principal`.
    let resp = http.get(format!("{}/notes", app.base_url)).send().await.unwrap();
    assert_eq!(resp.status(), 401);

    // 5. MCP `tools/list` over HTTP — the consumer-registered
    //    `note_search` tool shows up next to starter-shipped behaviour.
    let listed_tools: Value = http
        .post(format!("{}/mcp", app.base_url))
        .bearer_auth(&owner)
        .body(r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let tools = listed_tools["result"]["tools"].as_array().expect("tools array");
    assert!(
        tools.iter().any(|t| t["name"] == "note_search"),
        "tools/list missing note_search: {tools:?}",
    );

    // 6. MCP `tools/call` → invokes the consumer tool, finds our note.
    let called: Value = http
        .post(format!("{}/mcp", app.base_url))
        .bearer_auth(&owner)
        .body(
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"note_search","arguments":{"query":"milk"}}}"#,
        )
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let hits = called["result"]["structuredContent"].as_array().expect("array");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0]["id"], note_id);

    // 7. gRPC — starter ships no gRPC support; this is entirely
    //    consumer-owned, but reuses the SAME authenticator + store.
    let grpc_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let grpc_addr = grpc_listener.local_addr().unwrap();
    let grpc_svc = notes_grpc::NotesGrpc {
        store: grpc_store,
        authenticator: grpc_auth,
    };
    let grpc_handle = tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(grpc_svc.into_server())
            .serve_with_incoming(tokio_stream::wrappers::TcpListenerStream::new(grpc_listener))
            .await
            .expect("grpc serve");
    });

    let channel = Channel::from_shared(format!("http://{grpc_addr}"))
        .unwrap()
        .connect()
        .await
        .unwrap();
    // Build an authenticated client without the `with_interceptor`
    // helper — its `FnMut -> Result<_, Status>` makes clippy unhappy
    // (`Status` is large). Setting the auth metadata per-call is
    // equivalent and clearer.
    let mut client = NoteServiceClient::new(channel);
    let auth_md: tonic::metadata::MetadataValue<_> =
        format!("Bearer {owner}").parse().unwrap();

    let mut list_req = Request::new(ListRequest {});
    list_req.metadata_mut().insert("authorization", auth_md.clone());
    let grpc_list = client.list(list_req).await.unwrap().into_inner();
    assert_eq!(grpc_list.notes.len(), 1);
    assert_eq!(grpc_list.notes[0].id, note_id);

    let mut get_req = Request::new(GetRequest { id: note_id.clone() });
    get_req.metadata_mut().insert("authorization", auth_md);
    let grpc_got = client.get(get_req).await.unwrap().into_inner();
    assert_eq!(grpc_got.body, "buy milk");

    grpc_handle.abort();
    app.shutdown().await;
}
