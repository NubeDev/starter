//! Top-level router builder. The consumer calls [`sdui_router`]
//! with a configured [`crate::SduiState`] and merges the resulting
//! `Router<S>` into their own app.

use axum::routing::{get, post};
use axum::Router;

use crate::state::SduiState;

/// Build the SDUI router rooted at `/api/v1/ui`. Mounts:
///
/// - `POST /api/v1/ui/resolve`
/// - `POST /api/v1/ui/action`
/// - `GET  /api/v1/ui/table`
///
/// `S` is the outer app's state type — the router is parameterised
/// over it because axum requires `Router<S>` for `merge`. The
/// state extractor used by these routes is the [`SduiState`]
/// stored in a nested `Router::with_state` call.
pub fn sdui_router<S>(state: SduiState) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    let inner: Router<SduiState> = Router::new()
        .route("/api/v1/ui/resolve", post(crate::routes::resolve::handler))
        .route("/api/v1/ui/action", post(crate::routes::action::handler))
        .route("/api/v1/ui/table", get(crate::routes::table::handler));
    inner.with_state(state)
}
