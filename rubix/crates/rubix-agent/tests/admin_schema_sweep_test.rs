//! Schema sanity sweep — fails the build if any tool the agent
//! advertises ships without a usable JSON Schema for its input.
//!
//! The proposal — see [`docs/proposal/admin-introspection-and-test-console.md`]
//! §"Schemas — CI gate, not runtime fallback" — requires every
//! registered tool to declare either a real input schema or an
//! explicit "no input" schema (`{}` or `{"type":"object","properties":{}}`).
//! Either is acceptable; what is *not* acceptable is a missing,
//! null, or non-object schema, because the admin console (and the
//! MCP `tools/list` projection that shares the envelope) cannot
//! render a form for it.
//!
//! Running as a workspace test makes the gate blocking: a new
//! tool that forgets `input_schema` fails CI before it can land
//! on master. The sweep is finite (one assertion per registered
//! tool); when the registry grows the cost is linear.

use rubix_agent::registry::build_tool_registry;
use serde_json::Value;

#[test]
fn every_tool_declares_a_usable_input_schema() {
    let tools = build_tool_registry(90, None, None, None, None, None);
    assert!(
        !tools.is_empty(),
        "builtin tool registry must not be empty; check `build_tool_registry`",
    );
    let mut failures: Vec<String> = Vec::new();
    for tool in &tools {
        let def = tool.definition();
        match &def.input_schema {
            Value::Object(map) => {
                // Allowed shapes:
                //   - empty `{}` (parameterless tool)
                //   - `{"type":"object", ...}` (normal tool)
                //   - `{"$ref": ..., ...}` or other schemars output
                // Disallowed: declares `type` that isn't `object`,
                // because the form renderer expects a top-level
                // object the way every existing tool produces.
                if let Some(ty) = map.get("type") {
                    if ty.as_str() != Some("object") {
                        failures.push(format!(
                            "{}: input_schema.type = {ty} (must be \"object\" or absent)",
                            def.name,
                        ));
                    }
                }
            }
            Value::Null => failures.push(format!(
                "{}: input_schema is null — declare `{{}}` for parameterless tools",
                def.name,
            )),
            other => failures.push(format!(
                "{}: input_schema must be a JSON object, got {other}",
                def.name,
            )),
        }
    }
    assert!(
        failures.is_empty(),
        "tool input_schema sweep failed for {} tool(s):\n  - {}",
        failures.len(),
        failures.join("\n  - "),
    );
}
