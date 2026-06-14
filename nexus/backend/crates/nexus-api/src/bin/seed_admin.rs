//! Seed an admin tenant + user + global admin grant into the metadata database.
//!
//! Idempotent bootstrap for a fresh deployment: it applies every crate's
//! migrations (same set the server runs at startup), then creates one tenant,
//! one admin user bound to it, and an authz assignment that grants that user the
//! `admin` role. Re-running is safe — each step treats an existing row as
//! success, so `make seed` can run on every boot without erroring.
//!
//! Credentials and connection come from the environment, with local-dev
//! defaults (see the `env_or` calls). Override every one of them for anything
//! beyond a developer machine — the defaults are not secrets.
//!
//!   NEXUS_METADATA_URL   the database to seed (same as the server's)
//!   ADMIN_EMAIL          admin login            (default admin@nexus.local)
//!   ADMIN_PASSWORD       admin password         (default change-me-admin)
//!   ADMIN_TENANT_SLUG    tenant slug + id       (default nexus)
//!   ADMIN_TENANT_NAME    tenant display name    (default Nexus)

use nexus_api::bootstrap::migrate_all;
use starter_auth_users::admin::create_admin;
use starter_auth_users::store::{
    MembershipRecord, PgTenantStore, PgUserStore, TenantRecord, TenantStore,
};
use starter_auth_users::Role;
use starter_authz::store::{PolicyStore, PostgresPolicyStore, StoredAssignment};
use starter_store_postgres::pool::connect;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), String> {
    let url = req("NEXUS_METADATA_URL")?;
    let email = env_or("ADMIN_EMAIL", "admin@nexus.local");
    let password = env_or("ADMIN_PASSWORD", "change-me-admin");
    let tenant_slug = env_or("ADMIN_TENANT_SLUG", "nexus");
    let tenant_name = env_or("ADMIN_TENANT_NAME", "Nexus");

    let pool = connect(&url).await.map_err(|e| format!("connect: {e}"))?;
    migrate_all(&pool).await?;

    let tenants = PgTenantStore::new(pool.clone());
    let users = PgUserStore::new(pool.clone());
    let policies = PostgresPolicyStore::new(pool);

    // Tenant. The slug doubles as the stable id so the admin's tenant_id is
    // predictable across runs and matches what RLS binds.
    let tenant = TenantRecord {
        id: tenant_slug.clone(),
        slug: tenant_slug.clone(),
        display_name: tenant_name,
        audit_allow_sample: None,
        parent_id: None,
    };
    match tenants.create_tenant(&tenant).await {
        Ok(()) => println!("created tenant '{tenant_slug}'"),
        Err(e) if is_conflict(&e.to_string()) => println!("tenant '{tenant_slug}' exists"),
        Err(e) => return Err(format!("create tenant: {e}")),
    }

    // Admin user. `create_admin` validates + argon2-hashes the password and
    // returns the new id; on a re-run the user already exists, so we look the id
    // back up to keep the membership + grant steps idempotent.
    let user_id = match create_admin(&users, &email, &password, Role::Admin).await {
        Ok(id) => {
            println!("created admin user '{email}'");
            id
        }
        Err(e) if is_conflict(&e.to_string()) => {
            println!("admin user '{email}' exists");
            lookup_user_id(tenants.pool().sqlx(), &email).await?
        }
        Err(e) => return Err(format!("create admin: {e}")),
    };

    // Bind the user to the tenant.
    let membership = MembershipRecord {
        tenant_id: tenant_slug.clone(),
        user_id: user_id.clone(),
        role: "admin".to_string(),
        email: None,
    };
    match tenants.add_member(&membership).await {
        Ok(()) => println!("added '{email}' to tenant '{tenant_slug}'"),
        Err(e) if is_conflict(&e.to_string()) => println!("membership exists"),
        Err(e) => return Err(format!("add member: {e}")),
    }

    // Global admin grant (authz). Subject = the user id; role = admin.
    let assignment = StoredAssignment {
        id: Uuid::new_v4().to_string(),
        subject: user_id,
        role: "admin".to_string(),
        created_by: "seed".to_string(),
    };
    match policies.insert_assignment(&assignment).await {
        Ok(()) => println!("granted admin role"),
        Err(e) if is_conflict(&e.to_string()) => println!("admin grant exists"),
        Err(e) => return Err(format!("insert assignment: {e}")),
    }

    // Default navigation tree (WS-13 §6): ensure one `route` node per built-in
    // static page so the tenant *has* a navigable sidebar. The nodes are seeded
    // structurally only — they are NOT granted to anyone by default.
    //
    // Access model (deliberate): an admin sees every node via the built-in
    // admin-all rule, so the full sidebar is visible to admins with no per-node
    // grant. A non-admin (team / user) sees *only* the nodes explicitly granted
    // to them — `GET /api/v1/nav` is access-filtered per node, so an ungranted
    // member starts with an empty sidebar. This is the share model the product
    // wants: visibility is opt-in, granted per team/user, never world-default.
    //
    // (Previously the seed wrote a `role:"*"` view grant on every node, making
    // the whole default sidebar world-visible to any member — including
    // admin-only pages like Access/Audit. That over-shared and is removed.)
    //
    // Reconciling (not just seed-if-empty) backfills routes added after the
    // tenant was first seeded — e.g. `insights` on an established tenant — so
    // re-running seed-admin surfaces new built-in pages to admins. Idempotent:
    // only missing routes are created.
    let seeded =
        nexus_store::nav_node::reconcile_default_routes(tenants.pool().sqlx(), &tenant_slug)
            .await
            .map_err(|e| format!("seed nav tree: {e}"))?;
    if seeded.is_empty() {
        println!("nav tree already complete");
    } else {
        println!(
            "reconciled default nav tree (+{} node(s); admins see all, non-admins by grant only)",
            seeded.len()
        );
    }

    println!("seed complete — login {email}");
    Ok(())
}

/// Read the user id back after a conflict, so re-runs stay idempotent.
async fn lookup_user_id(pool: &sqlx::PgPool, email: &str) -> Result<String, String> {
    sqlx::query_scalar::<_, String>("SELECT id FROM starter_auth_users_users WHERE email = $1")
        .bind(email)
        .fetch_one(pool)
        .await
        .map_err(|e| format!("lookup existing user: {e}"))
}

fn is_conflict(msg: &str) -> bool {
    let m = msg.to_lowercase();
    m.contains("exist") || m.contains("conflict") || m.contains("duplicate")
}

fn req(key: &str) -> Result<String, String> {
    std::env::var(key).map_err(|_| format!("{key} must be set"))
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}
