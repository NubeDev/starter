//! Agent routes: CRUD for agent configurations, plus session create/list/get and
//! the per-session SSE event feed.
//!
//! Settings/CRUD are JSON over Bearer auth like every other resource. Starting a
//! session returns a signed token; the SSE feed at `…/events?token=…` is the only
//! query-string-authed route here, because a browser `EventSource` cannot set
//! headers (same pattern as live streams).

pub mod convert;
pub mod create;
pub mod create_session;
pub mod delete;
pub mod events;
pub mod get;
pub mod get_session;
pub mod list;
pub mod list_sessions;
pub mod update;

use axum::routing::get as http_get;
use axum::Router;

use crate::state::AppState;

/// The `/api/v1/agents` surface.
pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/agents",
            http_get(list::list_agents).post(create::create_agent),
        )
        .route(
            "/api/v1/agents/{id}",
            http_get(get::get_agent)
                .put(update::update_agent)
                .delete(delete::delete_agent),
        )
        .route(
            "/api/v1/agents/{id}/sessions",
            http_get(list_sessions::list_agent_sessions)
                .post(create_session::create_agent_session),
        )
        .route(
            "/api/v1/agents/sessions/{id}",
            http_get(get_session::get_agent_session),
        )
        .route(
            "/api/v1/agents/sessions/{id}/events",
            http_get(events::subscribe_agent_session),
        )
}
