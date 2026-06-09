//! Tenant-scoped datasource persistence with envelope-encrypted secrets.
//!
//! Reads return a redacted [`DatasourceRecord`] (no secret); the secret is
//! recovered only by [`open_secret`], at connection-build time, audited. Every
//! function runs inside a tenant-bound transaction so RLS isolates the rows.

mod decrypt;
mod delete;
mod fetch;
mod insert;
pub mod postgres;
mod record;
pub mod secret;
mod update;

use uuid::Uuid;

pub use decrypt::open_secret;
pub use delete::delete;
pub use fetch::{get, list};
pub use insert::insert;
pub use record::{DatasourcePatch, DatasourceRecord, NewDatasource};
pub use secret::{Envelope, SealedSecret, SecretError};
pub use update::update;

/// The cache key for a built datasource pool: the immutable datasource id within
/// its tenant (R5). Tenant-qualified so two tenants' equal-id rows can never
/// collide on one node. Kind-agnostic — every connector that holds a pool keys it
/// the same way.
pub fn pool_key(tenant_id: &str, id: Uuid) -> String {
    format!("{tenant_id}/{id}")
}
