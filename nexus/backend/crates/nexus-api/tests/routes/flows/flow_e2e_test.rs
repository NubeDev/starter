//! Flow acceptance: a saved http_poll → pipeline → postgres flow, created and
//! started over the API, ingests a real HTTP response into a datasource table;
//! stopping it halts the run. This is the weather→Postgres shape from the
//! topology, with a local endpoint standing in for the upstream API.

#![cfg(feature = "testing")]

use std::sync::Arc;
use std::time::Duration;

use axum::Extension;
use nexus_api::middleware::StreamTokenSigner;
use nexus_api::serve;
use nexus_api::state::AppState;
use nexus_engine::{FlowManager, LiveRunner};
use nexus_store::datasource::Envelope;
use nexus_store::testing::runtime_pool;
use nexus_store::QueryGuards;
use serde_json::{json, Value};
use starter_authz::testing::AllowAll;
use starter_server::testing::TestApp;
use starter_spi::auth::{Principal, Role};
use starter_store_postgres::testing::with_database;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

fn test_state(pool: &sqlx::PgPool) -> AppState {
    AppState {
        metadata: pool.clone(),
        datasource: pool.clone(),
        datasource_pools: Default::default(),
        envelope: Envelope::new(b"0123456789abcdef0123456789abcdef", 1).unwrap(),
        guards: QueryGuards {
            statement_timeout: Duration::from_secs(5),
            max_rows: 1000,
            max_bytes: 8 * 1024 * 1024,
        },
        live: LiveRunner::new().expect("engine init"),
        flows: FlowManager::new().expect("flow manager init"),
        sessions: nexus_api::agents::SessionRunner::new(std::env::temp_dir().join("nexus-knowledge-test"), nexus_skills::BrevityMode::Off),
        stream_signer: StreamTokenSigner::new(*b"test-stream-key-0123456789abcdef"),
        stream_token_ttl: Duration::from_secs(60),
        engine: Arc::new(AllowAll),
        kinds: Arc::new(nexus_api::kinds::Registry::empty()),
        prefs: nexus_api::prefs::prefs_store(pool.clone()),
        changelog: nexus_api::changelog::ChangelogHandles::new(
            pool.clone(),
            Envelope::new(b"0123456789abcdef0123456789abcdef", 1).unwrap(),
        ),
    }
}

fn acme_admin() -> Principal {
    Principal {
        subject: "alice".into(),
        role: Role::Admin,
        scopes: vec![],
        tenant_id: Some("acme".into()),
        teams: vec![],
        tenant_scope: Vec::new(),
        extra: Value::Null,
    }
}

async fn serve_json(body: &'static str, times: usize) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        for _ in 0..times {
            let Ok((mut sock, _)) = listener.accept().await else {
                break;
            };
            let mut buf = [0u8; 1024];
            let _ = sock.read(&mut buf).await;
            let resp = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{}",
                body.len(),
                body
            );
            let _ = sock.write_all(resp.as_bytes()).await;
            let _ = sock.flush().await;
        }
    });
    format!("http://{addr}/")
}

#[tokio::test]
#[ignore = "requires docker"]
async fn flow_ingests_http_into_postgres_then_stops() {
    let (admin, _guard) = with_database().await;
    let port = admin.sqlx().connect_options().as_ref().get_port();
    let target_uri = format!("postgres://postgres:postgres@127.0.0.1:{port}/postgres");
    let pool = runtime_pool(admin.sqlx()).await;

    // The flow's output table. Created by admin; the flow output connects as the
    // superuser uri so the test does not need to provision grants for the write.
    sqlx::query("CREATE TABLE ingested (city text, temp_c int)")
        .execute(admin.sqlx())
        .await
        .unwrap();

    let url = serve_json(r#"{"city":"berlin","temp_c":21}"#, 4).await;

    let router = serve::router(test_state(&pool)).layer(Extension(acme_admin()));
    let app = TestApp::spawn(router).await;
    let client = reqwest::Client::new();

    // Create the flow: poll the endpoint, shape with SQL, write to `ingested`.
    let created: Value = client
        .post(format!("{}/api/v1/flows", app.base_url))
        .json(&json!({
            "name": "weather",
            "input": { "type": "http_poll", "url": url, "interval": "1s" },
            "pipeline": [
                { "type": "json_to_arrow" },
                { "type": "sql", "query": "SELECT city, temp_c FROM flow" }
            ],
            "output": { "type": "postgres", "uri": target_uri, "table": "ingested" }
        }))
        .send()
        .await
        .expect("create")
        .json()
        .await
        .expect("body");
    let flow_id = created["id"].as_str().expect("id");

    // Start it.
    let start = client
        .post(format!("{}/api/v1/flows/{flow_id}/start", app.base_url))
        .send()
        .await
        .expect("start");
    assert_eq!(start.status(), 200);

    // Within a few seconds the poll should have inserted at least one row.
    let mut inserted = 0i64;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(8);
    while tokio::time::Instant::now() < deadline {
        inserted = sqlx::query_scalar::<_, i64>("SELECT count(*) FROM ingested WHERE city='berlin'")
            .fetch_one(admin.sqlx())
            .await
            .unwrap();
        if inserted > 0 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    assert!(inserted > 0, "the flow ingested the HTTP response into postgres");

    // Stop it; the flow is no longer running.
    let stop: Value = client
        .post(format!("{}/api/v1/flows/{flow_id}/stop", app.base_url))
        .send()
        .await
        .expect("stop")
        .json()
        .await
        .expect("body");
    assert_eq!(stop["running"], false);
    assert_eq!(stop["enabled"], false);

    drop(app);
}
