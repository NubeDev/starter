//! Dashboard SPI — async store trait + value types for the
//! Goal-1 SDUI page table.
//!
//! Lives next to (but not under) [`crate::dto::dashboard`] because
//! the DTOs are wire-shapes for the `rubix.dashboard.*` tool surface
//! while this module is the host-side contract the page resolver
//! and the `rubix.dashboard.*` tool bodies dispatch through.
//!
//! See `rubix/docs/scope/dashboards/01-storage.md` for the rationale
//! (option B — rubix-owned PG table) and the per-verb file layout
//! the PG implementation follows under
//! [`rubix-store-postgres::dashboards`].
//!
//! Zero deps on `sqlx` per SCOPE R6; the PG impl owns the SQL.

pub mod store;

pub use store::{
    DashboardRevision, DashboardStore, DashboardStoreError, InsertOutcome, ListFilter,
    NewRevision, BUNDLED_PRINCIPAL, BUNDLED_TENANT,
};
