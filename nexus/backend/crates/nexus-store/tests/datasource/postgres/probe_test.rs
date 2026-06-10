//! The pre-save connection probe: connecting from raw params, before a
//! datasource row exists. A reachable Postgres returns `Ok`; an unreachable one
//! returns an error the route turns into `{ ok: false, message }`. The no-listener
//! case needs no database, so it runs in a normal `cargo test`; the success case
//! is docker-gated like the connect test.

use nexus_store::datasource::postgres::{self, ProbeParams};

/// A probe to a port with nothing listening fails — the form's "couldn't reach
/// the host" path. Binding an ephemeral port and immediately dropping the
/// listener gives an address that is closed but unlikely to be reused, so the
/// probe gets a real connection-refused rather than a hang.
#[tokio::test]
async fn probe_unreachable_host_errors() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let err = postgres::probe(ProbeParams {
        host: "127.0.0.1",
        port,
        database: "postgres",
        user: "postgres",
        secret: "postgres",
    })
    .await
    .expect_err("closed port must fail the probe");

    // The error carries a driver/connect reason for the form to surface; it must
    // never be empty (the form would show a blank failure otherwise).
    assert!(!err.to_string().is_empty());
}

#[cfg(feature = "testing")]
#[tokio::test]
#[ignore = "requires docker"]
async fn probe_reachable_postgres_succeeds() {
    use starter_store_postgres::testing::with_database;

    let (admin, _guard) = with_database().await;
    let opts = admin.sqlx().connect_options();
    let host = opts.get_host().to_string();
    let port = opts.get_port();

    postgres::probe(ProbeParams {
        host: &host,
        port,
        database: "postgres",
        user: "postgres",
        secret: "postgres",
    })
    .await
    .expect("probe to the live container connects");
}
