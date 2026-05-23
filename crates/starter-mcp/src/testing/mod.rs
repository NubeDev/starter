//! In-memory transport pair for tests. `feature = "testing"`.
//!
//! See [`in_memory`] for the paired client + server constructors that
//! drive the dispatch core through real serialised JSON-RPC frames —
//! the surface PR 3 (rubix MCP exposure) needs to assert against
//! without standing up HTTP. Spec lives in
//! `docs/design/starter-changes/README.md` (Phase 2b U2).

pub mod in_memory;

pub use in_memory::{pair, pair_with_principal, InMemoryClient, InMemoryServer};
