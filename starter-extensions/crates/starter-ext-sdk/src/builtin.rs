//! Builtin flavour entry-point glue (SCOPE R1: builtin).
//!
//! Builtin extensions are statically linked into the host. The host calls
//! a `register(&mut BuiltinTable)` function (emitted by the
//! [`register_static_table!`] macro) once per linked extension at
//! startup; each registration inserts a [`BuiltinEntry`] keyed by the
//! extension's id. The host then dispatches tool calls by id through the
//! same `ExtensionDispatch::dispatch_tool` interface the other two
//! flavours use.
//!
//! Builtin runs in the host's address space; capability declarations are
//! documentation only (SCOPE R6 "Builtin: not enforced. … trust-equivalent
//! to host code."). Operators who need isolation choose WASM or process.
//!
//! ## Why dispatch is closure-based here
//!
//! Each `requires!{}` invocation generates a different per-extension
//! `Ctx` newtype, and the proc-macro-generated `ExtensionDispatch` impl
//! is parameterised over that per-extension `Ctx`. A flat
//! `HashMap<ExtensionId, …>` cannot store both `Weather`'s dispatch path
//! and `Reminders`'s dispatch path under the same concrete generic. The
//! closure shape erases the per-extension Ctx while keeping the
//! `CtxInner` end of the shape uniform: the `register_static_table!`
//! expansion constructs the per-extension Ctx newtype inside its closure
//! body (`WeatherCtx::__from_inner(inner)`), then calls
//! `ExtensionDispatch::dispatch_tool` with that concrete type.
//!
//! [`register_static_table!`]: crate::register_static_table

use std::collections::HashMap;
use std::sync::Arc;

use starter_ext_spi::{ExtensionId, Result};

use crate::ctx::CtxInner;

/// Type alias for the closure each extension registers. Taking `CtxInner`
/// keeps the host's dispatch path Ctx-generic-erased; the closure body
/// (emitted by `register_static_table!`) re-wraps it as the
/// per-extension `Ctx` newtype before invoking the typed handler.
pub type BuiltinDispatchFn =
    dyn Fn(&str, &CtxInner, serde_json::Value) -> Result<serde_json::Value> + Send + Sync + 'static;

/// One linked-in extension's registration entry. Held inside
/// [`BuiltinTable`] keyed by `ExtensionId`.
pub struct BuiltinEntry {
    dispatch: Arc<BuiltinDispatchFn>,
    declared_tool_ids: &'static [&'static str],
}

impl BuiltinEntry {
    /// Build an entry from the closure `register_static_table!` emits.
    ///
    /// `declared_tool_ids` is the slice returned by the proc-macro's
    /// `ExtensionDispatch::declared_tool_ids()`; surfaced here so the
    /// host registry can enumerate routes without re-parsing the
    /// manifest.
    pub fn new<F>(declared_tool_ids: &'static [&'static str], dispatch: F) -> Self
    where
        F: Fn(&str, &CtxInner, serde_json::Value) -> Result<serde_json::Value>
            + Send
            + Sync
            + 'static,
    {
        Self {
            dispatch: Arc::new(dispatch),
            declared_tool_ids,
        }
    }

    /// Dispatch a tool call against this extension. Called by the host
    /// from inside its request-handling task.
    pub fn dispatch(
        &self,
        tool_id: &str,
        ctx: &CtxInner,
        params: serde_json::Value,
    ) -> Result<serde_json::Value> {
        (self.dispatch)(tool_id, ctx, params)
    }

    /// The tool ids declared in `block.yaml`, in declaration order.
    pub fn declared_tool_ids(&self) -> &'static [&'static str] {
        self.declared_tool_ids
    }
}

impl std::fmt::Debug for BuiltinEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BuiltinEntry")
            .field("declared_tool_ids", &self.declared_tool_ids)
            .finish_non_exhaustive()
    }
}

/// Host-side dispatch table: id → entry. Populated by calling each
/// linked extension's `register(&mut BuiltinTable)` (the function
/// `register_static_table!` emits).
#[derive(Default, Debug)]
pub struct BuiltinTable {
    entries: HashMap<ExtensionId, BuiltinEntry>,
}

impl BuiltinTable {
    /// Construct an empty table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert one entry. Caller (the generated `register` fn) supplies
    /// the id from `ExtensionMeta::id`; collisions panic — two linked
    /// extensions sharing an id is a build-time configuration error
    /// the operator must resolve before the host can start.
    pub fn insert(&mut self, id: ExtensionId, entry: BuiltinEntry) {
        if self.entries.contains_key(&id) {
            panic!(
                "starter-ext-sdk: duplicate builtin extension id {:?} — two linked crates \
                 declared the same `id:` in block.yaml",
                id.as_str()
            );
        }
        self.entries.insert(id, entry);
    }

    /// Look up a registered extension by id.
    pub fn get(&self, id: &ExtensionId) -> Option<&BuiltinEntry> {
        self.entries.get(id)
    }

    /// Iterate over registered ids in unspecified order. Used by the
    /// host registry to populate its records list.
    pub fn ids(&self) -> impl Iterator<Item = &ExtensionId> {
        self.entries.keys()
    }

    /// Number of registered extensions.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// `true` when no extensions have been registered.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_table_lookup_returns_none() {
        let table = BuiltinTable::new();
        let id = ExtensionId::new("com.acme.absent").unwrap();
        assert!(table.get(&id).is_none());
        assert!(table.is_empty());
    }

    #[test]
    fn insert_and_get_round_trip() {
        let mut table = BuiltinTable::new();
        let id = ExtensionId::new("com.acme.weather").unwrap();
        let entry = BuiltinEntry::new(&[], |_tool, _ctx, _params| {
            Ok(serde_json::json!({ "ok": true }))
        });
        table.insert(id.clone(), entry);
        assert_eq!(table.len(), 1);
        assert!(table.get(&id).is_some());
    }

    #[test]
    #[should_panic(expected = "duplicate builtin extension id")]
    fn duplicate_insert_panics() {
        let mut table = BuiltinTable::new();
        let id = ExtensionId::new("com.acme.weather").unwrap();
        table.insert(
            id.clone(),
            BuiltinEntry::new(&[], |_, _, _| Ok(serde_json::Value::Null)),
        );
        table.insert(
            id,
            BuiltinEntry::new(&[], |_, _, _| Ok(serde_json::Value::Null)),
        );
    }
}
