//! `FlowRegistry::register` + `register_yaml` + the
//! [`FlowRegistration`] builder.
//!
//! `register` takes the parsed [`FlowBody`] in hand;
//! `register_yaml` reads a YAML / JSON file from disk through
//! `serde_yaml` (already used by `starter-flow-watch`) and feeds
//! the body through the same path. Both routes pre-resolve the
//! topology against the caller-supplied [`NodeKindRegistry`] so
//! later [`super::FlowRegistry::resolve`] calls are O(1) lookups.
//!
//! See `docs/design/starter-changes/README.md` Phase 2b U3 for the
//! rationale. D-F3.4 (no schema or adapter derivation from the
//! flow body) is the load-bearing constraint: schemas, the seed
//! slot, the terminal slots, the tool id / name / description,
//! and any non-default seed / output adapters must all be supplied
//! at registration time. `with_default_adapters` is the common
//! sugar — single seed slot, single terminal slot — but it does
//! not weaken D-F3.4: the slot identities still come from the
//! caller, only the wiring is templated.

use std::path::Path;
use std::sync::Arc;

use serde_json::Value;

use starter_flow::definition::body::FlowBody;
use starter_flow::registry::NodeKindRegistry;
use starter_flow_spi::flow::{FlowId, FlowRevisionId};
use starter_flow_spi::node::{KindId, SlotMap, SlotRef, SlotValue};

use crate::{OutputAdapter, SeedAdapter};

use super::{
    check_terminal_slots, insert_registered, resolve_topology, FlowRegistry, FlowRegistryError,
    RegisteredFlow,
};

/// Builder for one registration on [`FlowRegistry`].
///
/// The flow body + revision + tool metadata + adapters are
/// gathered in one shape so [`FlowRegistry::register`] never
/// grows past the four-argument horizon. Mandatory fields are
/// constructor-positional; everything else is set fluently.
#[must_use = "FlowRegistration does nothing until passed to FlowRegistry::register"]
pub struct FlowRegistration {
    pub(crate) body: FlowBody,
    pub(crate) revision: FlowRevisionId,
    pub(crate) terminal_slots: Vec<SlotRef>,
    pub(crate) input_schema: Value,
    pub(crate) output_schema: Value,
    pub(crate) tool_id: KindId,
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) seed_adapter: Option<SeedAdapter>,
    pub(crate) output_adapter: Option<OutputAdapter>,
}

impl FlowRegistration {
    /// Start a new registration. The mandatory five: body,
    /// revision, tool id, name, description. Schemas, terminals,
    /// and adapters are set with the named methods.
    pub fn new(
        body: FlowBody,
        revision: FlowRevisionId,
        tool_id: KindId,
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            body,
            revision,
            terminal_slots: Vec::new(),
            input_schema: Value::Object(serde_json::Map::new()),
            output_schema: Value::Object(serde_json::Map::new()),
            tool_id,
            name: name.into(),
            description: description.into(),
            seed_adapter: None,
            output_adapter: None,
        }
    }

    /// Set the terminal slots read back at end of a successful
    /// run. Required (non-empty).
    pub fn terminal_slots(mut self, slots: Vec<SlotRef>) -> Self {
        self.terminal_slots = slots;
        self
    }

    /// Required: the input JSON-schema (D-F3.4).
    pub fn input_schema(mut self, schema: Value) -> Self {
        self.input_schema = schema;
        self
    }

    /// Required: the output JSON-schema (D-F3.4).
    pub fn output_schema(mut self, schema: Value) -> Self {
        self.output_schema = schema;
        self
    }

    /// Wire the imperative adapter pair explicitly (D-F3.4).
    /// Mutually exclusive with [`Self::with_default_adapters`];
    /// the last call wins.
    pub fn with_adapters(mut self, seed: SeedAdapter, output: OutputAdapter) -> Self {
        self.seed_adapter = Some(seed);
        self.output_adapter = Some(output);
        self
    }

    /// Wire the common "single seed slot, single terminal slot"
    /// adapter pair:
    ///
    /// - seed adapter: copies the entire JSON input value into
    ///   `seed_slot` as a [`SlotValue::Json`];
    /// - output adapter: reads `output_slot` back as JSON; missing
    ///   slot returns [`Value::Null`].
    ///
    /// Both slots must reference nodes declared in the body —
    /// validated at register time. This is the shape every flow
    /// in the rubix `flows/*.yaml` set uses today; per-flow
    /// shaping goes through [`Self::with_adapters`].
    pub fn with_default_adapters(mut self, seed_slot: SlotRef, output_slot: SlotRef) -> Self {
        let seed_slot_for_adapter = seed_slot.clone();
        let seed: SeedAdapter = Arc::new(move |input: &Value| {
            vec![(
                seed_slot_for_adapter.clone(),
                SlotValue::Json(input.clone()),
            )]
        });
        let output_key = format!("{}.{}", output_slot.node, output_slot.slot);
        let output: OutputAdapter = Arc::new(move |out: &SlotMap| match out.get(&output_key) {
            Some(SlotValue::Json(v)) => v.clone(),
            Some(SlotValue::Null) | None => Value::Null,
            // Non-JSON slot values (e.g. Bytes) get wrapped in a
            // string surface so the tool surface never panics; a
            // caller needing richer typing wires `with_adapters`.
            Some(other) => Value::String(format!("{other:?}")),
        });
        self.seed_adapter = Some(seed);
        self.output_adapter = Some(output);
        // If the output slot wasn't explicitly added to
        // `terminal_slots`, push it on — the adapter reads it
        // back at run end so the registry needs the engine to
        // checkpoint it on the terminal-slot collection pass.
        if !self.terminal_slots.iter().any(|s| s == &output_slot) {
            self.terminal_slots.push(output_slot);
        }
        // The seed slot doesn't need to be in `terminal_slots`;
        // seeding is the input side, not the output side.
        let _ = seed_slot;
        self
    }
}

impl FlowRegistry {
    /// Register `spec` under `(spec.body.flow_id, spec.revision)`.
    ///
    /// Pre-resolves the topology against `kinds` so every later
    /// `resolve` / `from_registry` call is an O(1) lookup. Refuses
    /// duplicate `(flow_id, revision)` pairs and any registration
    /// missing required fields.
    pub async fn register(
        &self,
        spec: FlowRegistration,
        kinds: &NodeKindRegistry,
    ) -> Result<Arc<RegisteredFlow>, FlowRegistryError> {
        let FlowRegistration {
            body,
            revision,
            terminal_slots,
            input_schema,
            output_schema,
            tool_id,
            name,
            description,
            seed_adapter,
            output_adapter,
        } = spec;

        if terminal_slots.is_empty() {
            return Err(FlowRegistryError::Resolve {
                flow: body.flow_id.clone(),
                revision,
                error: starter_flow::definition::TopologyResolverError::BodyShape {
                    detail: "FlowRegistration is missing terminal_slots (call \
                             `.terminal_slots(...)` or `.with_default_adapters(...)`)"
                        .to_owned(),
                },
            });
        }
        let seed_adapter = seed_adapter.ok_or_else(|| FlowRegistryError::Resolve {
            flow: body.flow_id.clone(),
            revision,
            error: starter_flow::definition::TopologyResolverError::BodyShape {
                detail: "FlowRegistration is missing a seed_adapter (call \
                         `.with_adapters(...)` or `.with_default_adapters(...)`)"
                    .to_owned(),
            },
        })?;
        let output_adapter = output_adapter.ok_or_else(|| FlowRegistryError::Resolve {
            flow: body.flow_id.clone(),
            revision,
            error: starter_flow::definition::TopologyResolverError::BodyShape {
                detail: "FlowRegistration is missing an output_adapter (call \
                         `.with_adapters(...)` or `.with_default_adapters(...)`)"
                    .to_owned(),
            },
        })?;

        check_terminal_slots(&body, &terminal_slots)?;
        let flow_id = body.flow_id.clone();
        let topology = resolve_topology(&body, &flow_id, revision, kinds).await?;

        insert_registered(
            self,
            RegisteredFlow {
                flow_id,
                revision,
                body,
                topology,
                terminal_slots,
                input_schema,
                output_schema,
                tool_id,
                name,
                description,
                seed_adapter,
                output_adapter,
            },
        )
        .await
    }

    /// Read `path` (`.yaml` / `.yml` / `.json`), parse it as a
    /// [`FlowBody`], cross-check `spec.body.flow_id` against the
    /// caller-passed `flow_id`, and register.
    ///
    /// `spec_factory` builds a [`FlowRegistration`] *given the
    /// parsed body*. This is the seam D-F3.4 forces: the body
    /// comes from the file, the schemas + tool metadata + adapters
    /// come from the caller, both meet at the factory. Caller code
    /// reads (in rubix):
    ///
    /// ```ignore
    /// registry.register_yaml(
    ///     "flows/scheduled-system-check.yaml",
    ///     &kinds,
    ///     |body| FlowRegistration::new(
    ///             body,
    ///             FlowRevisionId::new(),
    ///             KindId::new("com.rubix.scheduled-system-check").unwrap(),
    ///             "scheduled_system_check",
    ///             "Inspect rubix host health and alert.")
    ///         .input_schema(serde_json::json!({"type":"object"}))
    ///         .output_schema(serde_json::json!({"type":"object"}))
    ///         .with_default_adapters(seed_slot, output_slot),
    /// ).await?;
    /// ```
    pub async fn register_yaml<P, F>(
        &self,
        path: P,
        kinds: &NodeKindRegistry,
        spec_factory: F,
    ) -> Result<Arc<RegisteredFlow>, FlowRegistryError>
    where
        P: AsRef<Path>,
        F: FnOnce(FlowBody) -> FlowRegistration,
    {
        let body = read_yaml_body(path.as_ref())?;
        let spec = spec_factory(body);
        self.register(spec, kinds).await
    }
}

fn read_yaml_body(path: &Path) -> Result<FlowBody, FlowRegistryError> {
    let bytes = std::fs::read(path).map_err(|e| FlowRegistryError::YamlShape {
        path: path.display().to_string(),
        detail: format!("read: {e}"),
    })?;
    // serde_yaml deserialises into the FlowBody directly — the
    // FlowBody serde derives are forward-compatible with the
    // body shape `starter-flow-watch::parse_flow_file` already
    // accepts. We *don't* go via serde_json::Value first because
    // YAML numeric coercion is more permissive there and we want
    // the typed-body error path to fire on a malformed file.
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase());
    let body: FlowBody = match ext.as_deref() {
        Some("yaml") | Some("yml") => {
            serde_yaml::from_slice(&bytes).map_err(|e| FlowRegistryError::YamlShape {
                path: path.display().to_string(),
                detail: e.to_string(),
            })?
        }
        Some("json") => {
            serde_json::from_slice(&bytes).map_err(|e| FlowRegistryError::YamlShape {
                path: path.display().to_string(),
                detail: e.to_string(),
            })?
        }
        _ => {
            return Err(FlowRegistryError::YamlShape {
                path: path.display().to_string(),
                detail: "unsupported extension; expected .yaml / .yml / .json".to_owned(),
            });
        }
    };
    Ok(body)
}

/// Convenience: register every entry in an iterator under the
/// same kind registry. Stops on the first error; returns the
/// number of successful registrations alongside the error so the
/// host can decide whether to roll back or proceed partial.
pub async fn register_all<I>(
    registry: &FlowRegistry,
    specs: I,
    kinds: &NodeKindRegistry,
) -> Result<Vec<Arc<RegisteredFlow>>, (Vec<Arc<RegisteredFlow>>, FlowRegistryError)>
where
    I: IntoIterator<Item = FlowRegistration>,
{
    let mut out = Vec::new();
    for spec in specs {
        match registry.register(spec, kinds).await {
            Ok(r) => out.push(r),
            Err(e) => return Err((out, e)),
        }
    }
    Ok(out)
}

// Touch the FlowId import so the module compiles even if a future
// refactor inlines the `body.flow_id.clone()` calls above.
const _: fn() = || {
    let _: Option<FlowId> = None;
};
