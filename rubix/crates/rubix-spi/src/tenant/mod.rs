//! Tenant SPI \u{2014} async store trait + value type for the
//! `rubix_tenants` table that backs the
//! `rubix.tenant.{create,update,list,delete}` verb surface.
//!
//! Companion to [`crate::audit`] (same layering pattern). The
//! production impl is
//! [`rubix-store-postgres::tenants::PgRubixTenantStore`]; the
//! in-memory test fake + the [`Reversible`] glue live in
//! `rubix-tools::tenant::store` so this crate retains zero deps
//! on `sqlx`, `tokio`, or the verb dispatch layer per SCOPE R6.
//!
//! Note: a *separate* `TenantStore` lives in `starter-auth-users`
//! for the auth-side tenant directory. The rubix-side tenant is
//! the verb-surface concept; unifying them is out of scope.
//!
//! [`Reversible`]: starter_spi::changelog::Reversible

pub mod store;

pub use store::{TenantRow, TenantStore, TENANT_KIND};
