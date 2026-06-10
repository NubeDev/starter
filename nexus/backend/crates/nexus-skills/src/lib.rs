//! Skill/rule knowledge loader (explicit, prompt-injection only).
//!
//! At session start an agent may name `skills` and `rules`; the named markdown
//! files are read from a configured knowledge root and rendered into a prompt
//! prefix prepended to the agent's base prompt. Ported in design from the
//! hcom-service `knowledge` module — same principles, no database:
//!
//! - **Explicit selection only.** An agent gets exactly the files it names — no
//!   trigger matching, no scoring. Those are additive layers that can sit on top
//!   later without changing the on-disk format or the selection schema.
//! - **Self-contained.** Pure filesystem + string rendering. No sqlx/Postgres,
//!   so the crate stays liftable. The file format is a plain `.md` body.
//! - **Tolerant.** A missing or unreadable file is logged and skipped; it never
//!   fails a launch. A reference to a removed skill degrades, not aborts.
//! - **Path-safe.** Names are sanitised: no absolute paths and no `..`
//!   components, so a caller cannot read files outside the knowledge root.

pub mod brevity;
mod store;

pub use brevity::BrevityMode;
pub use store::{KnowledgeFile, KnowledgeStore};

/// The valid `:kind` path segments for the knowledge surface (the on-disk
/// subdirs).
pub const KINDS: [&str; 2] = ["skills", "rules"];

/// What kind of knowledge entry a name refers to. Determines the subdirectory
/// (`skills/` vs `rules/`) and the label used in the rendered block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    Skill,
    Rule,
}

impl Kind {
    pub(crate) fn subdir(self) -> &'static str {
        match self {
            Kind::Skill => "skills",
            Kind::Rule => "rules",
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Kind::Skill => "Skill",
            Kind::Rule => "Rule",
        }
    }
}

/// One successfully loaded knowledge entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedEntry {
    pub(crate) kind: Kind,
    /// The name as written by the caller (e.g. `check`, `rust/quality`).
    pub name: String,
    /// The full markdown body of the file.
    pub content: String,
}

/// The result of resolving `skills`/`rules` lists against the knowledge root:
/// the entries that loaded plus the names that were requested but could not be
/// found (surfaced so callers can warn).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct KnowledgeBundle {
    pub entries: Vec<LoadedEntry>,
    pub missing: Vec<String>,
}

impl KnowledgeBundle {
    /// Render the loaded entries into a markdown prompt prefix, or `None` when
    /// nothing loaded. The prefix is self-delimiting so the agent can tell the
    /// injected guidance apart from its task, and ends with a separator so it
    /// concatenates cleanly before the base prompt.
    pub fn render_prompt_prefix(&self) -> Option<String> {
        if self.entries.is_empty() {
            return None;
        }
        let mut out = String::new();
        out.push_str(
            "# Project knowledge\n\nThe following skills and rules apply to this \
             task. Treat them as binding guidance, then complete the task that \
             follows.\n\n",
        );
        for entry in &self.entries {
            out.push_str(&format!("## {} `{}`\n\n", entry.kind.label(), entry.name));
            out.push_str(entry.content.trim_end());
            out.push_str("\n\n");
        }
        out.push_str("---\n\n");
        Some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(kind: Kind, name: &str, content: &str) -> LoadedEntry {
        LoadedEntry {
            kind,
            name: name.to_string(),
            content: content.to_string(),
        }
    }

    #[test]
    fn empty_bundle_renders_nothing() {
        assert!(KnowledgeBundle::default().render_prompt_prefix().is_none());
    }

    #[test]
    fn prefix_is_self_delimiting_and_separated() {
        let bundle = KnowledgeBundle {
            entries: vec![
                entry(Kind::Skill, "check", "# Check\nDo the thing."),
                entry(Kind::Rule, "rust/quality", "# Quality\nBe correct."),
            ],
            missing: vec![],
        };
        let out = bundle.render_prompt_prefix().unwrap();
        assert!(out.starts_with("# Project knowledge"));
        assert!(out.contains("## Skill `check`"));
        assert!(out.contains("## Rule `rust/quality`"));
        // Ends with the clean separator so it concatenates before the base prompt.
        assert!(out.trim_end().ends_with("---"));
    }
}
