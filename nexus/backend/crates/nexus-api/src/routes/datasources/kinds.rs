//! `GET /api/v1/datasources/kinds` — list the registered datasource-kinds (WS-08b).
//!
//! LAYER: transport (REST). Extract → call domain → shape DTO → return.
//! No SQL, no business predicates, no cross-resource walks here.
//! See docs/design/layering/.
//!
//! The per-kind config form reads this to render the right inputs for the
//! connector a user picks (and to label its secret fields write-only). The
//! handler exposes only each datasource-kind's descriptive surface from the
//! registry mounted in `AppState`; nothing about a connector's internals or any
//! stored secret crosses the wire.

use axum::extract::State;
use axum::Json;
use nexus_spi::dto::datasource::{DatasourceKindList, DatasourceKindSummary};

use crate::datasource_kinds::{Surface, TestSpec};
use crate::state::AppState;

/// Return the datasource-kind registry's entries, name-ordered (the registry is a
/// `BTreeMap`, so iteration is already sorted). A read-only catalogue.
#[utoipa::path(
    get,
    path = "/api/v1/datasources/kinds",
    tag = "datasources",
    operation_id = "list_datasource_kinds",
    responses(
        (status = 200, description = "Registered datasource-kinds", body = DatasourceKindList),
    ),
)]
pub async fn list_datasource_kinds(State(state): State<AppState>) -> Json<DatasourceKindList> {
    let kinds = state
        .datasource_kinds
        .iter()
        .map(|k| DatasourceKindSummary {
            name: k.name.clone(),
            surface: surface_label(k.surface).to_string(),
            config_schema: k.config_schema.clone(),
            secret_fields: k.secret_fields.clone(),
            test_mode: test_mode_label(&k.test).to_string(),
            dialect: k.dialect.clone(),
            description: k.description.clone(),
        })
        .collect();
    Json(DatasourceKindList { kinds })
}

/// The wire label for a connector's query surface.
fn surface_label(surface: Surface) -> &'static str {
    match surface {
        Surface::Query => "query",
        Surface::Stream => "stream",
    }
}

/// The wire label for how a connector is tested before save.
fn test_mode_label(test: &TestSpec) -> &'static str {
    match test {
        TestSpec::Query { .. } => "query",
        TestSpec::Connect => "connect",
    }
}
