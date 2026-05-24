//! Library surface of the rubix-agent binary.
//!
//! Exposes the boot-time wiring modules so integration tests under
//! `tests/` (and a future `rubix-admin` sibling binary that wants to
//! drive the same registries in-process) can reach
//! [`boot::mcp::build_flow_registry`] without re-implementing it.
//!
//! Pure barrel — no logic lives here. See
//! [docs/design/agent/](../../docs/design/agent/README.md) for the
//! boot order this crate composes.

pub mod boot;
pub mod health;
pub mod middleware;
pub mod registry;
pub mod routes;
