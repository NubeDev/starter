//! RW-07b acceptance: a contributed insight is removed on extension purge.
//!
//! The RW-07 deferral left this assertion open — the cleanup path existed and
//! was unit-covered, but no end-to-end test proved that purging an extension
//! actually deletes its contributed insight rows. This drives the real
//! [`InsightCleanupProvider`] against a live `nexus_extension_insights` table:
//! materialise a contributed insight, assert the dry-run discovers it, purge,
//! then assert it is gone and a second purge is idempotent.

#![cfg(feature = "testing")]

use nexus_api::extensions::cleanup_insights::InsightCleanupProvider;
use nexus_store::extension_insight::{self, NewExtensionInsight};
use starter_ext_server::CleanupProvider;
use starter_ext_spi::ExtensionId;
use starter_store_postgres::testing::with_database;

#[tokio::test]
#[ignore = "requires docker"]
async fn purge_removes_a_contributed_insight() {
    let (pool, _guard) = with_database().await;

    // Materialise a contributed insight, exactly as the boot/install path does
    // for an extension's `contributes.insights[]`.
    extension_insight::upsert(
        pool.sqlx(),
        "com.nexus.hello",
        &NewExtensionInsight {
            name: "com.nexus.hello.zscore".into(),
            script: "df.head(5)".into(),
            params_schema: None,
        },
    )
    .await
    .unwrap();

    let provider = InsightCleanupProvider::new(pool.sqlx().clone());
    let id = ExtensionId::new("com.nexus.hello").unwrap();

    // Dry-run discovers the contributed insight before any purge.
    let discovered = provider.discover(&id, None).await;
    assert!(
        discovered.iter().any(|i| i.label.contains("com.nexus.hello.zscore")),
        "the contributed insight shows in the cleanup dry-run"
    );

    // Purge removes it.
    provider.purge(&id, &discovered).await.unwrap();
    assert!(
        extension_insight::list_by_extension(pool.sqlx(), "com.nexus.hello")
            .await
            .unwrap()
            .is_empty(),
        "purge deletes the contributed insight rows"
    );

    // Re-purge is idempotent — deletes nothing, still Ok.
    provider.purge(&id, &[]).await.unwrap();
}
