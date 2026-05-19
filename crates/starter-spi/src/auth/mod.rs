//! Authentication seam. The trait lives here (in `spi`) so REST, MCP,
//! and any future transport share one auth abstraction. See SCOPE.md
//! "Authenticator async shape" — open question on whether `verify`
//! should return claims or a richer `Principal`.

mod authenticator;
mod principal;
mod role;
mod scope;

pub use authenticator::Authenticator;
pub use principal::Principal;
pub use role::Role;
pub use scope::Scope;
