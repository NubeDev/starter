//! Cross-checks that every bundled YAML parses, converts, and surfaces
//! the expected `ai-agent` root the host's MCP catalogue depends on.

use rubix_flows::{
    load_all, parse_yaml, AI_AGENT_KIND_ID, AI_AGENT_KIND_YAML, BUNDLED, DEFAULT_SEED_SLOT,
    NODE_ID_PREFIX,
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
