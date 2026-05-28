//! Cross-store integration coverage for `rubix.user.delete`
//! against real Postgres. The verb walks both
//! [`UserAdminStore`] (resolve target + delete) and
//! [`TeamAdminStore`] (cascade check: refuse if the user is a
//! member of any team).
//!
//! The unit tests in `rubix-tools` exercise this against
//! in-memory fakes \u{2014} this suite locks the same
//! contract against `PgUserAdminStore` + `PgTeamAdminStore`
//! so we'd catch:
//!
//! - a Pg store quietly returning a different error variant
//!   for `NotFound` than the in-memory fake,
//! - a `team_store.list()` shape mismatch between the JSONB
//!   `members` column and the `BTreeMap<String, i64>` the
//!   verb expects to walk,
//! - a teams-migration FK or unique constraint that bites
//!   only when both stores are wired live.
//!
//! Scenarios:
//!
//! 1. Delete an unassigned user against Pg \u{2014} succeeds,
//!    response carries the full prior row, subsequent `get`
//!    returns `None`.
//! 2. Create a user, assign them to a team, attempt delete
//!    \u{2014} refused with `Error::Conflict` whose message
//!    is the structured `rubix.user.in_teams` diagnostic
//!    naming the team. User row is still present.
//! 3. Unassign the user from the team, retry delete \u{2014}
//!    succeeds; the user row is gone, the team row still
//!    exists with empty members.

use std::collections::BTreeMap;
use std::sync::Arc;

use rubix_spi::dto::user::delete::UserDeleteResponse;
use rubix_spi::starter::error::Error;
use rubix_spi::team::{TeamAdminStore, TeamRow};
use rubix_spi::user::{UserAdminStore, UserRow};
use rubix_store_postgres::{
    PgTeamAdminStore, PgUserAdminStore, RUBIX_TEAMS_MIGRATION_SOURCE,
    RUBIX_TENANTS_MIGRATION_SOURCE, RUBIX_USERS_MIGRATION_SOURCE,
};
use rubix_tools::user::delete::UserDeleteTool;
use serde_json::json;
use starter_spi::tool::Tool;
use starter_store_postgres::{migrate, testing::with_database};

fn user_row(id: &str, email: &str) -> UserRow {
    UserRow {
        user_id: id.into(),
        email: email.into(),
        role: "reader".into(),
        disabled_at_ms: None,
        prefs_json: None,
        tenant_id: None,
    }
}

fn team_row(id: &str, name: &str) -> TeamRow {
    TeamRow {
        team_id: id.into(),
        name: name.into(),
        description: None,
        members: BTreeMap::new(),
    }
}

#[tokio::test]
#[ignore = "requires Docker (testcontainers Postgres); run via the integration job"]
async fn user_delete_cross_store_against_postgres() {
    let (pool, _guard) = with_database().await;
    migrate(&pool)
        .with_source(RUBIX_TENANTS_MIGRATION_SOURCE)
        .with_source(RUBIX_USERS_MIGRATION_SOURCE)
        .with_source(RUBIX_TEAMS_MIGRATION_SOURCE)
        .run()
        .await
        .expect("apply rubix-tenants/users/teams migrations");

    let users: Arc<dyn UserAdminStore> = Arc::new(PgUserAdminStore::new(pool.clone()));
    let teams: Arc<dyn TeamAdminStore> = Arc::new(PgTeamAdminStore::new(pool.clone()));
    let tool = UserDeleteTool::new(users.clone(), teams.clone());

    // ---- Scenario 1: unassigned user deletes cleanly. ----
    users
        .create(user_row("u-solo", "solo@x"))
        .await
        .expect("seed unassigned user");

    let out = tool
        .invoke(json!({ "user_id": "u-solo" }))
        .await
        .expect("delete unassigned user");
    let resp: UserDeleteResponse =
        serde_json::from_value(out).expect("UserDeleteResponse parses");
    assert_eq!(resp.user_id, "u-solo");
    assert_eq!(resp.email, "solo@x");
    assert_eq!(resp.role, "reader");
    assert!(resp.disabled_at_ms.is_none());
    assert!(resp.prefs_json.is_none());
    assert!(resp.tenant_id.is_none());
    assert!(resp.deleted_at_ms > 0);
    assert!(
        users.get("u-solo").await.expect("get").is_none(),
        "row removed after delete",
    );

    // ---- Scenario 2: user in a team \u{2192} refused. ----
    users
        .create(user_row("u-member", "member@x"))
        .await
        .expect("seed user");
    teams
        .create(team_row("t-ops", "Ops"))
        .await
        .expect("seed team");
    teams
        .assign("t-ops", "u-member", 1_700_000_000_000)
        .await
        .expect("assign u-member to Ops");

    let err = tool
        .invoke(json!({ "user_id": "u-member" }))
        .await
        .expect_err("delete refused while assigned");
    match err {
        Error::Conflict { message } => {
            // The diagnostic is serialized JSON \u{2014} we
            // assert on the structured payload, not on a
            // localized string.
            assert!(
                message.contains("rubix.user.in_teams"),
                "diagnostic key surfaces in conflict message: {message}",
            );
            assert!(
                message.contains("Ops"),
                "blocking team name surfaces in conflict message: {message}",
            );
            assert!(
                message.contains("u-member") || message.contains("member@x"),
                "user identity surfaces in conflict message: {message}",
            );
        }
        other => panic!("expected Conflict, got {other:?}"),
    }
    assert!(
        users.get("u-member").await.expect("get").is_some(),
        "user row preserved on refused delete",
    );

    // ---- Scenario 3: drain membership, retry. ----
    teams
        .unassign("t-ops", "u-member")
        .await
        .expect("unassign before retry");
    let out = tool
        .invoke(json!({ "email": "member@x" }))
        .await
        .expect("delete succeeds after unassign");
    let resp: UserDeleteResponse =
        serde_json::from_value(out).expect("UserDeleteResponse parses");
    assert_eq!(resp.user_id, "u-member");
    assert_eq!(resp.email, "member@x");
    assert!(
        users.get("u-member").await.expect("get").is_none(),
        "user row gone after retry",
    );
    let surviving = teams.get("t-ops").await.expect("get team").expect("present");
    assert!(
        surviving.members.is_empty(),
        "team row preserved with empty members after user delete: {:?}",
        surviving.members,
    );
}
