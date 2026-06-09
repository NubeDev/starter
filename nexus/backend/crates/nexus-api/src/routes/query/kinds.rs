//! `GET /api/v1/query/kinds` — list the registered query-kinds (WS-10).
//!
//! The query editor's kind picker reads this to offer the declarative queries a
//! panel can invoke by name. The handler exposes only each kind's descriptive
//! surface (name, description, datasource shape, params schema) from the registry
//! mounted in `AppState`; the kind's SQL is never sent to the client.

use axum::extract::State;
use axum::Json;
use nexus_spi::dto::query::{QueryKindList, QueryKindSummary};

use crate::state::AppState;

/// Return the registry's kinds, name-ordered (the registry is a `BTreeMap`, so
/// iteration is already sorted). A read-only catalogue — no datasource work.
#[utoipa::path(
    get,
    path = "/api/v1/query/kinds",
    tag = "query",
    operation_id = "list_query_kinds",
    responses(
        (status = 200, description = "Registered query-kinds", body = QueryKindList),
    ),
)]
pub async fn list_query_kinds(State(state): State<AppState>) -> Json<QueryKindList> {
    let kinds = state
        .kinds
        .iter()
        .map(|k| QueryKindSummary {
            name: k.name.clone(),
            description: k.description.clone(),
            datasource_kind: k.datasource_kind.clone(),
            params_schema: k.params_schema.clone(),
        })
        .collect();
    Json(QueryKindList { kinds })
}
