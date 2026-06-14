//! Dashboard-variable routes (WS-02). Variables are dashboard-scoped, so the
//! collection lives under a dashboard slug; an individual variable is addressed
//! by its immutable id. Authorization keys on the owning dashboard's grant
//! (view to list, edit to mutate); tenant scoping comes from the principal and
//! RLS enforces it.

mod convert;
pub mod create;
pub mod delete;
pub mod list;
pub mod update;

use axum::routing::{get, patch};
use axum::Router;

use crate::state::AppState;

/// The `/api/v1/.../variables` surface: list/create under a dashboard, and
/// update/delete by variable id.
pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/dashboards/{slug}/variables",
            get(list::list_variables).post(create::create_variable),
        )
        .route(
            "/api/v1/variables/{id}",
            patch(update::update_variable).delete(delete::delete_variable),
        )
}
