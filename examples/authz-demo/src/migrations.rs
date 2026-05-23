//! Three namespaced migration sources: auth-users tables (users,
//! sessions, tokens), authz tables (assignments, rules), and the
//! demo-owned `reports` table.

use starter_store_sqlite::MigrationSource;

static AUTH_USERS: sqlx::migrate::Migrator =
    sqlx::migrate!("../../crates/starter-auth-users/migrations/starter_auth_users");

static AUTHZ: sqlx::migrate::Migrator =
    sqlx::migrate!("../../crates/starter-authz/migrations/starter_authz_sqlite");

static REPORTS: sqlx::migrate::Migrator = sqlx::migrate!("./migrations/reports");

pub fn sources() -> [MigrationSource; 3] {
    [
        MigrationSource {
            name: "starter_auth_users",
            migrator: &AUTH_USERS,
        },
        MigrationSource {
            name: "starter_authz",
            migrator: &AUTHZ,
        },
        MigrationSource {
            name: "reports",
            migrator: &REPORTS,
        },
    ]
}
