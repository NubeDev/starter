//! Migration sources for the minimal example.
//!
//! Two namespaced sources: `starter_auth_token` (shipped by the auth
//! crate) and `app` (this example's own table). Both apply via the
//! single `starter_store_sqlite::migrate` runner so version counters
//! never collide.

use starter_store_sqlite::MigrationSource;

static AUTH_TOKEN: sqlx::migrate::Migrator =
    sqlx::migrate!("../../crates/starter-auth-token/migrations/starter_auth_token");

static APP: sqlx::migrate::Migrator = sqlx::migrate!("./migrations/app");

/// All migration sources this binary applies, in the order the runner
/// should walk them.
pub fn sources() -> [MigrationSource; 2] {
    [
        MigrationSource {
            name: "starter_auth_token",
            migrator: &AUTH_TOKEN,
        },
        MigrationSource {
            name: "app",
            migrator: &APP,
        },
    ]
}
