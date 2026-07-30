//! Phase 7a — tenants smoke tests (SCOPE-EXT.md R11/R12).

#![cfg(feature = "sqlite")]

use std::sync::Arc;

use starter_auth_users::{
    admin::create_admin,
    store::{
        is_reserved_slug, MembershipRecord, SqliteTenantStore, SqliteTokenStore, SqliteUserStore,
        TenantRecord, TenantStore, TenantStoreError, TokenStore,
    },
    token, Role,
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

#[tokio::test]
async fn reserved_slugs_are_rejected_at_application_level() {
    // Note: `system` is intentionally NOT reserved — migration 0007
    // promoted it to a real tenant row (the UNIQUE slug constraint
    // prevents duplicates), so it left the reserved list.
    for s in ["admin", "api", "auth", "v1", "v2", "static", "0", "123"] {
        assert!(is_reserved_slug(s), "expected {s} to be reserved");
    }
    assert!(!is_reserved_slug("acme"));
    assert!(!is_reserved_slug("acme1"));
}

#[tokio::test]
async fn create_tenant_with_reserved_slug_errors() {
    let pool = fresh_pool().await;
    let tenants = SqliteTenantStore::new(pool);
    let err = tenants.create_tenant(&tenant("admin")).await.unwrap_err();
    matches!(err, TenantStoreError::ReservedSlug(_));
}

#[tokio::test]
async fn create_tenant_with_all_digit_slug_errors() {
    let pool = fresh_pool().await;
    let tenants = SqliteTenantStore::new(pool);
    let err = tenants.create_tenant(&tenant("42")).await.unwrap_err();
    matches!(err, TenantStoreError::ReservedSlug(_));
}

#[tokio::test]
async fn token_immutability_trigger_rejects_tenant_change_on_update() {
    let pool = fresh_pool().await;
    let users = Arc::new(SqliteUserStore::new(pool.clone()));
    let tokens = Arc::new(SqliteTokenStore::new(pool.clone()));

    let user = create_admin(
        users.as_ref(),
        "ops@acme.com",
        "Hunter2!Hunter2!",
        Role::Admin,
    )
    .await
    .expect("admin user");

    let _issued = token::issue(tokens.as_ref(), &user, &[], "tenant-a", None)
        .await
        .expect("token issued");

    // Now try to flip the tenant_id by hand. The trigger must
    // refuse (RAISE(ABORT,...)).
    let res = sqlx::query(
        "UPDATE starter_auth_users_tokens SET tenant_id = 'tenant-b' WHERE user_id = ?1",
    )
    .bind(&user)
    .execute(pool.sqlx())
    .await;

    assert!(res.is_err(), "trigger should refuse tenant_id UPDATE");
    let msg = res.unwrap_err().to_string();
    assert!(
        msg.contains("immutable"),
        "trigger message should mention 'immutable': {msg}"
    );
}

#[tokio::test]
async fn remove_member_revokes_tokens_in_same_txn() {
    let pool = fresh_pool().await;
    let users = Arc::new(SqliteUserStore::new(pool.clone()));
    let tokens = Arc::new(SqliteTokenStore::new(pool.clone()));
    let tenants = SqliteTenantStore::new(pool.clone());

    let t = tenant("acme");
    tenants.create_tenant(&t).await.expect("tenant created");

    let user = create_admin(
        users.as_ref(),
        "alice@acme.com",
        "Hunter2!Hunter2!",
        Role::Writer,
    )
    .await
    .expect("user");

    tenants
        .add_member(&MembershipRecord {
            tenant_id: t.id.clone(),
            user_id: user.clone(),
            role: "writer".into(),
            email: None,
        })
        .await
        .expect("membership");

    let issued = token::issue(tokens.as_ref(), &user, &[], &t.id, None)
        .await
        .expect("token");

    // Active before revoke.
    assert!(tokens.find_active(&issued.id).await.unwrap().is_some());

    tenants
        .remove_member(&t.id, &user)
        .await
        .expect("remove_member");

    // Token now revoked by the cascade.
    assert!(
        tokens.find_active(&issued.id).await.unwrap().is_none(),
        "token should be revoked when membership removed"
    );
}
