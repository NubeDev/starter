//! `KnowledgeStore`: resolves and reads named skill/rule files from a root.

use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::Serialize;

use super::{Kind, KnowledgeBundle, LoadedEntry};

/// One discovered knowledge file, as listed by the knowledge surface. `name` is
/// the load name (no `.md`, nested paths kept, e.g. `rust/quality`).
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct KnowledgeFile {
    pub name: String,
    pub bytes: u64,
    /// Last-modified time in unix epoch seconds, or 0 when unavailable.
    pub modified: i64,
}

/// Reads named skill/rule markdown files from a knowledge root directory.
///
/// The root is expected to contain `skills/` and `rules/` subdirectories. The
/// store holds only the root path; every load re-reads from disk so edits to a
/// skill file take effect on the next session without a service restart.
#[derive(Debug, Clone)]
pub struct KnowledgeStore {
    root: PathBuf,
}

impl KnowledgeStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        KnowledgeStore { root: root.into() }
    }

    /// Resolve and read the named skills and rules. Missing/unsafe names are
    /// collected into `bundle.missing` rather than erroring.
    pub fn load(&self, skills: &[String], rules: &[String]) -> KnowledgeBundle {
        let mut bundle = KnowledgeBundle::default();
        for name in skills {
            self.load_one(Kind::Skill, name, &mut bundle);
        }
        for name in rules {
            self.load_one(Kind::Rule, name, &mut bundle);
        }
        bundle
    }

    /// Discover the `.md` files under `{root}/{kind}/`, returned sorted by name.
    /// Nested files keep their relative path as the name (`rust/quality`). Walks
    /// recursively; a missing root subdir yields an empty list (never errors).
    /// `kind` is the subdir label (`skills`|`rules`); unknown kinds yield `[]`.
    pub fn list(&self, kind: &str) -> Vec<KnowledgeFile> {
        let Some(kind) = Self::kind_from_label(kind) else {
            return Vec::new();
        };
        let base = self.root.join(kind.subdir());
        let mut out = Vec::new();
        walk_markdown(&base, &base, &mut out);
        out.sort_by(|a, b| a.name.cmp(&b.name));
        out
    }

    /// Read the full markdown body of a named knowledge file, or `None` if the
    /// kind is unknown, the name is unsafe, or the file does not exist. Reuses
    /// the same path-safety [`KnowledgeStore::resolve`] sanitiser as load.
    pub fn read(&self, kind: &str, name: &str) -> Option<String> {
        let kind = Self::kind_from_label(kind)?;
        let path = self.resolve(kind, name)?;
        std::fs::read_to_string(path).ok()
    }

    /// Map a subdir label (`skills`|`rules`) to its [`Kind`], or `None`.
    fn kind_from_label(label: &str) -> Option<Kind> {
        match label {
            "skills" => Some(Kind::Skill),
            "rules" => Some(Kind::Rule),
            _ => None,
        }
    }

    fn load_one(&self, kind: Kind, name: &str, bundle: &mut KnowledgeBundle) {
        let Some(path) = self.resolve(kind, name) else {
            tracing::warn!(
                kind = kind.label(),
                name,
                "rejected unsafe knowledge name (absolute or contains `..`)"
            );
            bundle.missing.push(name.to_string());
            return;
        };
        match std::fs::read_to_string(&path) {
            Ok(content) => bundle.entries.push(LoadedEntry {
                kind,
                name: name.to_string(),
                content,
            }),
            Err(e) => {
                tracing::warn!(
                    kind = kind.label(),
                    name,
                    path = %path.display(),
                    error = %e,
                    "knowledge file not loaded; skipping"
                );
                bundle.missing.push(name.to_string());
            }
        }
    }

    /// Map a name to its on-disk path, or `None` if the name is unsafe.
    ///
    /// Accepts nested names (`rust/quality`) but rejects absolute paths and any
    /// `..` component so the result is always confined under `{root}/{subdir}/`.
    pub(super) fn resolve(&self, kind: Kind, name: &str) -> Option<PathBuf> {
        let name = name.trim();
        if name.is_empty() {
            return None;
        }
        let rel = Path::new(name);
        if rel.is_absolute() {
            return None;
        }
        if rel.components().any(|c| {
            matches!(
                c,
                std::path::Component::ParentDir | std::path::Component::Prefix(_)
            )
        }) {
            return None;
        }
        Some(
            self.root
                .join(kind.subdir())
                .join(name)
                .with_extension("md"),
        )
    }
}

/// Recursively collect `.md` files under `dir`, naming each by its path relative
/// to `base` with the `.md` extension stripped (so `base/rust/quality.md` →
/// `rust/quality`). Tolerant: unreadable dirs/entries are skipped, not fatal.
fn walk_markdown(base: &Path, dir: &Path, out: &mut Vec<KnowledgeFile>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_markdown(base, &path, out);
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let Ok(rel) = path.strip_prefix(base) else {
            continue;
        };
        // Name = relative path without the `.md` extension, forward-slashed.
        let name = rel
            .with_extension("")
            .components()
            .filter_map(|c| c.as_os_str().to_str())
            .collect::<Vec<_>>()
            .join("/");
        if name.is_empty() {
            continue;
        }
        let meta = entry.metadata().ok();
        let bytes = meta.as_ref().map(|m| m.len()).unwrap_or(0);
        let modified = meta
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        out.push(KnowledgeFile {
            name,
            bytes,
            modified,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn fixture() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("skills/rust")).unwrap();
        fs::create_dir_all(root.join("rules")).unwrap();
        fs::write(root.join("skills/check.md"), "# Check\n").unwrap();
        fs::write(root.join("skills/rust/quality.md"), "# Quality\n").unwrap();
        fs::write(root.join("skills/notes.txt"), "ignored\n").unwrap();
        fs::write(root.join("rules/style.md"), "# Style\n").unwrap();
        dir
    }

    #[test]
    fn lists_markdown_recursively_sorted_and_named() {
        let dir = fixture();
        let store = KnowledgeStore::new(dir.path());
        let skills = store.list("skills");
        let names: Vec<_> = skills.iter().map(|f| f.name.as_str()).collect();
        // Nested name kept, `.txt` ignored, sorted.
        assert_eq!(names, vec!["check", "rust/quality"]);
        assert!(skills[0].bytes > 0);
        let rules = store.list("rules");
        assert_eq!(
            rules.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(),
            vec!["style"]
        );
    }

    #[test]
    fn unknown_kind_and_missing_root_yield_empty() {
        let dir = fixture();
        let store = KnowledgeStore::new(dir.path());
        assert!(store.list("bogus").is_empty());
        let absent = KnowledgeStore::new(dir.path().join("does-not-exist"));
        assert!(absent.list("skills").is_empty());
    }

    #[test]
    fn read_returns_body_for_nested_name_and_none_for_unsafe() {
        let dir = fixture();
        let store = KnowledgeStore::new(dir.path());
        assert_eq!(
            store.read("skills", "rust/quality").as_deref(),
            Some("# Quality\n")
        );
        assert_eq!(store.read("skills", "missing"), None);
        // Path traversal is rejected by the shared sanitiser.
        assert_eq!(store.read("skills", "../rules/style"), None);
        assert_eq!(store.read("bogus", "check"), None);
    }

    #[test]
    fn load_collects_entries_and_missing() {
        let dir = fixture();
        let store = KnowledgeStore::new(dir.path());
        let bundle = store.load(
            &["check".into(), "nope".into(), "../escape".into()],
            &["style".into()],
        );
        assert_eq!(bundle.entries.len(), 2); // check + style
        assert_eq!(bundle.missing, vec!["nope", "../escape"]);
    }
}
