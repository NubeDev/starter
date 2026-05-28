//! Team SPI \u{2014} async store trait + value types for the
//! `rubix_teams` table that backs the
//! `rubix.team.{create,update,delete,member.assign,member.unassign}`
//! verb surface.
//!
//! Companion to [`crate::user`] / [`crate::tenant`] (same
//! layering pattern). The production impl is
//! `rubix-store-postgres::teams::PgTeamAdminStore`; the
//! in-memory test fake + the [`Reversible`] glue live in
//! `rubix-tools::team::store` so this crate retains zero deps
//! on sqlx/tokio runtime logic (SCOPE R5/R6).

pub mod store;

pub use store::{TeamAdminStore, TeamPatch, TeamRow, TEAM_KIND};
