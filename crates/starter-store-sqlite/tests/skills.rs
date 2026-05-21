//! Round-trip smoke for the SQLite-backed `SkillApprovalStore`
//! (Phase 5, R-skills-7). The test exercises every method on the
//! `ApprovalStore` trait against a freshly-migrated in-memory
//! pool: record → lookup → list → revoke → lookup-after-revoke
//! returns `None`. Process-restart durability is implicit (it is
//! just SQL); no separate test needed.

#![cfg(all(feature = "skill-approvals", feature = "testing"))]

use starter_flow_spi::skill::SkillId;
use starter_skills::{ApprovalRow, ApprovalStore};
use starter_store_sqlite::skills::{SkillApprovalStore, SKILL_APPROVALS_MIGRATION_SOURCE};
use starter_store_sqlite::{migrate, testing::ephemeral};

fn sid(s: &str) -> SkillId {
    SkillId::new(s).expect("valid skill id")
}

#[tokio::test]
async fn approval_round_trip_record_lookup_list_revoke() {
    let pool = ephemeral().await;
    migrate(&pool)
        .with_source(SKILL_APPROVALS_MIGRATION_SOURCE)
        .run()
        .await
        .expect("skill-approvals migrations apply");
    let store = SkillApprovalStore::new(pool);

    // 1. Record two distinct approval rows.
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

    // 2. Lookup hits and misses round-trip correctly.
    let got_a = store
        .lookup(&sid("starter.docs.search"), "h-aaa")
        .await
        .unwrap();
    assert_eq!(got_a, Some(row_a.clone()));
    let got_miss = store
        .lookup(&sid("starter.docs.search"), "h-other")
        .await
        .unwrap();
    assert!(got_miss.is_none());

    // 3. List returns both rows (order is unspecified per trait).
    let mut listed = store.list().await.unwrap();
    listed.sort_by(|x, y| x.bundle_hash.cmp(&y.bundle_hash));
    assert_eq!(listed, vec![row_a.clone(), row_b.clone()]);

    // 4. Revoke removes only the targeted row.
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

    // 5. Revoke of an absent row is a no-op (trait contract).
    store
        .revoke(&sid("starter.docs.search"), "h-aaa")
        .await
        .unwrap();
}
