//! HTTP-side wiring for the `Authenticator` trait from
//! [`starter_spi::auth`].
//!
//! - [`with_principal`] extracts the credential from the request
//!   (Authorization header, falling back to session cookie), runs
//!   `Authenticator::verify`, and inserts the resulting `Principal`
//!   as a request extension. Routes that need authentication mount
//!   this once.
//! - [`with_role`] / [`with_scope`] enforce role / scope on the
//!   `Principal` extension.
//!
//! Layer order matters. `with_principal` must be the **outermost**
//! layer so it runs first on the incoming request and inserts the
//! extension before any guard checks it:
//!
//! ```ignore
//! let protected = with_principal(
//!     with_role(routes, Role::Admin),
//!     authenticator,
//! );
//! ```
//!
//! Reading the wrap order outside-in matches the request flow:
//! `with_principal` → `with_role` → the inner route.

mod anonymous_layer;
mod principal_layer;
mod require_role;
mod require_scope;

pub use anonymous_layer::{local_operator, with_anonymous_principal};
pub use principal_layer::with_principal;
pub use require_role::with_role;
pub use require_scope::with_scope;
