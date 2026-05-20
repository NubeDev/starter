//! One file per endpoint family. Each module hangs methods off
//! `impl Client` for its endpoints — keeps the surface readable and
//! lets AI loads pull "the health endpoint" cleanly.

mod auth;
mod health;
mod openapi;
mod prefs;

pub use auth::{LoginRequest, MeResponse};
pub use prefs::{UnitsQuantity, UnitsResponse};
