//! `/auth/oauth/*` routes.
//!
//! Phase 1c shipped `start` + `callback`. Phase 3 (this stage) adds
//! the link / unlink / list trio: `POST /auth/oauth/{provider}/link`,
//! `DELETE /auth/oauth/{provider}`, and `GET /auth/oauth/identities`.
//! All three sit behind a logged-in session; link + unlink also
//! enforce the standard double-submit CSRF cookie that
//! `starter-auth-users` mints. Only the callback `GET` is allowed to
//! skip CSRF — that handler uses the OAuth `state` parameter instead
//! (Hard rule R9).
//!
//! The module split mirrors the source SCOPE §"Repo layout"
//! one-file-per-route so adding a sixth route later is a new file.

mod callback;
mod link;
mod list;
mod router;
mod session_guard;
mod start;
mod state;
mod unlink;

pub use callback::{handler as callback_handler, CallbackQuery};
pub use link::{handler as link_handler, LinkRequest, LinkResponse};
pub use list::{handler as list_handler, IdentitiesResponse, IdentitySummary};
pub use router::oauth_router;
pub use start::{handler as start_handler, StartQuery};
pub use state::OAuthRoutesState;
pub use unlink::handler as unlink_handler;
