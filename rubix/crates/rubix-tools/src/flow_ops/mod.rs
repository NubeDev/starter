//! flow_ops goal — tool implementations.
//!
//! One file per verb (FILE-LAYOUT §2). This barrel re-exports the
//! verb modules and contains no logic of its own.

pub mod deploy;
pub mod duplicate;
pub mod lint;
pub mod list;
pub mod store;
pub mod validate;
