//! Authorization seam. The traits and value types live here (in
//! `spi`) so REST, MCP, and any future transport share one authz
//! abstraction. Concrete engines (RBAC, DB-backed, Casbin) ship in
//! `starter-authz`. See `DOCS/auth/authz/SCOPE.md`.
//!
//! Authz runs *after* whichever `Authenticator` produced the
//! [`crate::auth::Principal`]; it never authenticates. The trait
//! seam (`PolicyEngine`) consumes a `Principal` and decides whether
//! that principal may perform an `action` on an `object`.

mod decision;
mod engine;
mod registry;

pub use decision::{Decision, ResourceRef};
pub use engine::{NoopPolicyEngine, PolicyEngine};
pub use registry::{Ownership, ResourceRegistry, ResourceSpec};
