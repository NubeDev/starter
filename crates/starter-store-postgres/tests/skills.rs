//! Round-trip smoke for the Postgres-backed `SkillApprovalStore`
//! (Phase 5, R-skills-7). Twin of the SQLite test under
//! `starter-store-sqlite/tests/skills.rs`.
//!
//! Marked `#[ignore]` because it requires Docker on the host (same
//! as `tests/migrate.rs`). CI runs it explicitly via
//! `cargo test -p starter-store-postgres --features
//!  "testing skill-approvals" -- --ignored`.

#![cfg(all(feature = "skill-approvals", feature = "testing"))]

use starter_flow_spi::skill::SkillId;
use starter_skills::{ApprovalRow, ApprovalStore};
use starter_store_postgres::skills::{SkillApprovalStore, SKILL_APPROVALS_MIGRATION_SOURCE};
use starter_store_postgres::{migrate, testing::with_database};

fn sid(s: &str) -> SkillId {
    SkillId::new(s).expect("valid skill id")
}

#[tokio::test]
#[ignore = "requires docker"]
async fn approval_round_trip_record_lookup_list_revoke() {
    let (pool, _guard) = with_database().await;
    migrate(&pool)
        .with_source(SKILL_APPROVALS_MIGRATION_SOURCE)
        .run()
        .await
        .expect("skill-approvals migrations apply");
    let store = SkillApprovalStore::new(pool);

    let row_a = ApprovalRow {
        skill_id: sid("starter.docs.search"),
        bundle_hash: "h-aaa".into(),
        approved_by: "alice".into(),
        approved_at_unix_ms: 1_700_000_000_000,
    };
    let row_b = ApprovalRow {
        skill_id: sid("starter.docs.write"),
        bundle_hash: "h-bbb".into(),
        approved_by: "bob".into(),
        approved_at_unix_ms: 1_700_000_000_500,
    };
    store.record(row_a.clone()).await.unwrap();
    store.record(row_b.clone()).await.unwrap();

    let got_a = store
        .lookup(&sid("starter.docs.search"), "h-aaa")
        .await
        .unwrap();
    assert_eq!(got_a, Some(row_a.clone()));
    assert!(store
        .lookup(&sid("starter.docs.search"), "h-other")
        .await
        .unwrap()
        .is_none());

    let mut listed = store.list().await.unwrap();
    listed.sort_by(|x, y| x.bundle_hash.cmp(&y.bundle_hash));
    assert_eq!(listed, vec![row_a, row_b.clone()]);

    store
        .revoke(&sid("starter.docs.search"), "h-aaa")
        .await
        .unwrap();
    assert!(store
        .lookup(&sid("starter.docs.search"), "h-aaa")
        .await
        .unwrap()
        .is_none());
    assert_eq!(
        store
            .lookup(&sid("starter.docs.write"), "h-bbb")
            .await
            .unwrap(),
        Some(row_b)
    );

    // Revoke of absent row is a no-op.
    store
        .revoke(&sid("starter.docs.search"), "h-aaa")
        .await
        .unwrap();
}
