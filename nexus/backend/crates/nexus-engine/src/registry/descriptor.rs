//! The registry describing itself: a metadata layer alongside the builder
//! registrations so the node palette can be generated from the same source of
//! truth that builds the streams.
//!
//! ArkFlow's builder registries only know how to *construct* a component from a
//! config `Value`; they carry no description of the config they accept. The
//! visual flow builder needs the opposite — to *describe* each node so it can
//! offer a palette and a schema-driven config form. Rather than introduce a
//! second registry that could drift from the builders, this module hand-keeps a
//! descriptor next to each registered node; adding a node means registering its
//! builder ([`super::inputs`]/[`super::outputs`]) and adding its descriptor here.

use serde_json::{json, Value};

/// Whether a node is a source, a transform, or a sink — the three palette
/// groups the editor presents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeCategory {
    /// A flow source (ArkFlow input).
    Input,
    /// A pipeline transform (ArkFlow processor).
    Processor,
    /// A flow sink (ArkFlow output).
    Output,
}

impl NodeCategory {
    /// The wire token, matching the `NodeCategory` DTO discriminants.
    pub fn as_str(self) -> &'static str {
        match self {
            NodeCategory::Input => "input",
            NodeCategory::Processor => "processor",
            NodeCategory::Output => "output",
        }
    }
}

/// A described node type: enough for the editor to render a palette entry and a
/// config form, and to serialise the graph back to the ArkFlow `{type, ...}`
/// config the engine builds. `config_schema` is a JSON Schema (draft 2020-12)
/// for the node's config object.
#[derive(Debug, Clone)]
pub struct NodeDescriptor {
    /// The ArkFlow `type` discriminant, e.g. `http_poll`, `sql`, `postgres`.
    pub kind: &'static str,
    /// Which palette group the node belongs to.
    pub category: NodeCategory,
    /// Human label for the palette.
    pub label: &'static str,
    /// One-line description of what the node does.
    pub description: &'static str,
    /// JSON Schema describing the node's config object.
    pub config_schema: Value,
}

/// Every registered node, described. The order is palette order: inputs, then
/// processors, then outputs. Kept in lockstep with the builder registrations in
/// [`super::inputs`] and [`super::outputs`] (and the vendored ArkFlow
/// processors `sql`/`json_to_arrow`/`arrow_to_json`).
pub fn describe() -> Vec<NodeDescriptor> {
    vec![
        http_poll(),
        simulator(),
        sql_processor(),
        json_to_arrow(),
        arrow_to_json(),
        collector(),
        sse(),
        postgres(),
    ]
}

/// A required string property.
fn str_prop(description: &str) -> Value {
    json!({ "type": "string", "description": description })
}

fn http_poll() -> NodeDescriptor {
    NodeDescriptor {
        kind: "http_poll",
        category: NodeCategory::Input,
        label: "HTTP poll",
        description: "Fetch a JSON endpoint on a fixed interval; each response is one batch.",
        config_schema: json!({
            "type": "object",
            "properties": {
                "url": str_prop("Endpoint to GET each tick."),
                "interval": str_prop("Delay between polls, e.g. \"15m\", \"30s\"."),
                "bearer": str_prop("Optional bearer token sent as Authorization."),
            },
            "required": ["url", "interval"],
            "additionalProperties": false,
        }),
    }
}

fn simulator() -> NodeDescriptor {
    NodeDescriptor {
        kind: "simulator",
        category: NodeCategory::Input,
        label: "Device simulator",
        description: "Emit synthetic device telemetry on an interval — test data, no upstream.",
        config_schema: json!({
            "type": "object",
            "properties": {
                "profile": {
                    "type": "string",
                    "enum": ["hvac", "energy", "door"],
                    "description": "Which device shape to emit.",
                },
                "interval": str_prop("Delay between emits, e.g. \"5s\", \"1m\"."),
                "device_id": str_prop("Identifies the simulated device; copied onto every row."),
                "seed": {
                    "type": "integer",
                    "minimum": 0,
                    "description": "Seeds the deterministic generator so a flow replays identically.",
                },
            },
            "required": ["profile", "interval", "device_id"],
            "additionalProperties": false,
        }),
    }
}

fn sql_processor() -> NodeDescriptor {
    NodeDescriptor {
        kind: "sql",
        category: NodeCategory::Processor,
        label: "SQL transform",
        description: "Run a DataFusion SQL statement over the in-flight batch.",
        config_schema: json!({
            "type": "object",
            "properties": {
                "query": str_prop("SQL statement; the batch is the `flow` table by default."),
                "table_name": str_prop("Table name the batch is registered under (default \"flow\")."),
            },
            "required": ["query"],
            "additionalProperties": true,
        }),
    }
}

fn json_to_arrow() -> NodeDescriptor {
    NodeDescriptor {
        kind: "json_to_arrow",
        category: NodeCategory::Processor,
        label: "JSON → Arrow",
        description: "Parse a JSON-document batch into an Arrow record batch for SQL processing.",
        config_schema: json!({ "type": "object", "additionalProperties": true }),
    }
}

fn arrow_to_json() -> NodeDescriptor {
    NodeDescriptor {
        kind: "arrow_to_json",
        category: NodeCategory::Processor,
        label: "Arrow → JSON",
        description: "Render an Arrow record batch back to JSON-document rows.",
        config_schema: json!({ "type": "object", "additionalProperties": true }),
    }
}

fn collector() -> NodeDescriptor {
    NodeDescriptor {
        kind: "collector",
        category: NodeCategory::Output,
        label: "Collector (bounded)",
        description: "Capture rows in memory for a one-shot run; bounded by per-run caps.",
        config_schema: json!({
            "type": "object",
            "properties": {
                "run_id": str_prop("Run id the runner reserves; set by the engine, not the editor."),
            },
            "required": ["run_id"],
            "additionalProperties": false,
        }),
    }
}

fn sse() -> NodeDescriptor {
    NodeDescriptor {
        kind: "sse",
        category: NodeCategory::Output,
        label: "Live (SSE)",
        description: "Fan each batch out to live SSE subscribers.",
        config_schema: json!({
            "type": "object",
            "properties": {
                "run_id": str_prop("Run id the live runner reserves; set by the engine."),
            },
            "required": ["run_id"],
            "additionalProperties": false,
        }),
    }
}

fn postgres() -> NodeDescriptor {
    NodeDescriptor {
        kind: "postgres",
        category: NodeCategory::Output,
        label: "Postgres sink",
        description: "Insert each batch's rows into a table in a datasource Postgres.",
        config_schema: json!({
            "type": "object",
            "properties": {
                "uri": str_prop("Connection string for the target Postgres."),
                "table": str_prop("Table the shaped rows are inserted into."),
            },
            "required": ["uri", "table"],
            "additionalProperties": false,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn describes_every_registered_node() {
        let nodes = describe();
        let kinds: Vec<&str> = nodes.iter().map(|n| n.kind).collect();
        for expected in [
            "http_poll",
            "simulator",
            "sql",
            "json_to_arrow",
            "arrow_to_json",
            "collector",
            "sse",
            "postgres",
        ] {
            assert!(kinds.contains(&expected), "missing descriptor for {expected}");
        }
    }

    #[test]
    fn every_schema_is_an_object_schema() {
        for node in describe() {
            assert_eq!(
                node.config_schema.get("type").and_then(Value::as_str),
                Some("object"),
                "{} schema must describe an object",
                node.kind
            );
        }
    }

    #[test]
    fn categories_group_inputs_processors_outputs() {
        let nodes = describe();
        let cat = |k: &str| nodes.iter().find(|n| n.kind == k).unwrap().category;
        assert_eq!(cat("http_poll"), NodeCategory::Input);
        assert_eq!(cat("sql"), NodeCategory::Processor);
        assert_eq!(cat("postgres"), NodeCategory::Output);
    }
}
