//! User SPI \u{2014} async store trait + value type for the
//! `rubix_users` table that backs the
//! `rubix.user.{create,list,disable,enable,role.set,prefs.set,tenant.assign}`
//! verb surface.
//!
//! Companion to [`crate::tenant`] (same layering pattern). The
//! production impl is
//! [`rubix-store-postgres::users::PgUserAdminStore`]; the
//! in-memory test fake + the [`Reversible`] glue live in
//! `rubix-tools::user::store` so this crate retains zero deps
//! on `sqlx`, `tokio`, or the verb dispatch layer per SCOPE R6.
//!
//! [`Reversible`]: starter_spi::changelog::Reversible

pub mod store;

pub use store::{UserAdminStore, UserRow, USER_KIND};
