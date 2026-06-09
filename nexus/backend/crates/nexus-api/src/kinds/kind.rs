//! A loaded, validated query-kind: its SQL, its compiled params schema, and the
//! metadata the dispatcher needs to bind and run it.
//!
//! A `QueryKind` is immutable once loaded. It holds the raw SQL (the binder
//! rewrites it per request), the JSON Schema document its params validate
//! against, and the declared `tables` / `datasource_kind` the dispatcher checks
//! before running. Defaults from the schema are applied to caller params here.

use serde_json::Value;

/// One registered query-kind. The dispatcher resolves a request's `kind` name to
/// this, validates the caller params against [`QueryKind::params_schema`], binds
/// them plus host tokens through the shared binder, and runs the SQL.
#[derive(Debug, Clone)]
pub struct QueryKind {
    /// Reverse-DNS id a request invokes.
    pub name: String,

    /// The raw SQL template. Carries `$caller_tenant_id`, `$__time*` macros, and
    /// `$param` references — all bound by the shared binder, never inlined.
    pub sql: String,

    /// The JSON Schema document for this kind's params. Used both for validation
    /// and to read declared defaults.
    pub params_schema: Value,

    /// The datasource shape this kind targets (e.g. `postgres`).
    pub datasource_kind: String,

    /// Tables the kind reads. The lint guaranteed each tenant-scoped table here
    /// is guarded by `$caller_tenant_id` in the SQL.
    pub tables: Vec<String>,

    /// Optional pinned datasource id; `None` means any datasource of
    /// `datasource_kind` the caller can view.
    pub datasource_binding: Option<String>,

    /// Optional human description for the picker UI.
    pub description: Option<String>,
}

impl QueryKind {
    /// The names of the params this kind's schema declares, in document order.
    /// The lint uses this to reject SQL that references an undeclared `$param`.
    pub fn declared_params(&self) -> Vec<String> {
        self.params_schema
            .get("properties")
            .and_then(Value::as_object)
            .map(|props| props.keys().cloned().collect())
            .unwrap_or_default()
    }
}
