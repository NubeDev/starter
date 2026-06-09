//! User / org preferences (WS-11) — the nexus integration of `starter-prefs`.
//!
//! nexus reuses the platform preference machinery wholesale: the `PgPrefsStore`
//! persistence, the three-layer `resolve(user, org, default)` resolver, and the
//! `ResolvedPreferences` wire shape. This module only wires those into nexus'
//! tenancy model — `workspace_id` IS the nexus `tenant_id`, the user layer is
//! keyed on `principal.subject`, and the [`resolver::NexusPrefsResolver`] adapter
//! lets the `starter-server` `Accept-Units` middleware resolve a caller's units
//! once per request. Storage isolation is route-pinned (see `1501_prefs.sql`),
//! not RLS-bound, because the reused store runs outside `tenant_tx`.

mod factory;
mod resolver;

pub use factory::{prefs_store, NexusPrefs};
pub use resolver::NexusPrefsResolver;
