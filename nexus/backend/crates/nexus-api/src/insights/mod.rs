//! Apply an optional post-query insight to a query result.
//!
//! This is the query-path seam for RW-06: after a query returns its rows (from
//! the push-down path, federation, or cache) and before the response is
//! serialized, an attached [`InsightRef`] runs the result frame through the
//! sandboxed insight engine. A request without an insight skips this entirely, so
//! the path is unchanged for everyone else.
//!
//! Resolution: an inline `script` runs as given; a stored `insight_id` is looked
//! up in the caller's tenant (RLS-scoped) and its script runs. A stored id needs
//! a tenant, so the dev `POST /query` shortcut (no tenant) supports inline scripts
//! only — a stored reference there is a clean error, not a panic.

mod apply;

pub use apply::apply_insight;
pub(crate) use apply::reshape;
