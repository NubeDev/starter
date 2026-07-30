//! Integration test for `PgTenantStore` — spins up a real Postgres
//! container via `starter-store-postgres::testing::with_database`,
//! applies the `auth_users` migration source, and exercises every
//! method on the `TenantStore` trait against the live DB.
//!
//! Also covers the trigger-enforced immutability: a direct UPDATE
//! to a token's `user_id`/`tenant_id`, a session's `user_id`, or a
//! team's `slug`/`tenant_id` must fail with Postgres SQLSTATE
//! `23514` (`check_violation`). See docs/design/auth/README.md.
//!
//! Marked `#[ignore]` by default: the testcontainers run requires
//! Docker on the host and is meant for the integration CI job.
//!
//! Run with:
//!
//! ```text
//! cargo test -p starter-auth-users --features postgres \
//!     --test pg_tenant_store -- --ignored
//! ```

#![cfg(feature = "postgres")]

use starter_auth_users::migration::postgres_migration_source;
use starter_auth_users::role::Role;
use starter_auth_users::store::{
    MembershipRecord, PgTenantStore, PgUserStore, TeamRecord, TenantRecord, TenantStore,
    TenantStoreError, UserStore,
};
use starter_store_postgres::{migrate, testing::with_database};

const ARGON2_FIXTURE: &str =
    "$argon2id$v=19$m=19456,t=2,p=1$Y2FmZWJhYmU$ZmFrZWZha2VmYWtlZmFrZWZha2VmYWtl";

const SQLSTATE_CHECK_VIOLATION: &str = "23514";

fn db_code(e: &sqlx::Error) -> Option<String> {
    match e {
        sqlx::Error::Database(d) => d.code().map(|c| c.into_owned()),
        _ => None,
    }
}

#[tokio::test]
#[ignore = "requires Docker / testcontainers; run via the integration CI job"]
async fn pg_tenant_store_round_trips_every_method_against_live_postgres() {
    let (pool, _guard) = with_database().await;

    migrate(&pool)
        .with_source(postgres_migration_source())
        .run()
        .await
        .expect("apply auth_users postgres migrations");

    // Seed users (memberships FK user_id → users(id)).
    let users = PgUserStore::new(pool.clone());
    users
        .create("user-1", "a@example.com", Some(ARGON2_FIXTURE), Role::Admin)
        .await
        .expect("seed user-1");
    users
        .create(
            "user-2",
            "b@example.com",
            Some(ARGON2_FIXTURE),
            Role::Reader,
        )
        .await
        .expect("seed user-2");

    let store = PgTenantStore::new(pool.clone());

    // ---------- tenants ----------

    // create_tenant — reserved slug list refused at the app layer.
    let reserved_err = store
        .create_tenant(&TenantRecord {
            id: "t-bad".into(),
            slug: "admin".into(),
            display_name: "Bad".into(),
            audit_allow_sample: None,
            parent_id: None,
        })
        .await
        .expect_err("reserved slug must be refused");
    assert!(matches!(reserved_err, TenantStoreError::ReservedSlug(_)));

    // create_tenant — DB-level CHECK refuses all-digits slug
    // (POSIX regex `slug !~ '^[0-9]'`); the impl surfaces it as
    // ReservedSlug.
    let digits_err = store
        .create_tenant(&TenantRecord {
            id: "t-digits".into(),
            slug: "123foo".into(),
            display_name: "Digits".into(),
            audit_allow_sample: None,
            parent_id: None,
        })
        .await
        .expect_err("all-digits-prefix slug must be refused by CHECK");
    assert!(matches!(digits_err, TenantStoreError::ReservedSlug(_)));

    store
        .create_tenant(&TenantRecord {
            id: "t-1".into(),
            slug: "acme".into(),
            display_name: "Acme".into(),
            audit_allow_sample: Some(50),
            parent_id: None,
        })
        .await
        .expect("create acme");

    store
        .create_tenant(&TenantRecord {
            id: "t-2".into(),
            slug: "globex".into(),
            display_name: "Globex".into(),
            audit_allow_sample: None,
            parent_id: None,
        })
        .await
        .expect("create globex");

    // create_tenant — slug collision returns SlugConflict.
    let conflict = store
        .create_tenant(&TenantRecord {
            id: "t-3".into(),
            slug: "acme".into(),
            display_name: "Dup".into(),
            audit_allow_sample: None,
            parent_id: None,
        })
        .await
        .expect_err("duplicate slug must be refused");
    assert!(matches!(conflict, TenantStoreError::SlugConflict(_)));

    let listed = store.list_tenants().await.expect("list_tenants");
    assert_eq!(listed.len(), 2);

    let by_id = store
        .get_tenant("t-1")
        .await
        .expect("get_tenant")
        .expect("t-1 exists");
    assert_eq!(by_id.slug, "acme");
    assert_eq!(by_id.audit_allow_sample, Some(50));

    let by_slug = store
        .get_tenant_by_slug("globex")
        .await
        .expect("get_tenant_by_slug")
        .expect("globex exists");
    assert_eq!(by_slug.id, "t-2");

    assert!(store
        .get_tenant("nope")
        .await
        .expect("get_tenant miss")
        .is_none());

    // patch_tenant — display_name + audit_allow_sample=None clears.
    store
        .patch_tenant("t-1", Some("Acme Renamed"), Some(None))
        .await
        .expect("patch_tenant");
    let post = store.get_tenant("t-1").await.unwrap().unwrap();
    assert_eq!(post.display_name, "Acme Renamed");
    assert_eq!(post.audit_allow_sample, None);

    // patch_tenant — no-op when both fields are None.
    store
        .patch_tenant("t-1", None, None)
        .await
        .expect("patch_tenant noop");

    // patch_tenant — missing id surfaces NotFound.
    let missing = store
        .patch_tenant("ghost", Some("x"), None)
        .await
        .expect_err("patch_tenant missing");
    assert!(matches!(missing, TenantStoreError::NotFound(_)));

    // ---------- memberships ----------

    store
        .add_member(&MembershipRecord {
            tenant_id: "t-1".into(),
            user_id: "user-1".into(),
            role: "admin".into(),
            email: None,
        })
        .await
        .expect("add member u1@t-1");
    store
        .add_member(&MembershipRecord {
            tenant_id: "t-1".into(),
            user_id: "user-2".into(),
            role: "reader".into(),
            email: None,
        })
        .await
        .expect("add member u2@t-1");
    store
        .add_member(&MembershipRecord {
            tenant_id: "t-2".into(),
            user_id: "user-1".into(),
            role: "writer".into(),
            email: None,
        })
        .await
        .expect("add member u1@t-2");

    // Duplicate membership → SlugConflict.
    let dup = store
        .add_member(&MembershipRecord {
            tenant_id: "t-1".into(),
            user_id: "user-1".into(),
            role: "admin".into(),
            email: None,
        })
        .await
        .expect_err("dup membership");
    assert!(matches!(dup, TenantStoreError::SlugConflict(_)));

    store
        .patch_member_role("t-1", "user-2", "writer")
        .await
        .expect("patch role");
    let m = store.memberships_for_user("user-2").await.unwrap();
    assert_eq!(m.len(), 1);
    assert_eq!(m[0].role, "writer");

    let members_t1 = store.members_of_tenant("t-1").await.unwrap();
    assert_eq!(members_t1.len(), 2);

    let memberships_u1 = store.memberships_for_user("user-1").await.unwrap();
    assert_eq!(memberships_u1.len(), 2);

    // Seed a token for u1@t-1 so remove_member cascades a revoke.
    sqlx::query(
        "INSERT INTO starter_auth_users_tokens \
         (id, user_id, hashed_token, tenant_id) VALUES ($1, $2, $3, $4)",
    )
    .bind("tok-cascade")
    .bind("user-1")
    .bind(ARGON2_FIXTURE)
    .bind("t-1")
    .execute(pool.sqlx())
    .await
    .expect("seed cascade token");

    store
        .remove_member("t-1", "user-1")
        .await
        .expect("remove membership cascades token revoke");

    let revoked_at: Option<chrono::DateTime<chrono::Utc>> =
        sqlx::query_scalar("SELECT revoked_at FROM starter_auth_users_tokens WHERE id = $1")
            .bind("tok-cascade")
            .fetch_one(pool.sqlx())
            .await
            .expect("read tok-cascade");
    assert!(
        revoked_at.is_some(),
        "membership remove must revoke tokens in the same txn"
    );

    let nope = store
        .remove_member("t-1", "ghost")
        .await
        .expect_err("missing membership");
    assert!(matches!(nope, TenantStoreError::NotFound(_)));

    // ---------- teams ----------

    store
        .create_team(&TeamRecord {
            id: "team-1".into(),
            tenant_id: "t-1".into(),
            slug: "hvac-ops".into(),
            display_name: "HVAC Ops".into(),
        })
        .await
        .expect("create team-1");
    store
        .create_team(&TeamRecord {
            id: "team-2".into(),
            tenant_id: "t-1".into(),
            slug: "fire".into(),
            display_name: "Fire".into(),
        })
        .await
        .expect("create team-2");

    let dup_team = store
        .create_team(&TeamRecord {
            id: "team-dup".into(),
            tenant_id: "t-1".into(),
            slug: "hvac-ops".into(),
            display_name: "Dup".into(),
        })
        .await
        .expect_err("dup (tenant, slug)");
    assert!(matches!(dup_team, TenantStoreError::SlugConflict(_)));

    let teams = store.list_teams("t-1").await.unwrap();
    assert_eq!(teams.len(), 2);

    let team = store.get_team("team-1").await.unwrap().unwrap();
    assert_eq!(team.slug, "hvac-ops");

    // team_slugs_for_user — tenant-scoped join.
    store
        .add_team_member("team-1", "user-2")
        .await
        .expect("add team member");
    store
        .add_team_member("team-2", "user-2")
        .await
        .expect("add team member");
    // Idempotent re-add.
    store
        .add_team_member("team-1", "user-2")
        .await
        .expect("re-add team member is idempotent");

    let slugs = store.team_slugs_for_user("t-1", "user-2").await.unwrap();
    assert_eq!(slugs, vec!["fire".to_string(), "hvac-ops".to_string()]);

    // Tenant filter — same user has no teams in t-2.
    let none = store.team_slugs_for_user("t-2", "user-2").await.unwrap();
    assert!(none.is_empty());

    store
        .remove_team_member("team-2", "user-2")
        .await
        .expect("remove team member");
    let slugs2 = store.team_slugs_for_user("t-1", "user-2").await.unwrap();
    assert_eq!(slugs2, vec!["hvac-ops".to_string()]);

    let ghost = store
        .remove_team_member("team-2", "user-2")
        .await
        .expect_err("already removed");
    assert!(matches!(ghost, TenantStoreError::NotFound(_)));

    // ---------- trigger-enforced immutability (SQLSTATE 23514) ----------

    // Teams: slug change is refused.
    let slug_err =
        sqlx::query("UPDATE starter_auth_users_teams SET slug = 'renamed' WHERE id = $1")
            .bind("team-1")
            .execute(pool.sqlx())
            .await
            .expect_err("team slug update must fail");
    assert_eq!(
        db_code(&slug_err).as_deref(),
        Some(SQLSTATE_CHECK_VIOLATION),
        "team slug update must raise check_violation (got {slug_err:?})"
    );

    // Teams: tenant_id change is refused.
    let tenant_err =
        sqlx::query("UPDATE starter_auth_users_teams SET tenant_id = 't-2' WHERE id = $1")
            .bind("team-1")
            .execute(pool.sqlx())
            .await
            .expect_err("team tenant_id update must fail");
    assert_eq!(
        db_code(&tenant_err).as_deref(),
        Some(SQLSTATE_CHECK_VIOLATION)
    );

    // Teams: display_name change is allowed (mutable).
    sqlx::query("UPDATE starter_auth_users_teams SET display_name = 'Renamed' WHERE id = $1")
        .bind("team-1")
        .execute(pool.sqlx())
        .await
        .expect("display_name is mutable");

    // Tokens: (user_id, tenant_id) immutability.
    let tok_err =
        sqlx::query("UPDATE starter_auth_users_tokens SET tenant_id = 't-2' WHERE id = $1")
            .bind("tok-cascade")
            .execute(pool.sqlx())
            .await
            .expect_err("token tenant_id update must fail");
    assert_eq!(db_code(&tok_err).as_deref(), Some(SQLSTATE_CHECK_VIOLATION));

    // Sessions: user_id immutability. Seed a session first.
    sqlx::query(
        "INSERT INTO starter_auth_users_sessions \
         (id, user_id, csrf_token, tenant_id, expires_at) \
         VALUES ($1, $2, $3, $4, NOW() + INTERVAL '1 hour')",
    )
    .bind("sess-1")
    .bind("user-1")
    .bind("csrf")
    .bind(Option::<String>::None)
    .execute(pool.sqlx())
    .await
    .expect("seed session");

    // NULL → set tenant_id is allowed (one-shot bind).
    sqlx::query("UPDATE starter_auth_users_sessions SET tenant_id = 't-1' WHERE id = $1")
        .bind("sess-1")
        .execute(pool.sqlx())
        .await
        .expect("session tenant_id NULL → set is allowed");

    // Re-bind (set → different value) is refused.
    let rebind =
        sqlx::query("UPDATE starter_auth_users_sessions SET tenant_id = 't-2' WHERE id = $1")
            .bind("sess-1")
            .execute(pool.sqlx())
            .await
            .expect_err("session tenant_id rebind must fail");
    assert_eq!(db_code(&rebind).as_deref(), Some(SQLSTATE_CHECK_VIOLATION));

    // user_id change is refused.
    let sess_user =
        sqlx::query("UPDATE starter_auth_users_sessions SET user_id = 'user-2' WHERE id = $1")
            .bind("sess-1")
            .execute(pool.sqlx())
            .await
            .expect_err("session user_id update must fail");
    assert_eq!(
        db_code(&sess_user).as_deref(),
        Some(SQLSTATE_CHECK_VIOLATION)
    );

    // ---------- delete_team cascades team_members ----------

    store.delete_team("team-1").await.expect("delete team-1");
    assert!(store.get_team("team-1").await.unwrap().is_none());
    let after = store.team_slugs_for_user("t-1", "user-2").await.unwrap();
    assert!(after.is_empty(), "team_members cascaded on team delete");

    let missing_team = store
        .delete_team("ghost")
        .await
        .expect_err("delete missing");
    assert!(matches!(missing_team, TenantStoreError::NotFound(_)));
}
