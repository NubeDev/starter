//! Warehouse-read capability — shared wire types.
//!
//! Per [`docs/scope/extensions-north-star`](../../../../rubix/docs/scope/extensions-north-star/README.md)
//! row 2, the `warehouse_read` capability lets an extension issue
//! **named-template** queries against the host's time-series store.
//! Server-defined templates only — the handle is not a SQL gateway.
//! Templates have a name, a typed parameter schema, an allowed-tables
//! list, and an opaque SQL body the host (not the extension) authored.
//!
//! This module ships the **metadata types** every layer shares:
//!
//! - [`Row`] — one tuple yielded by a query, opaque JSON columns.
//! - [`TemplateSpec`] — a template's catalog entry. The host owns the
//!   registry; the extension only consults specs via the handle
//!   (lookup by name).
//! - [`WarehouseReadRequest`] — the wire shape an extension sends
//!   when invoking the `warehouse.query` host method.
//!
//! The capability *handle* lives in
//! [`starter-ext-sdk::ctx::WarehouseReadHandle`](../../../starter-ext-sdk/src/ctx.rs)
//! — there is no executor or registry in this crate (zero runtime
//! logic per SCOPE R2). Host-side template storage / execution lives
//! in `starter-ext-host` and `rubix-agent`.

use serde::{Deserialize, Serialize};

/// One row yielded by a warehouse query. Opaque JSON object — the
/// kernel does not interpret column types; the template's documented
/// schema is the contract. Empty maps are permitted but rare in
/// practice (Timescale projections always emit at least one column).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Row(pub serde_json::Map<String, serde_json::Value>);

impl Row {
    /// Construct from a JSON object literal.
    pub fn from_map(m: serde_json::Map<String, serde_json::Value>) -> Self {
        Self(m)
    }

    /// Borrow the underlying map.
    pub fn as_map(&self) -> &serde_json::Map<String, serde_json::Value> {
        &self.0
    }
}

/// A template's catalog entry — name, parameter schema, allowed
/// tables, and the SQL body the host registered.
///
/// Specs are stored in the host's `TemplateRegistry`. Extensions
/// see specs only as the result of a metadata lookup; they never
/// construct one (templates contributed by an installed extension
/// arrive through `contributes.warehouse_templates[]` and are parsed
/// at host load time per row 3 of the critical path).
///
/// The SQL body is **not** executed by anything in this crate. It
/// is captured here so the registry is a single audit surface
/// (`tables` field aside, the SQL is the source of truth for what
/// the template actually reads).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TemplateSpec {
    /// Globally-unique template name. Host-builtin names live in a
    /// flat namespace; extension-contributed names are conventionally
    /// prefixed by the extension id to avoid collisions.
    pub name: String,

    /// Parameter schema as a JSON Schema fragment. The host validates
    /// inbound `params` against this before binding. v1 ships with
    /// the minimum draft-2020 keywords (`type`, `properties`,
    /// `required`); richer keywords land additively.
    #[serde(default)]
    pub params: serde_json::Value,

    /// Tables the SQL body reads from. The supervisor's capability
    /// gate cross-checks this against the manifest's
    /// `capabilities.warehouse_read.tables` allowlist: a template
    /// that touches `events` cannot be invoked by an extension whose
    /// grant was `tables: [samples]`.
    #[serde(default)]
    pub tables: Vec<String>,

    /// SQL body. Opaque to this crate — the host's resolver binds
    /// the parameters and runs it. Carried in the spec so the
    /// registry is the full audit record (operator can `cat` the
    /// registry to see exactly what an extension can read).
    ///
    /// `None` for templates whose resolver is fully programmatic
    /// (the four host-builtin templates were originally hand-written
    /// `sqlx::query_as` calls and don't have a single SQL string;
    /// they carry their bucketed SQL in `Some` once lifted, and
    /// `None` if the host registers them as opaque resolvers).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sql: Option<String>,
}

/// The wire payload an extension sends on `warehouse.query`.
///
/// The handle in `starter-ext-sdk` constructs one of these for each
/// `query(template, params)` call and serialises it as the
/// JSON-RPC request's `params`. Receivers (the host's
/// `WarehouseReadBackend`) deserialise and dispatch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WarehouseReadRequest {
    /// Template name (must exist in the host's registry; otherwise
    /// the host returns `Error::Validation`).
    pub template: String,
    /// Parameters bound into the template. Validated against
    /// [`TemplateSpec::params`].
    #[serde(default)]
    pub params: serde_json::Value,
}

/// Wire response for `warehouse.query`. Wraps `Vec<Row>` so the
/// shape can grow additively (cursor token, row count, etc.)
/// without breaking the wire when v2 lands.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WarehouseReadResponse {
    /// Rows the template yielded. Same `Row` shape the builtin
    /// handle returns.
    #[serde(default)]
    pub rows: Vec<Row>,
}

/// The wire payload an extension sends on `warehouse.write`.
///
/// Companion to [`WarehouseReadRequest`]. Carries the unprefixed
/// `table` name (as declared in `contributes.warehouse_tables[]`)
/// and the rows to insert. The host clamps `tenant_id` on every
/// row before issuing the INSERT.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WarehouseWriteRequest {
    /// Unprefixed table name (as in
    /// `contributes.warehouse_tables[].name`). The host resolves
    /// to `<sanitized_extension_id>__<table>` before DDL.
    pub table: String,
    /// Rows to insert. Empty vector is permitted (returns 0 rows
    /// inserted); convenient for batched callers that occasionally
    /// have nothing to flush.
    #[serde(default)]
    pub rows: Vec<Row>,
}

/// Wire response for `warehouse.write`. Wraps the engine's
/// acknowledged row count so v2 can grow shape additively
/// (per-row error map, last-insert id, etc.) without breaking the
/// wire.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WarehouseWriteResponse {
    /// Rows the engine acknowledged. Should match
    /// `request.rows.len()` on success.
    pub rows_inserted: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_spec_round_trip() {
        let spec = TemplateSpec {
            name: "samples_window".into(),
            params: serde_json::json!({
                "type": "object",
                "required": ["entity_id", "from", "to"],
                "properties": {
                    "entity_id": { "type": "string" },
                    "from":      { "type": "string" },
                    "to":        { "type": "string" },
                    "limit":     { "type": "integer", "maximum": 10000 }
                }
            }),
            tables: vec!["samples".into()],
            sql: Some("SELECT ts, value_num FROM samples WHERE …".into()),
        };
        let j = serde_json::to_string(&spec).unwrap();
        let back: TemplateSpec = serde_json::from_str(&j).unwrap();
        assert_eq!(back, spec);
    }

    #[test]
    fn template_spec_optional_sql_elided() {
        let spec = TemplateSpec {
            name: "n".into(),
            params: serde_json::json!({}),
            tables: vec![],
            sql: None,
        };
        let j = serde_json::to_string(&spec).unwrap();
        assert!(!j.contains("sql"));
    }

    #[test]
    fn warehouse_read_request_round_trip() {
        let req = WarehouseReadRequest {
            template: "meter_kwh_last_24h".into(),
            params: serde_json::json!({ "tenant_id": "t-1" }),
        };
        let j = serde_json::to_string(&req).unwrap();
        let back: WarehouseReadRequest = serde_json::from_str(&j).unwrap();
        assert_eq!(back, req);
    }

    #[test]
    fn row_is_transparent() {
        let mut m = serde_json::Map::new();
        m.insert("kwh".into(), serde_json::json!(42.0));
        let r = Row::from_map(m);
        let j = serde_json::to_string(&r).unwrap();
        // Transparent: serialises as the underlying object, not as
        // `{ "0": { … } }`.
        assert_eq!(j, r#"{"kwh":42.0}"#);
    }
}
