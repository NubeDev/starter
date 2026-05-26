//! Cross-checks that every bundled YAML parses, converts, and surfaces
//! the expected `ai-agent` root the host's MCP catalogue depends on.

use rubix_flows::{
    convert, load_all, parse_yaml, AI_AGENT_KIND_ID, AI_AGENT_KIND_YAML, ALLOWED_TOOLS_KEY,
    BUNDLED, DEFAULT_SEED_SLOT, NODE_ID_PREFIX, TOOLS_KEY,
};

const EXPECTED_FLOW_IDS: &[&str] = &[
    "com.rubix.scheduled-system-check",
    "com.rubix.weekly-report",
    "com.rubix.dashboard-assistant",
    "com.rubix.flow-programmer",
    "com.rubix.clickhouse-ruler",
    "com.rubix.user-admin",
    "com.rubix.tick-counter",
];

/// Bundled flows that do NOT root at `ai-agent` and therefore opt
/// out of the AI-agent-shape assertions below. The Phase D
/// `tick-counter` flow is the first such flow: it roots at
/// `starter.flow.trigger.schedule` and chains into the counter +
/// log node kinds, exercising the always-on flow runtime + the
/// new `NodeStateStore` seam end to end.
const NON_AI_AGENT_FLOW_IDS: &[&str] = &["com.rubix.tick-counter"];

fn sorted(v: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut out: Vec<String> = v.into_iter().collect();
    out.sort();
    out
}

#[test]
fn every_bundled_yaml_parses_with_ai_agent_root() {
    let mut seen = Vec::new();
    for file in BUNDLED.files() {
        let path = file.path().to_string_lossy().into_owned();
        if !(path.ends_with(".yaml") || path.ends_with(".yml")) {
            continue;
        }
        let yaml = parse_yaml(&path, file.contents()).expect("yaml parses");
        assert!(!yaml.nodes.is_empty(), "flow `{path}` has zero nodes");
        let root = &yaml.nodes[0];
        assert!(!root.id.is_empty());
        if !NON_AI_AGENT_FLOW_IDS.contains(&yaml.id.as_str()) {
            assert_eq!(root.kind, AI_AGENT_KIND_YAML);
        }
        seen.push(yaml.id);
    }
    let expected: Vec<String> = EXPECTED_FLOW_IDS.iter().map(|s| (*s).to_owned()).collect();
    assert_eq!(sorted(seen), sorted(expected));
}

#[test]
fn load_all_converts_every_bundled_flow() {
    let triples = load_all().expect("load_all succeeds");
    assert_eq!(triples.len(), EXPECTED_FLOW_IDS.len());
    for (flow_id, _rev, body) in &triples {
        assert_eq!(&body.flow_id, flow_id);
        assert!(!body.nodes.is_empty());
        let root = &body.nodes[0];
        assert!(root.id.as_str().starts_with(NODE_ID_PREFIX));
        assert!(root.triggers.iter().any(|t| t == DEFAULT_SEED_SLOT));
        if !NON_AI_AGENT_FLOW_IDS.contains(&flow_id.as_str()) {
            assert_eq!(root.kind.as_str(), AI_AGENT_KIND_ID);
        }
    }
    let seen = triples.iter().map(|(id, _, _)| id.to_string());
    let expected = EXPECTED_FLOW_IDS.iter().map(|s| (*s).to_owned());
    assert_eq!(sorted(seen), sorted(expected));
}

/// Phase A.3 — the full `allowed_tools[]` list must round-trip from
/// YAML through `convert()` onto the AiAgentNode config, not just
/// `[0]`. Asserts both the surface-level `RubixNodeYaml::allowed_tools`
/// accessor and the post-conversion `NodeDecl.settings.allowed_tools`
/// JSON array carry every entry, in declaration order, so AgentLoop's
/// `ToolSet` filter can scope tool visibility per flow.
#[test]
fn allowed_tools_multi_entry_list_round_trips_through_convert() {
    const PATH: &str = "tests/fixtures/multi-tool.yaml";
    let yaml_src = r#"
id: com.rubix.multi-tool-test
description: Phase A.3 fixture — multi-entry allowed_tools list.
trigger: explicit
nodes:
  - id: agent
    kind: ai-agent
    config:
      session_policy: fresh
      skill_hint: com.rubix.system-checker
      allowed_tools:
        - rubix.system.disk
        - rubix.system.db
        - rubix.system.flow_errors
        - rubix.alert.send
links: []
"#;

    let parsed = parse_yaml(PATH, yaml_src.as_bytes()).expect("yaml parses");

    // (a) Surface accessor returns the full list, in order.
    let surface = parsed.nodes[0]
        .allowed_tools()
        .expect("allowed_tools parses");
    assert_eq!(
        surface,
        vec![
            "rubix.system.disk".to_owned(),
            "rubix.system.db".to_owned(),
            "rubix.system.flow_errors".to_owned(),
            "rubix.alert.send".to_owned(),
        ],
        "RubixNodeYaml::allowed_tools must surface every entry, not just [0]"
    );

    // (b) After convert(), the AiAgentNode settings carry the same
    //     list — this is the seam AgentLoop's ToolSet filter reads.
    let (_flow_id, _rev, body) = convert(PATH, parsed).expect("convert succeeds");
    let root = &body.nodes[0];
    let arr = root
        .settings
        .get(ALLOWED_TOOLS_KEY)
        .and_then(|v| v.as_array())
        .expect("settings.allowed_tools is a JSON array");
    let on_node: Vec<String> = arr
        .iter()
        .map(|v| v.as_str().expect("string entry").to_owned())
        .collect();
    assert_eq!(
        on_node,
        vec![
            "rubix.system.disk".to_owned(),
            "rubix.system.db".to_owned(),
            "rubix.system.flow_errors".to_owned(),
            "rubix.alert.send".to_owned(),
        ],
        "NodeDecl.settings.allowed_tools must carry every entry post-convert"
    );
}

/// Stage 07 — `config.tools` (the CLI built-in restriction, distinct
/// from `allowed_tools` which gates MCP-bridged tools) must round-trip
/// from YAML through `convert()` onto the NodeDecl settings,
/// **preserving an empty list**. `tools: []` is the documented
/// "MCP only, no built-ins" lockdown; collapsing empty to absent
/// would silently re-enable Bash / Read / Edit and let the model
/// curl around the MCP surface. Also asserts the absent-key case
/// surfaces as `None` so callers can distinguish lockdown from
/// "no per-flow restriction".
#[test]
fn tools_empty_list_round_trips_through_convert_as_empty_not_absent() {
    const PATH: &str = "tests/fixtures/tools-empty.yaml";
    let yaml_src = r#"
id: com.rubix.tools-empty-test
description: Stage 07 fixture — tools:[] lockdown round-trip.
trigger: explicit
nodes:
  - id: agent
    kind: ai-agent
    config:
      session_policy: fresh
      skill_hint: com.rubix.dashboard-builder
      tools: []
      allowed_tools:
        - rubix.dashboard.list
links: []
"#;

    let parsed = parse_yaml(PATH, yaml_src.as_bytes()).expect("yaml parses");

    // (a) Surface accessor distinguishes empty list from absent key.
    let surface = parsed.nodes[0].tools().expect("tools parses");
    assert_eq!(
        surface,
        Some(Vec::<String>::new()),
        "RubixNodeYaml::tools must surface `tools: []` as Some(empty), \
         not None — the empty-list lockdown shape"
    );

    // (b) After convert(), the NodeDecl settings carry an empty JSON
    //     array (not a missing key).
    let (_flow_id, _rev, body) = convert(PATH, parsed).expect("convert succeeds");
    let root = &body.nodes[0];
    let arr = root
        .settings
        .get(TOOLS_KEY)
        .and_then(|v| v.as_array())
        .expect("settings.tools must be present as a JSON array even when empty");
    assert!(
        arr.is_empty(),
        "settings.tools must be an empty array for `tools: []` lockdown"
    );

    // (c) Absent `tools` key surfaces as `None`.
    let absent_yaml = r#"
id: com.rubix.tools-absent-test
description: tools key omitted.
trigger: explicit
nodes:
  - id: agent
    kind: ai-agent
    config:
      session_policy: fresh
      allowed_tools:
        - rubix.dashboard.list
links: []
"#;
    let parsed_absent = parse_yaml(PATH, absent_yaml.as_bytes()).expect("yaml parses");
    assert_eq!(
        parsed_absent.nodes[0].tools().expect("tools parses"),
        None,
        "absent `tools` key must surface as None — CLI default catalogue \
         stays in scope when the YAML does not declare a lockdown"
    );
    let (_, _, body_absent) = convert(PATH, parsed_absent).expect("convert succeeds");
    assert!(
        body_absent.nodes[0].settings.get(TOOLS_KEY).is_none(),
        "absent `tools` key must not synthesise a settings entry"
    );
}

/// Phase D sanity — the bundled `com.rubix.tick-counter` YAML parses
/// and converts to a `FlowBody` with three nodes (schedule trigger ->
/// counter -> log) and two edges (`tick.fire -> count.in` and
/// `count.out -> emit.value`). This is the smallest end-to-end
/// proof of the always-on flow runtime + `NodeStateStore` seam.
#[test]
fn bundled_tick_counter_parses_with_three_nodes_and_two_edges() {
    const PATH: &str = "tick-counter.yaml";
    let bytes = BUNDLED
        .get_file(PATH)
        .expect("tick-counter.yaml is bundled under flows/")
        .contents();

    let yaml = parse_yaml(PATH, bytes).expect("tick-counter yaml parses");
    assert_eq!(yaml.id, "com.rubix.tick-counter");
    assert_eq!(yaml.trigger.as_deref(), Some("schedule"));
    assert_eq!(yaml.cron_expr.as_deref(), Some("*/5 * * * * *"));
    assert_eq!(yaml.nodes.len(), 3);
    assert_eq!(yaml.links.len(), 2);

    let (flow_id, _rev, body) = convert(PATH, yaml).expect("tick-counter converts");
    assert_eq!(flow_id.as_str(), "com.rubix.tick-counter");
    assert_eq!(body.nodes.len(), 3, "three nodes: trigger -> counter -> log");
    assert_eq!(
        body.links.len(),
        2,
        "two edges: tick.fire -> count.in and count.out -> emit.value"
    );

    let kinds: Vec<&str> = body.nodes.iter().map(|n| n.kind.as_str()).collect();
    assert_eq!(
        kinds,
        vec![
            "starter.flow.trigger.schedule",
            "starter.flow.counter",
            "starter.flow.log",
        ],
        "node kinds preserved in declaration order (kinds other than `ai-agent` pass through verbatim per convert.rs)"
    );

    let ids: Vec<&str> = body.nodes.iter().map(|n| n.id.as_str()).collect();
    assert_eq!(
        ids,
        vec!["com.rubix.tick", "com.rubix.count", "com.rubix.emit"],
        "short YAML ids are reverse-DNS-prefixed by convert()"
    );

    let froms: Vec<&str> = body.links.iter().map(|l| l.from.as_str()).collect();
    let tos: Vec<&str> = body.links.iter().map(|l| l.to.as_str()).collect();
    assert_eq!(
        froms,
        vec!["com.rubix.tick.fire", "com.rubix.count.out"],
        "link endpoint node ids are reverse-DNS-prefixed to match the rewritten node ids; \
         the trigger.schedule node fans out on its per-tick `fire` output \
         (starter-flow-nodes::trigger_schedule::FIRE_SLOT), not the constant `schedule` \
         slot which would defeat R3's idempotent-write short-circuit after tick 1"
    );
    assert_eq!(tos, vec!["com.rubix.count.in", "com.rubix.emit.value"]);
}
