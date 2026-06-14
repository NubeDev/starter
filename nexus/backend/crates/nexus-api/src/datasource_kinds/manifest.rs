//! The YAML manifest that declares a datasource-kinds pack — the connector-as-
//! declaration format (WS-10 §4.1B): a datasource *type* described by files
//! instead of a Rust enum edited across DTOs, forms, and the registry.
//!
//! A manifest is data only: it names each datasource-kind and points at its
//! config-schema file by relative path, lists which config fields are secrets,
//! and declares how its connectivity is tested and (for query connectors) which
//! SQL dialect shapes its time macros. Adding a connector becomes a manifest
//! entry plus a thin per-protocol builder, not enum edits scattered across the
//! tree. Field names are kept aligned to the query-kind manifest mental model.

use serde::Deserialize;

/// A parsed pack manifest: the list of datasource-kinds it contributes.
#[derive(Debug, Clone, Deserialize)]
pub struct Manifest {
    /// The datasource-kinds this pack declares. Each entry is a connector type
    /// wired by the manifest, not by code.
    #[serde(default)]
    pub datasource_kinds: Vec<ManifestEntry>,
}

/// One datasource-kind declaration. `name` is the id a datasource record stores
/// and a config form is rendered from; file paths are relative to the manifest's
/// directory.
#[derive(Debug, Clone, Deserialize)]
pub struct ManifestEntry {
    /// The datasource-kind id (e.g. `postgres`, `mqtt`). A datasource record
    /// stores this; the registry refuses anything it does not hold.
    pub name: String,

    /// Which query surface this connector serves. A `query` connector answers
    /// `POST /query` (request → rows); a `stream` connector feeds live panels and
    /// flows (subscribe → events) and is not a `POST /query` target.
    pub surface: Surface,

    /// Relative path to the JSON-Schema config file
    /// (`additionalProperties: false`, defaults, min/max). Validated before a
    /// datasource of this kind is accepted.
    pub config_schema: String,

    /// Which config properties hold secrets (e.g. `password`). They are sealed by
    /// the envelope at rest, redacted on read, and decrypted only at connect — the
    /// declaration drives the secret boundary instead of hard-coding it per kind.
    #[serde(default)]
    pub secret_fields: Vec<String>,

    /// How this connector's connectivity is tested before save. For a query
    /// connector this is a probe query; for a stream connector it is a connect
    /// probe (open + immediately close a session). See [`TestSpec`].
    pub test: TestSpec,

    /// The SQL dialect a *query* connector expresses its time macros in. Absent
    /// for stream connectors (they are not `POST /query` targets, so they bind no
    /// `$__timeFilter`/`$__timeGroup`).
    #[serde(default)]
    pub dialect: Option<String>,

    /// Optional human description for the per-kind config form UI.
    #[serde(default)]
    pub description: Option<String>,
}

/// Which query surface a datasource-kind serves. Determines whether it is a
/// `POST /query` target (`query`) or a live subscribe-only source (`stream`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Surface {
    /// Request → rows: queried ad hoc by panels (Postgres, HTTP-REST, Prometheus).
    Query,
    /// Subscribe → events: feeds live panels and flows (MQTT, Kafka).
    Stream,
}

/// How a datasource-kind's connectivity is tested before the datasource is saved.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case", tag = "mode")]
pub enum TestSpec {
    /// Run a trivial probe query and assert it round-trips. For SQL connectors the
    /// `query` defaults to `SELECT 1`; the connector's probe runs it.
    Query {
        /// The probe SQL (defaults to `SELECT 1` when omitted).
        #[serde(default = "default_probe_query")]
        query: String,
    },
    /// Open a session against the broker/endpoint and immediately close it. Proves
    /// the address + credentials connect without subscribing to anything.
    Connect,
}

/// The default probe query for a SQL connector whose manifest omits one.
fn default_probe_query() -> String {
    "SELECT 1".to_string()
}
