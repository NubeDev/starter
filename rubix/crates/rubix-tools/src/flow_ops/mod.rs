//! flow_ops goal — tool implementations.
//!
//! One file per verb (FILE-LAYOUT §2). This barrel re-exports the
//! verb modules and contains no logic of its own.

pub mod deploy;
pub mod validate;
pub mod lint;
pub mod list;
pub mod duplicate;
