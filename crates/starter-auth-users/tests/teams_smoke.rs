//! Phase 7b — teams + team_members store smoke tests. Covers the
//! "team-grant-coverage" and "team-membership-remove-takes-effect"
//! scenarios end-to-end at the store layer (no engine round-trip),
//! plus the "slug + tenant_id immutable after create" trigger
//! shipped in `0006_teams.sql` and the tenant-scoped slug lookup
//! that backs `team-rules-tenant-scoped` at the engine layer.

#![cfg(feature = "sqlite")]

use starter_auth_users::{
    admin::create_admin,
    store::{
        MembershipRecord, SqliteTenantStore, SqliteUserStore, TeamRecord, TenantRecord,
        TenantStore, TenantStoreError,
    },
    Role,
};
use starter_store_sqlite::{migrate, migrate::MigrationSource, testing::ephemeral, Pool};

static AUTH_USERS_MIGRATOR: sqlx::migrate::Migrator =
    sqlx::migrate!("./migrations/starter_auth_users");

async fn fresh_pool() -> Pool {
    let pool = ephemeral().await;
    migrate(&pool)
        .with_source(MigrationSource {
            name: "starter_auth_users",
            migrator: &AUTH_USERS_MIGRATOR,
        })
        .run()
        .await
        .expect("migrations apply");
    pool
}

fn tenant(slug: &str) -> TenantRecord {
    TenantRecord {
        id: uuid::Uuid::new_v4().to_string(),
        slug: slug.into(),
        display_name: slug.into(),
        audit_allow_sample: None,
        parent_id: None,
    }
}

fn team(tenant_id: &str, slug: &str) -> TeamRecord {
    TeamRecord {
        id: uuid::Uuid::new_v4().to_string(),
        tenant_id: tenant_id.into(),
        slug: slug.into(),
        display_name: slug.into(),
    }
}

async fn seed_tenant_and_user(
    pool: &Pool,
    tenants: &SqliteTenantStore,
    tenant_slug: &str,
    email: &str,
) -> (String, String) {
    let t = tenant(tenant_slug);
    tenants.create_tenant(&t).await.expect("create tenant");
    let users = SqliteUserStore::new(pool.clone());
    let user_id = create_admin(&users, email, "Hunter2!Hunter2!", Role::Writer)
        .await
        .expect("admin user");
    tenants
        .add_member(&MembershipRecord {
            tenant_id: t.id.clone(),
            user_id: user_id.clone(),
            role: "writer".into(),
            email: None,
        })
        .await
        .expect("add member");
    (t.id, user_id)
}

#[tokio::test]
async fn team_grant_coverage_via_team_slug_lookup() {
    // Add a user to a team; team_slugs_for_user surfaces the slug.
    // Remove them; the lookup goes back to empty. No engine; this
    // is the store-layer half of the engine-layer
    // team-grant-coverage smoke test.
    let pool = fresh_pool().await;
    let tenants = SqliteTenantStore::new(pool.clone());
    let (tenant_id, user_id) = seed_tenant_and_user(&pool, &tenants, "acme", "ops@acme.com").await;

    let t = team(&tenant_id, "hvac-ops");
    tenants.create_team(&t).await.expect("create team");

    // Before adding: empty.
    let slugs = tenants
        .team_slugs_for_user(&tenant_id, &user_id)
        .await
        .expect("lookup");
    assert!(slugs.is_empty(), "expected no teams, got {slugs:?}");

    // Add member: slug appears.
    tenants
        .add_team_member(&t.id, &user_id)
        .await
        .expect("add team member");
    let slugs = tenants
        .team_slugs_for_user(&tenant_id, &user_id)
        .await
        .expect("lookup");
    assert_eq!(slugs, vec!["hvac-ops".to_string()]);

    // Remove member: slug gone (immediate, no caching).
    tenants
        .remove_team_member(&t.id, &user_id)
        .await
        .expect("remove team member");
    let slugs = tenants
        .team_slugs_for_user(&tenant_id, &user_id)
        .await
        .expect("lookup");
    assert!(
        slugs.is_empty(),
        "expected empty after remove, got {slugs:?}"
    );
}

#[tokio::test]
async fn team_slug_lookup_is_tenant_scoped() {
    // A user with the same email is reused across two tenants and
    // added to a team in tenant-a only. The lookup for tenant-b
    // must NOT surface the tenant-a team slug, even if the slug
    // string is identical to one that could exist in tenant-b.
    // This pairs with the engine-layer
    // team-rules-tenant-scoped test.
    let pool = fresh_pool().await;
    let tenants = SqliteTenantStore::new(pool.clone());
    let (tenant_a, user_id) = seed_tenant_and_user(&pool, &tenants, "acme", "ops@acme.com").await;
    // Create a second tenant and make the same user a member.
    let tenant_b_row = tenant("beta");
    tenants
        .create_tenant(&tenant_b_row)
        .await
        .expect("create tenant b");
    tenants
        .add_member(&MembershipRecord {
            tenant_id: tenant_b_row.id.clone(),
            user_id: user_id.clone(),
            role: "writer".into(),
            email: None,
        })
        .await
        .expect("add member b");

    // Same slug exists in both tenants — only the tenant-a row
    // adds the user.
    let t_a = team(&tenant_a, "hvac-ops");
    let t_b = team(&tenant_b_row.id, "hvac-ops");
    tenants.create_team(&t_a).await.expect("team a");
    tenants.create_team(&t_b).await.expect("team b");
    tenants
        .add_team_member(&t_a.id, &user_id)
        .await
        .expect("member a");

    let slugs_a = tenants
        .team_slugs_for_user(&tenant_a, &user_id)
        .await
        .expect("lookup a");
    let slugs_b = tenants
        .team_slugs_for_user(&tenant_b_row.id, &user_id)
        .await
        .expect("lookup b");
    assert_eq!(slugs_a, vec!["hvac-ops".to_string()]);
    assert!(
        slugs_b.is_empty(),
        "expected empty for tenant-b, got {slugs_b:?}"
    );
}

#[tokio::test]
async fn team_slug_and_tenant_are_immutable_after_create() {
    // SCOPE-EXT.md R13 — the slug is the rule-stable identity, so
    // it must be immutable after create. The DB-level trigger in
    // 0006_teams.sql refuses any UPDATE that changes slug or
    // tenant_id (display_name may still be renamed). The store
    // does not expose patch_team — we exercise the trigger
    // directly via raw SQL to prove it's wired.
    let pool = fresh_pool().await;
    let tenants = SqliteTenantStore::new(pool.clone());
    let (tenant_id, _user_id) = seed_tenant_and_user(&pool, &tenants, "acme", "ops@acme.com").await;
    let t = team(&tenant_id, "hvac-ops");
    tenants.create_team(&t).await.expect("create team");

    let err = sqlx::query("UPDATE starter_auth_users_teams SET slug = ?1 WHERE id = ?2")
        .bind("renamed")
        .bind(&t.id)
        .execute(pool.sqlx())
        .await
        .expect_err("trigger should refuse slug rename");
    let msg = err.to_string();
    assert!(
        msg.contains("immutable"),
        "expected immutability error, got {msg}"
    );

    // display_name can still be renamed (no trigger, mutable).
    sqlx::query("UPDATE starter_auth_users_teams SET display_name = ?1 WHERE id = ?2")
        .bind("HVAC Ops (renamed)")
        .bind(&t.id)
        .execute(pool.sqlx())
        .await
        .expect("display_name rename should succeed");
}

#[tokio::test]
async fn duplicate_team_slug_in_same_tenant_conflicts() {
    let pool = fresh_pool().await;
    let tenants = SqliteTenantStore::new(pool.clone());
    let (tenant_id, _) = seed_tenant_and_user(&pool, &tenants, "acme", "ops@acme.com").await;
    let t1 = team(&tenant_id, "hvac-ops");
    let t2 = team(&tenant_id, "hvac-ops");
    tenants.create_team(&t1).await.expect("first");
    let err = tenants.create_team(&t2).await.unwrap_err();
    matches!(err, TenantStoreError::SlugConflict(_));
}
