//! Cross-checks that every bundled YAML parses, converts, and surfaces
//! the expected `ai-agent` root the host's MCP catalogue depends on.

use rubix_flows::{
    convert, load_all, parse_yaml, AI_AGENT_KIND_ID, AI_AGENT_KIND_YAML, ALLOWED_TOOLS_KEY,
    BUNDLED, DEFAULT_SEED_SLOT, NODE_ID_PREFIX,
};

const EXPECTED_FLOW_IDS: &[&str] = &[
    "com.rubix.scheduled-system-check",
    "com.rubix.weekly-report",
    "com.rubix.dashboard-assistant",
    "com.rubix.flow-programmer",
    "com.rubix.clickhouse-ruler",
    "com.rubix.user-admin",
];

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
        assert_eq!(root.kind, AI_AGENT_KIND_YAML);
        assert!(!root.id.is_empty());
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
        assert_eq!(root.kind.as_str(), AI_AGENT_KIND_ID);
        assert!(root.id.as_str().starts_with(NODE_ID_PREFIX));
        assert!(root.triggers.iter().any(|t| t == DEFAULT_SEED_SLOT));
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
