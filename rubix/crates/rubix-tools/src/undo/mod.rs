//! undo goal — tool implementations.
//!
//! One file per verb (FILE-LAYOUT §2). This barrel re-exports the
//! verb modules and contains no logic of its own.

pub mod dispatch;
pub mod last;
pub mod redo;
