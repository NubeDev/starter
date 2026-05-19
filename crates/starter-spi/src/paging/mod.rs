//! Cursor-based paging primitives. Used by every list endpoint in
//! the starter ecosystem so clients see one consistent shape.
//!
//! # `Repository<T>` derive (SCOPE open question 1) — deferred to v0.2
//!
//! The original SCOPE flirted with a `#[derive(Repository)]` macro
//! emitting CRUD + paging + optimistic-locking on top of the
//! `starter-store-*` crates. Deferred deliberately:
//!
//! 1. **No consumer to design against.** The shipped store crates
//!    (`starter-store-sqlite`, `starter-store-postgres`) already expose
//!    the migration runner, connection helpers, and cursor codec
//!    consumers need. A derive would freeze a contract before a real
//!    repository surface has been used in anger.
//! 2. **Hand-written queries are fine at this volume.** `sqlx` already
//!    provides compile-time-checked SQL; the derive would only save a
//!    handful of lines per entity while taking on a permanent
//!    maintenance + proc-macro debugging tax.
//! 3. **Scope creep risk.** Past attempts grew from "CRUD + paging"
//!    to include filtering, soft-delete, audit columns, event hooks…
//!    Better to wait until a consumer says "I keep writing this same
//!    code" and let the derive shape follow the pain.
//!
//! When it does land, the recommended scope is **exactly**:
//! `find_by_id`, `list` (with [`Page`] + [`Cursor`]), `insert`,
//! `update` (with optimistic version bump), `delete`. Nothing more.

mod cursor;
mod page;

pub use cursor::Cursor;
pub use page::Page;
