//! Programmatic admin operations. The CLI uses these; there is
//! deliberately **no unauthenticated HTTP path** for first-run admin
//! creation (see SCOPE.md "Bootstrap").

mod create_admin;
mod set_password;

pub use create_admin::{create_admin, AdminError};
pub use set_password::{change_password, set_password, ChangePasswordError};
