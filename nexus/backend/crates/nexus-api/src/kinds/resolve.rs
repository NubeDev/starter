//! The output of resolving a kind request: the kind's SQL plus its validated,
//! lowered params, ready for the store to bind and run.
//!
//! Separating resolution (nexus-api: file-backed registry, JSON-Schema) from
//! execution (nexus-store: binder + guards) keeps the kinds layer free of any
//! database concern and the store free of any file/manifest concern. The
//! dispatcher carries a `BoundKind` across that seam.

use std::collections::BTreeMap;

use nexus_store::ParamValue;

/// A resolved kind invocation: everything the store needs to bind and execute,
/// with the dispatch metadata the handler checks first.
#[derive(Debug, Clone)]
pub struct BoundKind {
    /// The kind's SQL template (still carries `$caller_tenant_id`, `$__time*`,
    /// and `$param` tokens — the store's binder expands them to bound args).
    pub sql: String,

    /// The validated, defaulted caller params, lowered to binder scalars.
    pub params: BTreeMap<String, ParamValue>,

    /// The datasource shape the kind targets; the dispatcher checks it against
    /// the bound datasource's `kind` before running.
    pub datasource_kind: String,

    /// An optional pinned datasource id; when set the kind only runs against it.
    pub datasource_binding: Option<String>,

    /// Tables the kind reads — the read-capability surface (and future cache
    /// invalidation key).
    pub tables: Vec<String>,
}
