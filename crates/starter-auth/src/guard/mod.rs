//! Route-guard middleware factories. The consumer applies these
//! per-route or per-router:
//!
//! ```ignore
//! Router::new()
//!     .route("/items", get(list))
//!     .route("/items", post(create).layer(require_role(Role::Writer)))
//!     .route("/items/:id", delete(remove).layer(require_role(Role::Admin)))
//! ```

mod require_role;
mod require_scope;

pub use require_role::require_role;
pub use require_scope::require_scope;
