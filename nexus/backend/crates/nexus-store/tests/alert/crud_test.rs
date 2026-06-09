//! Alerting store against real Postgres under the runtime role: rule CRUD with
//! its state row, the cross-tenant due-claim, event history, channels, and the
//! active-silence check — all tenant-isolated.

#![cfg(feature = "testing")]

use chrono::{Duration as ChronoDuration, Utc};
use nexus_store::alert::{channel, due, event, rule, silence};
use nexus_store::alert::{NewChannel, NewEvent, NewRule, NewSilence, RulePatch};
use nexus_store::testing::runtime_pool;
use serde_json::json;
use starter_store_postgres::testing::with_database;

fn new_rule(name: &str) -> NewRule {
    NewRule {
        name: name.into(),
        datasource_id: None,
        query: "SELECT 1".into(),
        op: "gt".into(),
        threshold: 90.0,
        for_secs: 0,
        interval_secs: 60,
        enabled: true,
        channel_ids: vec![],
    }
}

#[tokio::test]
#[ignore = "requires docker"]
async fn rule_crud_and_state_and_due_claim() {
    let (admin, _guard) = with_database().await;
    let pg = &runtime_pool(admin.sqlx()).await;

    let r = rule::insert(pg, "acme", &new_rule("cpu")).await.unwrap();
    // A state row is created with the rule.
    let st = rule::get_state(pg, "acme", r.id).await.unwrap().unwrap();
    assert_eq!(st.state, "ok");

    // Tenant isolation + name conflict.
    assert!(matches!(
        rule::insert(pg, "acme", &new_rule("cpu")).await,
        Err(starter_spi::Error::Conflict { .. })
    ));
    rule::insert(pg, "globex", &new_rule("cpu")).await.unwrap();
    assert_eq!(rule::list(pg, "acme").await.unwrap().len(), 1);

    // Patch + state write.
    rule::update(
        pg,
        "acme",
        r.id,
        &RulePatch {
            threshold: Some(80.0),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(rule::get(pg, "acme", r.id).await.unwrap().unwrap().threshold, 80.0);
    rule::put_state(pg, "acme", r.id, "firing", true, Some(95.0))
        .await
        .unwrap();
    let st = rule::get_state(pg, "acme", r.id).await.unwrap().unwrap();
    assert_eq!(st.state, "firing");
    assert_eq!(st.last_value, Some(95.0));

    // The due-claim sees rules across both tenants (system actor) and advances
    // next_eval_at so a second claim returns nothing immediately.
    let due1 = due::claim_due(pg, 10).await.unwrap();
    assert_eq!(due1.len(), 2, "both tenants' enabled rules are due at creation");
    let due2 = due::claim_due(pg, 10).await.unwrap();
    assert!(due2.is_empty(), "claimed rules are not immediately re-claimed");
}

#[tokio::test]
#[ignore = "requires docker"]
async fn events_channels_and_silences() {
    let (admin, _guard) = with_database().await;
    let pg = &runtime_pool(admin.sqlx()).await;
    let r = rule::insert(pg, "acme", &new_rule("disk")).await.unwrap();

    // Event history, newest first, tenant-scoped.
    event::insert(
        pg,
        "acme",
        &NewEvent {
            rule_id: r.id,
            transition: "firing".into(),
            value: Some(99.0),
            silenced: false,
            notified: true,
            detail: None,
        },
    )
    .await
    .unwrap();
    assert_eq!(event::list(pg, "acme", 50).await.unwrap().len(), 1);
    assert!(event::list(pg, "globex", 50).await.unwrap().is_empty());

    // Channel CRUD + by_ids lookup.
    let ch = channel::insert(
        pg,
        "acme",
        &NewChannel {
            name: "ops-webhook".into(),
            kind: "webhook".into(),
            config: json!({ "url": "https://example/hook" }),
        },
    )
    .await
    .unwrap();
    assert_eq!(channel::by_ids(pg, "acme", &[ch.id]).await.unwrap().len(), 1);
    assert!(channel::by_ids(pg, "globex", &[ch.id]).await.unwrap().is_empty());

    // A silence covering the rule suppresses notification for its window.
    let now = Utc::now();
    silence::insert(
        pg,
        "acme",
        &NewSilence {
            rule_id: Some(r.id),
            starts_at: now - ChronoDuration::minutes(1),
            ends_at: now + ChronoDuration::hours(1),
            reason: Some("deploy".into()),
            created_by: "alice".into(),
        },
    )
    .await
    .unwrap();
    assert!(silence::is_silenced(pg, "acme", r.id, now).await.unwrap());
    // Outside the window it does not apply.
    assert!(!silence::is_silenced(pg, "acme", r.id, now + ChronoDuration::hours(2))
        .await
        .unwrap());
    // And a tenant-wide silence (rule_id NULL) covers any rule.
    silence::insert(
        pg,
        "globex",
        &NewSilence {
            rule_id: None,
            starts_at: now - ChronoDuration::minutes(1),
            ends_at: now + ChronoDuration::hours(1),
            reason: None,
            created_by: "bob".into(),
        },
    )
    .await
    .unwrap();
    let other = rule::insert(pg, "globex", &new_rule("mem")).await.unwrap();
    assert!(silence::is_silenced(pg, "globex", other.id, now).await.unwrap());
}
