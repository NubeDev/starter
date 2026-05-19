//! `/auth/oauth/*` routes.
//!
//! The two routes that ship in Phase 1c (this stage) are the start
//! redirect and the callback; `link`, `unlink`, and `list` land in
//! Phase 3 (stage 8). The module split here mirrors the source
//! SCOPE §"Repo layout" so adding those later is a new file each.

mod callback;
mod router;
mod start;
mod state;

pub use callback::{handler as callback_handler, CallbackQuery};
pub use router::oauth_router;
pub use start::{handler as start_handler, StartQuery};
pub use state::OAuthRoutesState;
