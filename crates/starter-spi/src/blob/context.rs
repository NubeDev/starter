//! [`BlobContext`] — structured view of where a [`BlobRef`] lives
//! in the combinator stack.
//!
//! # Why this exists
//!
//! `starter-blob-axum`'s `blob_proxy_handler` takes a consumer-
//! supplied authz closure. The naive shape `Fn(&BlobRef, &Request)`
//! would force the closure to *re-parse* the namespace prefix
//! (e.g. `"project-7"`) out of an opaque `BlobRef` to decide
//! "does this viewer have access to project 7?". That leaks the
//! namespace scheme into authorization code and undermines both
//! B1 ("the store knows nothing about the consumer's domain") and
//! B2 (the `BlobRef` is supposed to be opaque).
//!
//! `BlobContext` is the structured value the proxy hands to the
//! authz closure instead: it carries the namespace prefix that the
//! combinator stack peeled off, so the consumer authorizes against
//! a parsed string they already understand rather than fishing it
//! out of an opaque handle.
//!
//! # How it is populated
//!
//! [`BlobStore::context_for`](super::BlobStore::context_for) has a
//! default impl returning an empty context. Combinators that
//! transform keys (today: `Namespaced`) override it to fill in
//! their slice of the prefix and delegate to the inner store, so
//! a nested `Namespaced("tenant-7", Namespaced("project-3", fs))`
//! reports both prefixes in order.
//!
//! Engines (memory, fs, s3, garage) do *not* override the default —
//! they have no domain prefix to contribute.

use super::blob_ref::BackendId;

/// Structured view of a [`BlobRef`](super::BlobRef)'s position in
/// the combinator stack, handed to consumer authz closures by the
/// `starter-blob-axum` proxy handler.
///
/// The struct is `#[non_exhaustive]` so future combinators (e.g. a
/// `TenantTagged` one) can add their own fields without a breaking
/// change.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct BlobContext {
    /// Namespace prefixes peeled off by `Namespaced` combinators,
    /// outermost first. For `Namespaced("tenant-7", Namespaced(
    /// "project-3", fs))` this is `["tenant-7", "project-3"]`.
    ///
    /// Empty when the store is a bare engine (no combinators), or
    /// the combinator stack contains no `Namespaced` layers.
    pub namespace_prefixes: Vec<String>,

    /// Backend id of the *innermost* engine — the one that
    /// actually holds the bytes. Useful for diagnostics and for
    /// authz policies that vary by backend (e.g. "deny GETs from
    /// the cold-tier mirror unless the user has admin").
    pub backend_id: Option<BackendId>,
}

impl BlobContext {
    /// Empty context. Engines reach for this in their default
    /// `context_for` impl; combinators reach for it and then push
    /// their prefix.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Prepend a namespace prefix. Used by `Namespaced` to fold its
    /// own prefix onto the context returned by its inner store, so
    /// the outermost layer sits at index 0.
    pub fn prepend_namespace(mut self, prefix: impl Into<String>) -> Self {
        self.namespace_prefixes.insert(0, prefix.into());
        self
    }

    /// Set [`BlobContext::backend_id`].
    pub fn with_backend_id(mut self, backend_id: BackendId) -> Self {
        self.backend_id = Some(backend_id);
        self
    }

    /// Convenience: the outermost namespace, if any.
    /// Equivalent to `self.namespace_prefixes.first()`.
    pub fn outer_namespace(&self) -> Option<&str> {
        self.namespace_prefixes.first().map(String::as_str)
    }
}
