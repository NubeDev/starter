//! user goal — tool implementations.
//!
//! One file per verb (FILE-LAYOUT §2). This barrel re-exports the
//! verb modules and contains no logic of its own.

pub mod create;
pub mod delete;
pub mod disable;
pub mod enable;
pub mod list;
pub mod prefs_set;
pub mod role_set;
pub mod store;
pub mod tenant_assign;
