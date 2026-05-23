//! Cookie-session lifecycle. The persistence seam lives in
//! [`crate::store::SessionStore`]; this module wraps it with the
//! issue / lookup / revoke flow and exports the cookie name.

mod cookie;
mod issue;
mod revoke;
mod verify;

pub use cookie::{cookie_name, SESSION_COOKIE};
pub use issue::{issue, issue_for_tenant, IssuedSession, SessionError};
pub use revoke::revoke;
pub use verify::{
    verify_session, verify_session_with_extras, verify_session_with_teams,
    verify_session_with_teams_and_extras,
};
