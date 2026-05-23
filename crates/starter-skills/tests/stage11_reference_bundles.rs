//! Phase 7 stage 11 — integration test for the two reference
//! `SKILL.md` bundles that unblock ai-builder Phase 5.
//!
//! The bundles live at the workspace root under
//! `skills/starter.ai-builder.dashboards/SKILL.md` and
//! `skills/starter.ai-builder.themes/SKILL.md`. Per R-skills-3 row 1
//! (`load_dir(...)` + frontmatter `approved`/absent → approved), both
//! bundles must land in [`SkillRegistry::list`] (not the quarantined
//! list) when loaded via `load_dir` against the host-binary skills dir.
//!
//! What this test pins:
//!
//! - both `SKILL.md` files parse (no `MissingSkillMd`, no
//!   `InvalidFrontmatter`, no `UnsupportedResourceScheme`);
//! - both register as approved (the trust matrix's default path);
//! - a `select()` call with a matching query routes the
//!   [`KeywordSkillSelector`] to one of the two reference skills —
//!   "dashboards" → `starter.ai-builder.dashboards`,
//!   "themes"     → `starter.ai-builder.themes`.
//!
//! What this test deliberately does **not** pin:
//!
//! - the body content, beyond it being non-empty (the SCOPE that
//!   sources it lives in the ai-builder job, not here);
//! - the resource hashes (Phase 4b smokes already cover that path);
//! - the LLM selector's prompt shape (Phase 4 smokes cover it).

use std::path::PathBuf;

use starter_flow_spi::node::{SlotMap, SlotValue};
use starter_flow_spi::skill::{SkillId, SkillSelection, SkillSelector};
use starter_flow_spi::Principal;
use starter_skills::{InMemoryApprovalStore, KeywordSkillSelector, SkillRegistry};

/// Resolve the workspace-root `skills/` directory from this crate's
/// `CARGO_MANIFEST_DIR`. The crate lives at `crates/starter-skills`,
/// so the workspace root is two `..` segments up.
fn workspace_skills_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("skills")
}

fn principal() -> Principal {
    Principal {
        subject: "operator-alice".into(),
        role: starter_spi::auth::Role::Admin,
        scopes: Vec::new(),
        tenant_id: None,
        extra: serde_json::Value::Null,
    }
}

fn input_with(query: &str) -> SlotMap {
    let mut m = SlotMap::new();
    m.insert("query".into(), SlotValue::String(query.to_owned()));
    m
}

#[tokio::test]
async fn workspace_skills_dir_loads_both_reference_bundles_as_approved() {
    let dir = workspace_skills_dir();
    assert!(
        dir.is_dir(),
        "workspace skills/ dir is missing at {}",
        dir.display()
    );

    let registry = SkillRegistry::builder()
        .with_approval_store(InMemoryApprovalStore::new())
        .with_default_selector(KeywordSkillSelector::new())
        .load_dir(&dir)
        .build()
        .await
        .expect("registry builds from workspace skills/ dir");

    // No quarantined entries — both bundles ship with explicit
    // `trust: approved`, and `load_dir` honours that (R-skills-3
    // row 1).
    assert!(
        registry.list_quarantined().is_empty(),
        "no reference bundle should land quarantined; got {:?}",
        registry
            .list_quarantined()
            .iter()
            .map(|s| s.id.to_string())
            .collect::<Vec<_>>(),
    );

    let approved_ids: Vec<String> = registry.list().iter().map(|s| s.id.to_string()).collect();
    assert!(
        approved_ids.contains(&"starter.ai-builder.dashboards".to_string()),
        "dashboards bundle missing from approved list: {approved_ids:?}",
    );
    assert!(
        approved_ids.contains(&"starter.ai-builder.themes".to_string()),
        "themes bundle missing from approved list: {approved_ids:?}",
    );

    // Bodies are non-empty (the model eventually sees these bytes —
    // an empty body would be a parser bug or an authoring bug).
    for id_str in ["starter.ai-builder.dashboards", "starter.ai-builder.themes"] {
        let id = SkillId::new(id_str).unwrap();
        let s = registry.get(&id).expect("get sees approved");
        assert!(!s.body.is_empty(), "{id_str} body must not be empty");
        assert!(
            !s.bundle_hash.is_empty(),
            "{id_str} bundle hash must be set"
        );
    }
}

#[tokio::test]
async fn select_routes_dashboards_query_to_dashboards_skill() {
    let registry = SkillRegistry::builder()
        .with_approval_store(InMemoryApprovalStore::new())
        .with_default_selector(KeywordSkillSelector::new())
        .load_dir(workspace_skills_dir())
        .build()
        .await
        .expect("registry builds");

    // The dashboards bundle's description contains the token
    // "dashboard" (and "dashboards"); a query containing the same
    // token must route there via KeywordSkillSelector's
    // first-overlap rule.
    let sel = registry
        .select(&input_with("please build a dashboard"), &principal())
        .await
        .expect("select ok");

    match sel {
        SkillSelection::Selected {
            skill_id,
            content_hash,
            ..
        } => {
            assert_eq!(
                skill_id.as_str(),
                "starter.ai-builder.dashboards",
                "dashboard query must route to dashboards skill",
            );
            assert!(!content_hash.is_empty());
        }
        other => panic!("select must return a skill for matching query, got {other:?}"),
    }
}

#[tokio::test]
async fn select_routes_themes_query_to_themes_skill() {
    let registry = SkillRegistry::builder()
        .with_approval_store(InMemoryApprovalStore::new())
        .with_default_selector(KeywordSkillSelector::new())
        .load_dir(workspace_skills_dir())
        .build()
        .await
        .expect("registry builds");

    // "theme" appears in the themes bundle description but not in
    // the dashboards one; the keyword selector's first-overlap rule
    // (in `BTreeMap<SkillId, _>` order) must pick themes here.
    let sel = registry
        .select(&input_with("restyle palette"), &principal())
        .await
        .expect("select ok");

    match sel {
        SkillSelection::Selected { skill_id, .. } => {
            assert_eq!(
                skill_id.as_str(),
                "starter.ai-builder.themes",
                "theme query must route to themes skill",
            );
        }
        other => panic!("select must return a skill for matching query, got {other:?}"),
    }
}
