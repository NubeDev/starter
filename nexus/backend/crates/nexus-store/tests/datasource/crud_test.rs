//! Datasource CRUD against real Postgres: tenant-scoped, secret sealed at rest,
//! recovered only through the audited decrypt boundary, and isolated from other
//! tenants by RLS. Runs under the non-superuser runtime role so RLS is actually
//! enforced (a superuser would bypass it).

#![cfg(feature = "testing")]

use nexus_store::datasource::{self, Envelope, NewDatasource};
use nexus_store::testing::runtime_pool;
use sqlx::Row;
use starter_store_postgres::testing::with_database;

fn envelope() -> Envelope {
    Envelope::new(b"0123456789abcdef0123456789abcdef", 1).unwrap()
}

fn new_ds(name: &str) -> NewDatasource {
    NewDatasource {
        name: name.into(),
        kind: "postgres".into(),
        host: "db.internal".into(),
        port: 5432,
        database: "metrics".into(),
        db_user: "reader".into(),
        secret: "s3cr3t-pw".into(),
    }
}

#[tokio::test]
#[ignore = "requires docker"]
async fn insert_get_list_decrypt_are_tenant_scoped() {
    let (admin, _guard) = with_database().await;
    let pg = &runtime_pool(admin.sqlx()).await;
    let env = envelope();

    let created = datasource::insert(pg, &env, "acme", &new_ds("warehouse"))
        .await
        .expect("insert");
    assert_eq!(created.tenant_id, "acme");

    let got = datasource::get(pg, "acme", created.id)
        .await
        .expect("get")
        .expect("exists for acme");
    assert_eq!(got.name, "warehouse");

    // Another tenant cannot see it — get returns None, not the row.
    assert!(datasource::get(pg, "globex", created.id)
        .await
        .expect("get")
        .is_none());

    assert_eq!(datasource::list(pg, "acme").await.unwrap().len(), 1);
    assert_eq!(datasource::list(pg, "globex").await.unwrap().len(), 0);

    let secret = datasource::open_secret(pg, &env, "acme", "tester", created.id)
        .await
        .expect("decrypt");
    assert_eq!(secret, "s3cr3t-pw");
}

#[tokio::test]
#[ignore = "requires docker"]
async fn the_plaintext_secret_is_never_stored() {
    let (admin, _guard) = with_database().await;
    let pg = &runtime_pool(admin.sqlx()).await;

    let created = datasource::insert(pg, &envelope(), "acme", &new_ds("warehouse"))
        .await
        .unwrap();

    // Read the raw ciphertext as the admin (so RLS doesn't hide it) and confirm
    // the plaintext appears nowhere in it.
    let cipher: Vec<u8> = sqlx::query("SELECT secret_cipher FROM nexus_datasources WHERE id = $1")
        .bind(created.id)
        .fetch_one(admin.sqlx())
        .await
        .unwrap()
        .get("secret_cipher");
    assert!(
        !cipher.windows(8).any(|w| w == b"s3cr3t-p"),
        "the plaintext secret must not appear in the stored ciphertext"
    );
}

#[tokio::test]
#[ignore = "requires docker"]
async fn delete_removes_only_within_the_tenant() {
    let (admin, _guard) = with_database().await;
    let pg = &runtime_pool(admin.sqlx()).await;

    let created = datasource::insert(pg, &envelope(), "acme", &new_ds("warehouse"))
        .await
        .unwrap();

    assert!(!datasource::delete(pg, "globex", created.id).await.unwrap());
    assert!(datasource::get(pg, "acme", created.id)
        .await
        .unwrap()
        .is_some());

    assert!(datasource::delete(pg, "acme", created.id).await.unwrap());
    assert!(datasource::get(pg, "acme", created.id)
        .await
        .unwrap()
        .is_none());
}
