//! Namespaced migration source for the flow-agent example. Lands in
//! `_sqlx_migrations_flow_agent`, separate from any starter-owned
//! migration sources a future consumer might compose in.
//!
//! Composes the `agent_sessions` schema shipped by
//! `starter-store-sqlite` (DOCS/agent/MEMORY.md Phase M-A) so the
//! page-builder route can persist turns + artifacts under each
//! session. Critically we do NOT pull in the full
//! `FLOW_MIGRATION_SOURCE` here — the flow-agent owns its own
//! `runs` table with a different schema, and the starter-flow
//! migrations would collide.

use starter_store_sqlite::flow::AGENT_SESSION_MIGRATION_SOURCE;
use starter_store_sqlite::MigrationSource;

static FLOW_AGENT: sqlx::migrate::Migrator = sqlx::migrate!("./migrations/flow_agent");

pub fn sources() -> [MigrationSource; 2] {
    [
        MigrationSource {
            name: "flow_agent",
            migrator: &FLOW_AGENT,
        },
        AGENT_SESSION_MIGRATION_SOURCE,
    ]
}
