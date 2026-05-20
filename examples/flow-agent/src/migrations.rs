//! Namespaced migration source for the flow-agent example. Lands in
//! `_sqlx_migrations_flow_agent`, separate from any starter-owned
//! migration sources a future consumer might compose in.

use starter_store_sqlite::MigrationSource;

static FLOW_AGENT: sqlx::migrate::Migrator = sqlx::migrate!("./migrations/flow_agent");

pub fn sources() -> [MigrationSource; 1] {
    [MigrationSource {
        name: "flow_agent",
        migrator: &FLOW_AGENT,
    }]
}
