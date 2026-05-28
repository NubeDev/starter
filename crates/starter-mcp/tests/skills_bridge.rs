//! Integration test for the `skills_bridge` adapter.
//!
//! Loads two SKILL.md bundles from a tempdir (one approved-by-frontmatter,
//! one quarantined-by-frontmatter), builds a ToolRegistry through
//! `register_approved_skills`, and verifies:
//!
//! - tools/list exposes only the approved bundle.
//! - invoking the quarantined skill name fails (no such tool).
//! - approving the quarantined bundle through `ApprovalStore`,
//!   then rebuilding the registry, exposes and invokes it.
//! - a revoke (without a registry rebuild) causes invoke to fail
//!   at call time — the adapter re-checks the approval store on
//!   every call.
//! - the `add_favorite` meta-tool writes a quarantined SKILL.md and
//!   does not surface it until an operator approves.

#![cfg(feature = "skills")]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::json;
use starter_mcp::skills_bridge::{
    register_approved_skills, AddFavoriteTool, SkillTool, TracingSkillAuditSink,
};
use starter_mcp::ToolRegistry;
use starter_skills::{ApprovalRow, ApprovalStore, InMemoryApprovalStore, SkillRegistry};
use starter_spi::auth::{Principal, Role};
use starter_spi::tool::Tool;

fn principal() -> Principal {
    Principal {
        subject: "operator-alice".into(),
        role: Role::Admin,
        scopes: Vec::new(),
        tenant_id: None,
        teams: Vec::new(),
        extra: serde_json::Value::Null,
    }
}

// ---------- tempdir helper (no `tempfile` dep) ----------

struct TempDir(PathBuf);
impl TempDir {
    fn new(tag: &str) -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let p = std::env::temp_dir().join(format!(
            "starter-mcp-skills-{tag}-{}-{nanos}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        Self(p)
    }
    fn path(&self) -> &Path {
        &self.0
    }
}
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn write_skill(dir: &Path, name: &str, trust: &str, id: &str, body: &str) {
    let bundle = dir.join(name);
    fs::create_dir_all(&bundle).unwrap();
    let contents = format!(
        "---\nid: {id}\ndescription: smoke {id}\ntrust: {trust}\n---\n{body}\n"
    );
    fs::write(bundle.join("SKILL.md"), contents).unwrap();
}

async fn build_registry(
    skills_dir: &Path,
    store: Arc<dyn ApprovalStore>,
) -> SkillRegistry {
    SkillRegistry::builder()
        .with_approval_store_arc(store)
        .load_dir(skills_dir)
        .build()
        .await
        .expect("registry builds")
}

#[tokio::test]
async fn approved_only_in_tools_list_and_revoke_fails_at_call_time() {
    let tmp = TempDir::new("approve-revoke");
    write_skill(
        tmp.path(),
        "ship-it",
        "approved",
        "starter.ship_it_check",
        "Run pre-flight checks.",
    );
    write_skill(
        tmp.path(),
        "review",
        "quarantined",
        "starter.pr_review",
        "Review the open PR.",
    );

    let store: Arc<dyn ApprovalStore> = Arc::new(InMemoryApprovalStore::new());
    let skills = build_registry(tmp.path(), store.clone()).await;
    let registry = register_approved_skills(ToolRegistry::new(), &skills);

    // tools/list shows only the approved one.
    let names: Vec<String> = registry.list().into_iter().map(|d| d.name).collect();
    assert_eq!(names, vec!["starter.ship_it_check".to_string()]);

    // Quarantined skill is not even registered, so lookup returns None.
    assert!(registry.get("starter.pr_review").is_none());

    // Calling the approved one returns the verbatim body.
    let tool = registry.get("starter.ship_it_check").expect("registered");
    let out = tool.invoke(json!({})).await.expect("invoke ok");
    assert_eq!(out["body"], "Run pre-flight checks.\n");

    // Approve the quarantined bundle, rebuild, assert it now shows up
    // and invokes.
    // SkillRegistry::list_quarantined() is the operator-UI surface
    // for unapproved bundles — find ours there.
    let q = skills
        .list_quarantined()
        .into_iter()
        .find(|s| s.id.as_str() == "starter.pr_review")
        .expect("quarantined skill loaded");
    let row = ApprovalRow::now(q.id.clone(), q.bundle_hash.clone(), "operator-alice");
    store.record(row).await.unwrap();
    skills.reload().await.unwrap();

    let registry = register_approved_skills(ToolRegistry::new(), &skills);
    let names: Vec<String> = {
        let mut v: Vec<String> = registry.list().into_iter().map(|d| d.name).collect();
        v.sort();
        v
    };
    assert_eq!(
        names,
        vec![
            "starter.pr_review".to_string(),
            "starter.ship_it_check".to_string(),
        ]
    );

    let tool = registry.get("starter.pr_review").expect("now registered");
    let out = tool.invoke(json!({})).await.expect("invoke ok");
    assert_eq!(out["body"], "Review the open PR.\n");

    // Revoke through the registry — *without* rebuilding the
    // ToolRegistry — and assert the existing SkillTool fails at
    // call time. This is the load-bearing "re-check on every call"
    // guarantee from the design.
    skills
        .revoke(&q.id, &q.bundle_hash, &principal())
        .await
        .unwrap();
    let err = tool.invoke(json!({})).await.expect_err("revoked → forbidden");
    assert!(
        matches!(err, starter_spi::error::Error::Forbidden),
        "expected Forbidden, got {err:?}"
    );
}

#[tokio::test]
async fn skill_tool_definition_uses_skill_id_and_description() {
    let tmp = TempDir::new("definition");
    write_skill(
        tmp.path(),
        "x",
        "approved",
        "starter.example.x",
        "body x",
    );
    let store: Arc<dyn ApprovalStore> = Arc::new(InMemoryApprovalStore::new());
    let skills = build_registry(tmp.path(), store).await;
    let skill = skills.list().pop().unwrap();
    let tool = SkillTool::new(skill, skills.clone(), Arc::new(TracingSkillAuditSink));
    let def = tool.definition();
    assert_eq!(def.name, "starter.example.x");
    assert_eq!(def.description, "smoke starter.example.x");
    assert_eq!(def.input_schema["type"], "object");
    assert_eq!(def.input_schema["additionalProperties"], false);
}

#[tokio::test]
async fn add_favorite_writes_quarantined_bundle_not_in_tools_list() {
    let tmp = TempDir::new("addfav");
    let user_dir = tmp.path().join("user-skills");
    fs::create_dir_all(&user_dir).unwrap();

    let add_fav = AddFavoriteTool::new(&user_dir);
    let result = add_fav
        .invoke(json!({
            "id":          "starter.user.my_favorite",
            "description": "Custom workflow",
            "body":        "Do the thing the operator wants.",
        }))
        .await
        .expect("add_favorite ok");
    assert_eq!(result["skill_id"], "starter.user.my_favorite");
    assert_eq!(result["status"], "quarantined");
    assert!(result["bundle_hash"].as_str().unwrap().len() == 64);

    // The file landed on disk.
    let skill_md = user_dir
        .join("starter.user.my_favorite")
        .join("SKILL.md");
    assert!(skill_md.is_file(), "SKILL.md must exist at {skill_md:?}");

    // A registry that loads `user_dir` sees it as quarantined.
    let store: Arc<dyn ApprovalStore> = Arc::new(InMemoryApprovalStore::new());
    let skills = SkillRegistry::builder()
        .with_approval_store_arc(store)
        .load_dir_quarantined(&user_dir)
        .build()
        .await
        .expect("registry builds");
    let q_ids: Vec<String> = skills
        .list_quarantined()
        .iter()
        .map(|s| s.id.to_string())
        .collect();
    assert_eq!(q_ids, vec!["starter.user.my_favorite".to_string()]);
    assert!(skills.list().is_empty(), "must be quarantined, not approved");

    // The bridge does NOT register quarantined skills as tools.
    let registry = register_approved_skills(ToolRegistry::new(), &skills);
    assert!(registry.get("starter.user.my_favorite").is_none());
    assert!(registry.list().is_empty());
}
