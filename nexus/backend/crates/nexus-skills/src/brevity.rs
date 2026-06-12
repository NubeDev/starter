//! Response brevity — output-token discipline as an injected, prose-only rule.
//!
//! A small, local brevity rule injected through the same prompt-prefix seam the
//! knowledge loader uses. English-only (`lite`/`full` compression); there is no
//! non-English mode. The rule constrains the agent's *prose* only — code blocks,
//! file paths, command output, error strings, CLI help, and user-facing text
//! MUST stay verbatim, and the injected text says so explicitly.
//!
//! Ported from the hcom-service `knowledge::brevity` module, minus its
//! clap/`ValueEnum` and harness-specific cross-references — here it is plain
//! serde so it slots into a REST DTO or an agent config blob unchanged.

use serde::{Deserialize, Serialize};

/// Tri-state brevity toggle. `inherit` defers to a service/agent default;
/// `off`/`lite`/`full` override it. `inherit` is the serde default so configs
/// that omit it parse unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BrevityMode {
    #[default]
    Inherit,
    Off,
    Lite,
    Full,
}

impl BrevityMode {
    /// Resolve this toggle against a default. `inherit` takes the default; any
    /// explicit value overrides it. `inherit` only survives if the default is
    /// itself `inherit`.
    pub fn resolve(self, default: BrevityMode) -> BrevityMode {
        match self {
            BrevityMode::Inherit => default,
            other => other,
        }
    }

    /// The raw rule body for a resolved level, or `None` when no rule should be
    /// injected (`inherit` left unresolved, or `off`).
    fn rule_body(self) -> Option<&'static str> {
        match self {
            BrevityMode::Lite => Some(LITE_RULE),
            BrevityMode::Full => Some(FULL_RULE),
            BrevityMode::Inherit | BrevityMode::Off => None,
        }
    }

    /// Render a brevity prompt prefix for this mode, resolving it against
    /// `default` first. `None` when nothing should be injected. Self-delimiting
    /// and separator-terminated so it concatenates cleanly with the knowledge
    /// prefix and the base prompt, matching
    /// [`crate::KnowledgeBundle::render_prompt_prefix`].
    pub fn render_prompt_prefix(self, default: BrevityMode) -> Option<String> {
        let body = self.resolve(default).rule_body()?;
        let mut out = String::new();
        out.push_str(body.trim_end());
        out.push_str("\n\n---\n\n");
        Some(out)
    }
}

// The rule bodies are written out in full here (English only, ASCII) so the
// module stays dependency-free. Both share the same "preserve verbatim" clause.
const LITE_RULE: &str = "# Response brevity (lite)\n\n\
     Be concise. Prefer short, direct English. Omit filler, restated questions, \
     and unnecessary preamble or closing summaries. Use lists over prose where \
     that is clearer.\n\n\
     PRESERVE EXACTLY, never abbreviate or paraphrase: code blocks, file paths, \
     command output, error strings, CLI help text, identifiers, and any text \
     shown to an end user. Brevity applies to YOUR prose, not to literal \
     artifacts.";

const FULL_RULE: &str = "# Response brevity (full)\n\n\
     Maximize terseness. Answer in the fewest words that stay correct and \
     complete. Skip all preamble, restated context, and closing summaries. Use \
     sentence fragments and lists. Do not explain unless asked.\n\n\
     PRESERVE EXACTLY, never abbreviate or paraphrase: code blocks, file paths, \
     command output, error strings, CLI help text, identifiers, and any text \
     shown to an end user. Brevity applies to YOUR prose, not to literal \
     artifacts.";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inherit_takes_default_and_explicit_overrides() {
        assert_eq!(
            BrevityMode::Inherit.resolve(BrevityMode::Lite),
            BrevityMode::Lite
        );
        assert_eq!(
            BrevityMode::Off.resolve(BrevityMode::Full),
            BrevityMode::Off
        );
        assert_eq!(
            BrevityMode::Full.resolve(BrevityMode::Lite),
            BrevityMode::Full
        );
    }

    #[test]
    fn off_and_inherit_inject_nothing() {
        assert!(BrevityMode::Off
            .render_prompt_prefix(BrevityMode::Off)
            .is_none());
        // inherit resolving to off → nothing.
        assert!(BrevityMode::Inherit
            .render_prompt_prefix(BrevityMode::Off)
            .is_none());
    }

    #[test]
    fn lite_and_full_inject_a_separated_block() {
        let lite = BrevityMode::Lite
            .render_prompt_prefix(BrevityMode::Off)
            .unwrap();
        assert!(lite.starts_with("# Response brevity (lite)"));
        assert!(lite.trim_end().ends_with("---"));
        // inherit resolving to full via the default also injects.
        let full = BrevityMode::Inherit
            .render_prompt_prefix(BrevityMode::Full)
            .unwrap();
        assert!(full.starts_with("# Response brevity (full)"));
    }

    #[test]
    fn rule_preserves_literal_artifacts_clause() {
        for mode in [BrevityMode::Lite, BrevityMode::Full] {
            let text = mode.render_prompt_prefix(BrevityMode::Off).unwrap();
            assert!(text.contains("PRESERVE EXACTLY"));
            assert!(text.contains("code blocks"));
            assert!(text.contains("file paths"));
            assert!(text.contains("error strings"));
            assert!(text.contains("CLI help"));
        }
    }

    #[test]
    fn rules_are_english_ascii_only() {
        assert!(LITE_RULE.is_ascii());
        assert!(FULL_RULE.is_ascii());
    }
}
