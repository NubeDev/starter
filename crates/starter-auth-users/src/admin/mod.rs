//! Programmatic admin operations. The CLI uses these; there is
//! deliberately **no unauthenticated HTTP path** for first-run admin
//! creation (see SCOPE.md "Bootstrap").

mod create_admin;

pub use create_admin::{create_admin, AdminError};
