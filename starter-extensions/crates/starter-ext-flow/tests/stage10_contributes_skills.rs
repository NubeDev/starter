//! Phase 6 / Stage 10 smoke: `starter-ext-flow` wires
//! `contributes.skills` through
//! [`SkillRegistry::extend`][starter_skills::SkillRegistry::extend].
//!
//! Setup: stub an extension manifest with a `contributes.skills`
//! block pointing at a `skills/` directory that holds a single
//! `SKILL.md` bundle whose frontmatter says `trust: approved`.
//!
//! Expectations (DOCS/agent/SKILLS.md R-skills-3 row 3):
//!
//! - The discovered bundle appears in
//!   [`SkillRegistry::list_quarantined`]
//!   [starter_skills::SkillRegistry::list_quarantined].
//! - It does **not** appear in
//!   [`SkillRegistry::list`][starter_skills::SkillRegistry::list]
//!   — the extension cannot self-approve, regardless of frontmatter
//!   `trust: approved`.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use starter_ext_flow::contributed_skills;
use starter_ext_spi::manifest::Manifest;
use starter_skills::{InMemoryApprovalStore, SkillRegistry};

/// Minimal tempdir helper — same shape as the registry tests in
/// `starter-skills`. Avoids pulling `tempfile` into the dep tree.
struct TempDir(PathBuf);

impl TempDir {
    fn new(tag: &str) -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let base = std::env::temp_dir().join(format!(
            "starter-ext-flow-{tag}-{}-{nanos}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(&base).unwrap();
        Self(base)
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

fn write_file(dir: &Path, rel: &str, body: &str) {
    let p = dir.join(rel);
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    let mut f = fs::File::create(&p).unwrap();
    f.write_all(body.as_bytes()).unwrap();
}

#[tokio::test]
async fn contributes_skills_lands_quarantined_via_extend() {
    let tmp = TempDir::new("contrib-skills");
    let ext_root = tmp.path();

    // 1. Stub the extension manifest. The shape is whatever
    //    `starter-ext-host` would have already validated and parsed:
    //    just the in-memory `Manifest` value with a
    //    `contributes.skills` block. We do not write a real
    //    `block.yaml` because this adapter consumes the parsed
    //    struct, not the on-disk YAML.
    let manifest_yaml = r#"
v: 1
id: com.acme.skilled
version: 0.0.1
display_name: "Skilled"
runtime: { kind: builtin, crate_name: skilled }
contributes:
  skills:
    - dir: skills/
"#;
    let manifest: Manifest = serde_yaml::from_str(manifest_yaml).expect("manifest parses");

    // 2. Drop a SKILL.md bundle under skills/greet/. Frontmatter
    //    explicitly asks for `trust: approved` — the matrix must
    //    ignore it for extension-contributed skills (R-skills-3
    //    row 3).
    write_file(
        ext_root,
        "skills/greet/SKILL.md",
        "---\nid: com.acme.skilled.greet\ndescription: \
         smoke fixture for stage 10\ntrust: approved\n---\nbody\n",
    );
    // A sibling directory without a SKILL.md should be ignored
    // (the one-level walker stops at directories that are not
    // bundles — see `starter-ext-flow::contributed_skills` docs).
    fs::create_dir_all(ext_root.join("skills/not-a-bundle/sub")).unwrap();
    write_file(ext_root, "skills/not-a-bundle/sub/README.md", "noise\n");

    // 3. Run the adapter against the parsed manifest + bundle root.
    let contributed = contributed_skills(&manifest, ext_root).expect("walk contributes.skills");
    assert_eq!(
        contributed.len(),
        1,
        "expected exactly one bundle under skills/ (the greet one); \
         not-a-bundle/ has no SKILL.md and must be skipped"
    );

    // 4. Feed into a registry via extend(...). No approval store
    //    rows yet ⇒ everything lands quarantined.
    let registry = SkillRegistry::builder()
        .with_approval_store(InMemoryApprovalStore::new())
        .extend(contributed)
        .build()
        .await
        .expect("registry builds");

    let approved_ids: Vec<String> = registry.list().iter().map(|s| s.id.to_string()).collect();
    let quarantined_ids: Vec<String> = registry
        .list_quarantined()
        .iter()
        .map(|s| s.id.to_string())
        .collect();

    assert_eq!(
        quarantined_ids,
        vec!["com.acme.skilled.greet".to_string()],
        "extension-contributed bundle must land in list_quarantined()"
    );
    assert!(
        approved_ids.is_empty(),
        "extension-contributed bundle must NOT land in list() despite \
         frontmatter trust: approved (R-skills-3 row 3); got {approved_ids:?}"
    );
}

#[tokio::test]
async fn contributes_skills_missing_dir_is_a_typed_error() {
    // A manifest pointing at a non-existent skills/ directory is an
    // operator-fixable mistake; the adapter must surface it as a
    // typed error naming the resolved path (not panic, not silently
    // return zero bundles — that would mask an installation bug).
    let tmp = TempDir::new("contrib-missing");
    let manifest_yaml = r#"
v: 1
id: com.acme.missing
version: 0.0.1
display_name: "Missing"
runtime: { kind: builtin, crate_name: missing }
contributes:
  skills:
    - dir: does-not-exist/
"#;
    let manifest: Manifest = serde_yaml::from_str(manifest_yaml).unwrap();
    let err = contributed_skills(&manifest, tmp.path()).expect_err("must fail");
    let rendered = format!("{err}");
    assert!(
        rendered.contains("does-not-exist"),
        "error must name the offending dir; got: {rendered}"
    );
}

#[tokio::test]
async fn manifest_without_contributes_skills_yields_empty_batch() {
    // A perfectly normal extension that contributes only (say) tools
    // must not error from this adapter — `contributes.skills` is
    // optional and defaults to empty.
    let tmp = TempDir::new("contrib-none");
    let manifest_yaml = r#"
v: 1
id: com.acme.nothing
version: 0.0.1
display_name: "Nothing"
runtime: { kind: builtin, crate_name: nothing }
"#;
    let manifest: Manifest = serde_yaml::from_str(manifest_yaml).unwrap();
    let out = contributed_skills(&manifest, tmp.path()).expect("ok");
    assert!(out.is_empty());
}
