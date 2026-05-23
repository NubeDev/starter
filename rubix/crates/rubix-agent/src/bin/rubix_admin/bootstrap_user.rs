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
use starter_auth_users::store::{PgUserStore, UserStore};
use starter_auth_users::Role;
use starter_store_postgres::{migrate, pool::connect};
use tracing::{info, warn};

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

    let store = PgUserStore::new(pool);
    match create_admin(&store, &email, &password, Role::Admin).await {
        Ok(id) => {
            info!(
                target: "rubix.admin.bootstrap",
                user_id = %id,
                email = %email,
                role = "admin",
                "admin user created",
            );
            Ok(())
        }
        Err(AdminError::Conflict) => reconcile_existing(&store, &email).await,
        Err(e) => Err(anyhow!("create_admin failed: {e}")),
    }
}

/// On Conflict, re-read the existing row. Same email + admin role →
/// log and exit 0. Different role → hard error; the operator must
/// resolve manually rather than have the CLI silently escalate or
/// demote.
async fn reconcile_existing(store: &PgUserStore, email: &str) -> Result<()> {
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
        Ok(())
    } else {
        Err(anyhow!(
            "user {email} already exists with role {:?}; refusing to overwrite",
            existing.role
        ))
    }
}
