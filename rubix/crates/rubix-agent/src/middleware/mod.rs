//! Cross-cutting middleware layered onto the rubix HTTP surface.
//!
//! One verb-per-file: each module exports a single `*_layer` /
//! `with_*` helper the binary composes onto its router tree. No
//! domain logic lives here — these are wrappers over upstream
//! middleware crates plus rubix-specific extraction (path → tool
//! id, principal → actor, etc.). See
//! [docs/design/audit/](../../docs/design/audit/README.md).

pub mod authz_gate;
pub mod changelog;

pub use authz_gate::gate_tools;
pub use changelog::{changelog_layer, ChangelogState};
