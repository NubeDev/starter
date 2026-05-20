//! Test seams. Trivial `PolicyEngine` impls and a hand-built
//! engine for tests of consumer routes that want to assert "this
//! route is gated by permission X" without spinning up the full
//! policy loader.
//!
//! Re-exported under `starter_authz::testing` so consumer crates
//! reach for `use starter_authz::testing::{AllowAll, DenyAll};`.

use async_trait::async_trait;

use starter_spi::auth::Principal;
use starter_spi::authz::{Decision, PolicyEngine, ResourceRef};

/// Always allows. Used by unit tests that don't care about authz.
#[derive(Debug, Default, Clone, Copy)]
pub struct AllowAll;

#[async_trait]
impl PolicyEngine for AllowAll {
    async fn check(&self, _: &Principal, _: &str, _: &ResourceRef) -> Decision {
        Decision::allow()
    }
}

/// Always denies (`reason = "test_deny"`). Used by tests that
/// assert "this route is gated" — flip the engine to `DenyAll`
/// and confirm the route now returns `403`.
#[derive(Debug, Default, Clone, Copy)]
pub struct DenyAll;

#[async_trait]
impl PolicyEngine for DenyAll {
    async fn check(&self, _: &Principal, _: &str, _: &ResourceRef) -> Decision {
        Decision::deny("test_deny")
    }
}
