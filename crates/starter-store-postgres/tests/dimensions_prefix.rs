//! Tags T6 BI-4 enforcement: two packs claiming the same prefix
//! must fail the install transaction.

#![cfg(all(feature = "dimensions", feature = "testing"))]

use starter_store_postgres::dimensions::{tag_prefix_registry, DIMENSIONS_MIGRATION_SOURCE};
use starter_store_postgres::{migrate, testing::with_database};

#[tokio::test]
#[ignore = "requires docker"]
async fn two_packs_claiming_the_same_prefix_fail_the_txn() {
    let (pool, _g) = with_database().await;
    migrate(&pool)
        .with_source(DIMENSIONS_MIGRATION_SOURCE)
        .run()
        .await
        .expect("dimensions migrations apply");

    // Pack A claims `energy.*`.
    let mut tx_a = pool.sqlx().begin().await.unwrap();
    let res_a = tag_prefix_registry::register_in_tx(
        &mut tx_a,
        "energy.*",
        "pack:com.acme.energy",
        Some("ACME Energy pack"),
    )
    .await
    .unwrap();
    assert!(res_a.is_some(), "first registration must succeed");
    tx_a.commit().await.unwrap();

    // Pack B claims the same prefix inside a transaction. The
    // helper returns `Ok(None)` on unique-violation; the caller
    // is expected to abort the install txn.
    let mut tx_b = pool.sqlx().begin().await.unwrap();
    let res_b = tag_prefix_registry::register_in_tx(
        &mut tx_b,
        "energy.*",
        "pack:com.evil.energy",
        Some("competing pack"),
    )
    .await
    .unwrap();
    assert!(res_b.is_none(), "second registration must conflict");
    tx_b.rollback().await.unwrap();

    // The pack-A row survives; nothing leaked from pack B.
    let rows = tag_prefix_registry::list(&pool).await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].owner_pack, "pack:com.acme.energy");

    // The high-level `register` surface returns a typed Conflict
    // error with the pre-existing owner.
    let err = tag_prefix_registry::register(
        &pool,
        "energy.*",
        "pack:com.evil.energy",
        None,
    )
    .await
    .expect_err("conflict expected");
    match err {
        tag_prefix_registry::RegisterError::Conflict { existing_owner, .. } => {
            assert_eq!(existing_owner, "pack:com.acme.energy");
        }
        other => panic!("expected Conflict, got {other:?}"),
    }
}

#[tokio::test]
#[ignore = "requires docker"]
async fn prefix_shape_check_rejects_bad_inputs() {
    let (pool, _g) = with_database().await;
    migrate(&pool)
        .with_source(DIMENSIONS_MIGRATION_SOURCE)
        .run()
        .await
        .unwrap();

    // Missing trailing `.*`.
    let err = tag_prefix_registry::register(&pool, "energy", "pack:x", None).await;
    assert!(err.is_err(), "bare prefix must fail the CHECK");

    // Uppercase.
    let err = tag_prefix_registry::register(&pool, "Energy.*", "pack:x", None).await;
    assert!(err.is_err());
}
