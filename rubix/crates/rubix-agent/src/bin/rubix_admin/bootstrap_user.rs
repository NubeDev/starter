//! `rubix-admin bootstrap-user` — idempotent first-run admin creation.
//!
//! Contract: read `RUBIX_DSN`, open a Postgres pool, apply the
//! `starter_auth_users` migration source so the tables exist on a
//! fresh DB, then call `starter_auth_users::admin::create_admin`
//! against a `PgUserStore`. Conflict is success only when the
//! existing user has the same email and the admin role; a different
//! role is a hard error rather than a silent overwrite.
//!
//! See [docs/design/auth/README.md](../../../../docs/design/auth/README.md)
//! and
//! [docs/design/migrations/README.md](../../../../docs/design/migrations/README.md).

use anyhow::{anyhow, Result};
use clap::Args as ClapArgs;
use starter_auth_users::admin::{create_admin, AdminError};
use starter_auth_users::migration::postgres_migration_source;
use starter_auth_users::store::{
    MembershipRecord, PgTenantStore, PgUserStore, TenantStore, TenantStoreError, UserStore,
};
use starter_auth_users::Role;
use starter_store_postgres::{migrate, pool::connect};
use tracing::{info, warn};

/// The system tenant id every Admin user gets membership in. Matches
/// `BUNDLED_TENANT` in `rubix-spi/src/dashboard/store.rs` — bundled
/// resources (dashboards, flows) live under this tenant and the
/// session-binding rules surface them to Admins via the super-admin
/// sentinel `"*"`. Granting explicit membership here also lets a
/// non-`*` session backstop work if the super-admin path is ever
/// removed.
const SYSTEM_TENANT_ID: &str = "system";

/// CLI flags. `--password` / `--email` override the env vars when
/// both are present (CLI wins).
#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Admin email. Falls back to `RUBIX_BOOTSTRAP_EMAIL`.
    #[arg(long, env = "RUBIX_BOOTSTRAP_EMAIL")]
    email: Option<String>,
    /// Admin password. Falls back to `RUBIX_BOOTSTRAP_PASSWORD`.
    /// Never logged.
    #[arg(long, env = "RUBIX_BOOTSTRAP_PASSWORD", hide_env_values = true)]
    password: Option<String>,
}

pub async fn run(args: Args) -> Result<()> {
    let email = args
        .email
        .ok_or_else(|| anyhow!("missing admin email: pass --email or set RUBIX_BOOTSTRAP_EMAIL"))?;
    let password = args.password.ok_or_else(|| {
        anyhow!("missing admin password: pass --password or set RUBIX_BOOTSTRAP_PASSWORD")
    })?;
    let dsn = std::env::var("RUBIX_DSN")
        .map_err(|_| anyhow!("RUBIX_DSN unset; bootstrap-user requires a Postgres DSN"))?;

    let pool = connect(&dsn)
        .await
        .map_err(|e| anyhow!("connect to RUBIX_DSN: {e}"))?;

    // Idempotent on a fresh DB: chain the auth-users source through
    // the same runner the agent boot uses for changelog tables.
    migrate(&pool)
        .with_source(postgres_migration_source())
        .run()
        .await
        .map_err(|e| anyhow!("apply starter_auth_users migrations: {e}"))?;

    let store = PgUserStore::new(pool.clone());
    let tenants = PgTenantStore::new(pool);
    let user_id = match create_admin(&store, &email, &password, Role::Admin).await {
        Ok(id) => {
            info!(
                target: "rubix.admin.bootstrap",
                user_id = %id,
                email = %email,
                role = "admin",
                "admin user created",
            );
            id
        }
        Err(AdminError::Conflict) => reconcile_existing(&store, &email).await?,
        Err(e) => return Err(anyhow!("create_admin failed: {e}")),
    };
    grant_system_membership(&tenants, &user_id, &email).await
}

/// Idempotently grant the admin user membership in the `system`
/// tenant. The system tenant is seeded by migration
/// `0007_system_tenant.sql`; `add_member` is a no-op on conflict
/// at the PK so re-running `bootstrap-user` is safe.
async fn grant_system_membership(
    tenants: &PgTenantStore,
    user_id: &str,
    email: &str,
) -> Result<()> {
    let row = MembershipRecord {
        tenant_id: SYSTEM_TENANT_ID.to_string(),
        user_id: user_id.to_string(),
        role: "admin".to_string(),
        email: None,
    };
    match tenants.add_member(&row).await {
        Ok(()) => {
            info!(
                target: "rubix.admin.bootstrap",
                user_id = %user_id,
                email = %email,
                tenant_id = SYSTEM_TENANT_ID,
                "granted system-tenant admin membership",
            );
            Ok(())
        }
        // `PgTenantStore::add_member` maps a `(tenant_id, user_id)`
        // PK unique-violation onto `SlugConflict("<tenant>:<user>")`;
        // re-bootstrap of the same admin is a no-op.
        Err(TenantStoreError::SlugConflict(_)) => Ok(()),
        Err(e) => Err(anyhow!("grant system membership: {e}")),
    }
}

/// On Conflict, re-read the existing row. Same email + admin role →
/// log and return the user id (so the caller can still reconcile
/// memberships). Different role → hard error; the operator must
/// resolve manually rather than have the CLI silently escalate or
/// demote.
async fn reconcile_existing(store: &PgUserStore, email: &str) -> Result<String> {
    let existing = store
        .find_by_email(email)
        .await
        .map_err(|e| anyhow!("post-conflict lookup failed: {e}"))?
        .ok_or_else(|| {
            anyhow!("create_admin reported Conflict but no row matched email={email}")
        })?;

    if existing.role == Role::Admin {
        warn!(
            target: "rubix.admin.bootstrap",
            user_id = %existing.id,
            email = %email,
            role = "admin",
            "admin user already exists; bootstrap is a no-op",
        );
        Ok(existing.id)
    } else {
        Err(anyhow!(
            "user {email} already exists with role {:?}; refusing to overwrite",
            existing.role
        ))
    }
}
