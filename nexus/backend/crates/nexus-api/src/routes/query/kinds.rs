//! `GET /api/v1/query/kinds` — list the available query-kinds (WS-10).
//!
//! The query editor's kind picker reads this to offer the declarative queries a
//! panel can invoke by name. The catalogue is two-source (§4.5c): the built-in
//! file pack (global, all tenants) plus the caller's own tenant-authored kinds
//! from the metadata DB. The handler exposes only each kind's descriptive surface
//! (name, description, datasource shape, params schema); the kind's SQL is never
//! sent to the client. An unauthenticated caller sees the file pack alone.

use axum::extract::State;
use axum::{Extension, Json};
use nexus_spi::dto::query::{QueryKindList, QueryKindSummary};
use starter_spi::auth::Principal;

use crate::middleware::tenant::tenant_of;
use crate::state::AppState;

/// Return the file-pack kinds (the registry is a `BTreeMap`, so already sorted)
/// unioned with the caller's tenant-authored kinds. A read-only catalogue — no
/// datasource work. A DB read failure or an absent tenant degrades to the file
/// pack alone rather than failing the picker.
#[utoipa::path(
    get,
    path = "/api/v1/query/kinds",
    tag = "query",
    operation_id = "list_query_kinds",
    responses(
        (status = 200, description = "Available query-kinds", body = QueryKindList),
    ),
)]
pub async fn list_query_kinds(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
) -> Json<QueryKindList> {
    let mut kinds: Vec<QueryKindSummary> = state
        .kinds
        .iter()
        .map(|k| QueryKindSummary {
            name: k.name.clone(),
            description: k.description.clone(),
            datasource_kind: k.datasource_kind.clone(),
            params_schema: k.params_schema.clone(),
        })
        .collect();

    // Append the caller's tenant-authored kinds. A file kind always shadows a
    // same-named DB kind (the create handler rejects that collision, so this is
    // belt-and-braces); skip any DB name already present, then re-sort.
    if let Ok(tenant) = tenant_of(&principal) {
        match nexus_store::query_kind::list(&state.metadata, &tenant).await {
            Ok(rows) => {
                for r in rows {
                    if kinds.iter().any(|k| k.name == r.name) {
                        continue;
                    }
                    kinds.push(QueryKindSummary {
                        name: r.name,
                        description: r.description,
                        datasource_kind: r.datasource_kind,
                        params_schema: r.params_schema,
                    });
                }
                kinds.sort_by(|a, b| a.name.cmp(&b.name));
            }
            Err(e) => tracing::warn!(error = %e, "failed to list tenant query-kinds"),
        }
    }

    Json(QueryKindList { kinds })
}
