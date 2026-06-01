//! Agent transport: a thin reqwest proxy to rubix-agent plus the
//! session/credential state the commands share. Barrel only.

pub mod client;
pub mod error;
pub mod session;
