//! Map the store's datasource record to the redacted wire DTOs.
//!
//! The record carries the tenant and key metadata; the DTOs expose only what a
//! client may see — never the secret. This is the one place the mapping lives so
//! every datasource route returns a consistent shape.

use nexus_spi::dto::datasource::{
    DatasourceDetail, DatasourceKind, DatasourceSummary, RedactedConnection,
};
use nexus_store::datasource::DatasourceRecord;

/// The stored string for a wire kind. The store column is the connector
/// selector the engine builders key on, so the mapping is explicit, not a
/// `Debug` of the enum. A new connector is one new arm here and in [`kind_of`].
pub fn kind_to_stored(kind: DatasourceKind) -> &'static str {
    match kind {
        DatasourceKind::Postgres => "postgres",
        DatasourceKind::Mqtt => "mqtt",
        DatasourceKind::Zenoh => "zenoh",
    }
}

/// The wire kind for a stored string. An unrecognized value (e.g. a kind written
/// by a newer server, then downgraded) surfaces as `postgres` rather than
/// failing the read — the redacted view stays available even if the kind is
/// ahead of this binary.
fn kind_of(stored: &str) -> DatasourceKind {
    // Only `postgres` is wired today, so every stored value maps to it. When a
    // second connector lands this becomes a real `match stored { "mqtt" => …,
    // _ => Postgres }` — the fallback keeps a read from failing on an unknown
    // kind written by a newer server.
    let _ = stored;
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
