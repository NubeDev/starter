//! Message-catalogue lookup trait for the `$msg.<key>` binding source
//! (G6). The host supplies an implementation backed by whatever
//! catalogue it ships (`starter-i18n::MessageBundle`, an in-memory
//! `HashMap`, a database table, …). The default
//! [`NullBag`] returns `None` for every key — useful as a placeholder
//! in tests and contexts that don't need i18n.

use serde_json::Value as JsonValue;

/// Lookup a localised message by key under a given locale.
///
/// Implementations should be pure / side-effect free for the
/// duration of a resolve.
pub trait MessageBag {
    /// Resolve `key` for `locale`. Returns `None` when the key is
    /// not present in the catalogue for that locale (the evaluator
    /// then surfaces `BindingError::UnknownMessage` unless the
    /// binding is marked Optional via the `?` qualifier).
    fn lookup(&self, key: &str, locale: &str) -> Option<JsonValue>;
}

/// No-op catalogue — every lookup returns `None`. Default for
/// [`crate::EvalContext::new`].
#[derive(Debug, Default, Clone, Copy)]
pub struct NullBag;

impl MessageBag for NullBag {
    fn lookup(&self, _key: &str, _locale: &str) -> Option<JsonValue> {
        None
    }
}
