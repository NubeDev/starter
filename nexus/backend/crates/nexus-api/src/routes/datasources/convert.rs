//! Map the store's datasource record to the redacted wire DTOs.
//!
//! The record carries the tenant and key metadata; the DTOs expose only what a
//! client may see — never the secret. This is the one place the mapping lives so
//! every datasource route returns a consistent shape.

use nexus_spi::dto::datasource::{
    DatasourceDetail, DatasourceKind, DatasourceSummary, RedactedConnection,
};
use nexus_store::datasource::DatasourceRecord;

/// Coarse kind mapping. Unknown stored kinds surface as `postgres` for now —
/// the enum grows as connectors land.
fn kind_of(_stored: &str) -> DatasourceKind {
    DatasourceKind::Postgres
}

/// List view: identity + kind only.
pub fn to_summary(r: &DatasourceRecord) -> DatasourceSummary {
    DatasourceSummary {
        id: r.id,
        name: r.name.clone(),
        kind: kind_of(&r.kind),
    }
}

/// Detail view: adds the redacted connection (never the secret).
pub fn to_detail(r: &DatasourceRecord) -> DatasourceDetail {
    DatasourceDetail {
        id: r.id,
        name: r.name.clone(),
        kind: kind_of(&r.kind),
        connection: RedactedConnection {
            host: r.host.clone(),
            port: r.port as u16,
            database: r.database.clone(),
            user: r.db_user.clone(),
            has_secret: true,
        },
    }
}
