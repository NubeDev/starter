//! Domain-shaped errors. Never HTTP-shaped — transports map these
//! to status codes at their own boundaries.
//!
//! One file per error concept lives in this module. The barrel below
//! is re-export only.

mod kind;
mod result;

pub use kind::Error;
pub use result::Result;
