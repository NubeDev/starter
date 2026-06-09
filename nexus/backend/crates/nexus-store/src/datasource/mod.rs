//! Tenant-scoped datasource persistence with envelope-encrypted secrets.
//!
//! Reads return a redacted [`DatasourceRecord`] (no secret); the secret is
//! recovered only by [`open_secret`], at connection-build time, audited. Every
//! function runs inside a tenant-bound transaction so RLS isolates the rows.

mod decrypt;
mod delete;
mod fetch;
mod insert;
mod record;
pub mod secret;
mod update;

pub use decrypt::open_secret;
pub use delete::delete;
pub use fetch::{get, list};
pub use insert::insert;
pub use record::{DatasourcePatch, DatasourceRecord, NewDatasource};
pub use secret::{Envelope, SealedSecret, SecretError};
pub use update::update;
