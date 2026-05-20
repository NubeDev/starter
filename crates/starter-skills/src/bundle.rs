//! Bundle directory walker.
//!
//! A bundle is a directory containing exactly one `SKILL.md` plus
//! zero or more resource files referenced from the frontmatter's
//! `resources:` list. [`load_bundle`] reads the `SKILL.md`, parses
//! the frontmatter, then reads every listed resource into memory.
//!
//! Phase 1 invariant: a bundle is loaded **once** and the bytes are
//! held by `Arc<[u8]>`. Later stages add the content-hash + frozen
//! `ResourceRef` plumbing; this stage just produces an in-memory
//! [`Bundle`] value that the parser tests can exercise.

use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use crate::error::SkillParseError;
use crate::parser::{parse_skill_md, ParsedSkill};

/// One resource file as read from disk.
///
/// `uri` is preserved verbatim (e.g. `file://docs/style.md`) so the
/// content-hash and `ResourceRef` plumbing in later stages can frame
/// each entry without re-deriving the canonical form.
#[derive(Debug, Clone)]
pub struct Resource {
    /// Raw URI as written in the frontmatter (always `file://…`
    /// in v1 — other schemes parse-fail).
    pub uri: String,
    /// Bundle-relative path the URI resolved to, with `/`
    /// separators. The walker guarantees this stays inside the
    /// bundle root.
    pub relative_path: String,
    /// File bytes read once at load time; held by `Arc` so the
    /// registry can hand out cheap clones.
    pub bytes: Arc<[u8]>,
}

/// One loaded bundle: parsed `SKILL.md` plus every resource read
/// into memory.
#[derive(Debug, Clone)]
pub struct Bundle {
    /// Bundle directory on disk.
    pub root: PathBuf,
    /// Path of the `SKILL.md` inside [`Bundle::root`].
    pub skill_path: PathBuf,
    /// Parsed frontmatter + verbatim body.
    pub skill: ParsedSkill,
    /// Resources in the same order they appeared in the
    /// frontmatter (the hash algorithm sorts later; this list
    /// preserves authoring order for diagnostics).
    pub resources: Vec<Resource>,
}

/// Load a bundle from `bundle_root`.
///
/// Fails with a structured [`SkillParseError`] if:
///
/// - the directory has no `SKILL.md`,
/// - the frontmatter is malformed / violates `deny_unknown_fields`,
/// - a resource URI uses an unsupported scheme (S-D2),
/// - a resource path escapes the bundle root,
/// - any I/O step fails.
pub fn load_bundle(bundle_root: impl AsRef<Path>) -> Result<Bundle, SkillParseError> {
    let bundle_root = bundle_root.as_ref().to_path_buf();
    let skill_path = bundle_root.join("SKILL.md");
    if !skill_path.is_file() {
        return Err(SkillParseError::MissingSkillMd {
            bundle_root: bundle_root.clone(),
        });
    }

    let src = fs::read_to_string(&skill_path).map_err(|source| SkillParseError::Io {
        path: skill_path.clone(),
        source,
    })?;
    let parsed = parse_skill_md(&skill_path, &src)?;

    let mut resources = Vec::with_capacity(parsed.resources.len());
    for uri in &parsed.resources {
        let resource = load_resource(&bundle_root, &skill_path, uri)?;
        resources.push(resource);
    }

    Ok(Bundle {
        root: bundle_root,
        skill_path,
        skill: parsed,
        resources,
    })
}

/// Resolve a `file://` URI against the bundle root and read it.
///
/// Path safety: the post-`file://` portion is split into components
/// and any `..` or absolute prefix is rejected so a malicious bundle
/// cannot reach files outside its directory at load time. This
/// complements the later stages' resource hash check; defence in
/// depth is intentional.
fn load_resource(
    bundle_root: &Path,
    skill_path: &Path,
    uri: &str,
) -> Result<Resource, SkillParseError> {
    // The parser already vetted the scheme; double-check here so
    // the walker is independently safe.
    let rest = uri.strip_prefix("file://").ok_or_else(|| {
        SkillParseError::UnsupportedResourceScheme {
            skill_path: skill_path.to_path_buf(),
            resource_uri: uri.to_owned(),
            scheme: uri.split_once("://").map(|(s, _)| s).unwrap_or("").to_owned(),
        }
    })?;

    let relative = Path::new(rest);
    if relative.is_absolute() {
        return Err(SkillParseError::ResourcePathEscapesBundle {
            skill_path: skill_path.to_path_buf(),
            resource_uri: uri.to_owned(),
        });
    }
    let mut normalised = PathBuf::new();
    for comp in relative.components() {
        match comp {
            Component::Normal(seg) => normalised.push(seg),
            Component::CurDir => {}
            Component::ParentDir
            | Component::RootDir
            | Component::Prefix(_) => {
                return Err(SkillParseError::ResourcePathEscapesBundle {
                    skill_path: skill_path.to_path_buf(),
                    resource_uri: uri.to_owned(),
                });
            }
        }
    }
    let absolute = bundle_root.join(&normalised);
    let bytes = fs::read(&absolute).map_err(|source| SkillParseError::Io {
        path: absolute.clone(),
        source,
    })?;

    // Always emit forward slashes for the canonical relative path so
    // downstream (hash algorithm, ResourceRef.uri reconstruction)
    // sees the same shape on Windows + Unix.
    let rel_str = normalised
        .components()
        .filter_map(|c| match c {
            Component::Normal(s) => s.to_str(),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/");

    Ok(Resource {
        uri: uri.to_owned(),
        relative_path: rel_str,
        bytes: Arc::from(bytes.into_boxed_slice()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;

    struct TempDir(PathBuf);
    impl TempDir {
        fn new(tag: &str) -> Self {
            let base = std::env::temp_dir().join(format!(
                "starter-skills-{tag}-{}",
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

    fn write(dir: &Path, name: &str, body: &str) {
        let p = dir.join(name);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut f = fs::File::create(&p).unwrap();
        f.write_all(body.as_bytes()).unwrap();
    }

    #[test]
    fn missing_skill_md_is_rejected_by_path() {
        let tmp = TempDir::new("missing");
        let err = load_bundle(tmp.path()).expect_err("must reject");
        match err {
            SkillParseError::MissingSkillMd { bundle_root } => {
                assert_eq!(bundle_root, tmp.path());
            }
            other => panic!("wrong variant: {other:?}"),
        }
    }

    #[test]
    fn happy_path_reads_skill_and_resource() {
        let tmp = TempDir::new("happy");
        write(
            tmp.path(),
            "SKILL.md",
            "---\nid: starter.example.greet\ndescription: x\nresources:\n  - file://greeting.txt\n---\nbody\n",
        );
        write(tmp.path(), "greeting.txt", "Hello!\n");

        let bundle = load_bundle(tmp.path()).expect("loads");
        assert_eq!(bundle.skill.id.as_str(), "starter.example.greet");
        assert_eq!(bundle.resources.len(), 1);
        assert_eq!(bundle.resources[0].relative_path, "greeting.txt");
        assert_eq!(&*bundle.resources[0].bytes, b"Hello!\n");
    }

    #[test]
    fn parent_dir_traversal_is_rejected() {
        let tmp = TempDir::new("traversal");
        write(
            tmp.path(),
            "SKILL.md",
            "---\nid: starter.example.x\ndescription: x\nresources:\n  - file://../escape.txt\n---\nbody\n",
        );
        let err = load_bundle(tmp.path()).expect_err("must reject");
        assert!(matches!(
            err,
            SkillParseError::ResourcePathEscapesBundle { .. }
        ));
    }

    #[test]
    fn missing_resource_file_returns_io_error_with_path() {
        let tmp = TempDir::new("missing-res");
        write(
            tmp.path(),
            "SKILL.md",
            "---\nid: starter.example.x\ndescription: x\nresources:\n  - file://nope.txt\n---\nbody\n",
        );
        let err = load_bundle(tmp.path()).expect_err("must fail");
        match err {
            SkillParseError::Io { path, .. } => {
                assert!(path.ends_with("nope.txt"));
            }
            other => panic!("wrong variant: {other:?}"),
        }
    }
}
