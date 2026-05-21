//! `SKILL.md` frontmatter parser.
//!
//! A `SKILL.md` document is a YAML frontmatter block fenced by `---`
//! lines, followed by a Markdown body. The frontmatter is the only
//! part the parser inspects; the body is opaque text held verbatim
//! and eventually surfaced to the model (R-skills-1: no templating).
//!
//! Schema (deny_unknown_fields):
//!
//! ```yaml
//! ---
//! id: starter.example.greet
//! description: Greets the user.
//! allowed_tools: [starter.tool.echo]
//! model_hint: claude-3-5-sonnet   # optional
//! trust: approved                  # approved | quarantined
//! resources: [file://greeting.txt] # optional, file:// only in v1
//! ---
//! ```
//!
//! All schema violations (unknown key, wrong type, missing required
//! field, invalid id, unsupported URI scheme) return a structured
//! [`crate::SkillParseError`] that names the offending path.

use std::path::Path;

use serde::Deserialize;
use starter_flow_spi::node::KindId;
use starter_flow_spi::skill::SkillId;

use crate::error::SkillParseError;

/// V1 resource URI schemes (S-D2 locked). Adding a scheme is a
/// one-line edit here plus a parser test; broadening is additive
/// and backwards compatible. Narrowing later would break shipped
/// extensions, so this list is conservative on purpose.
pub const SUPPORTED_RESOURCE_SCHEMES: &[&str] = &["file"];

/// Trust hint authored into the frontmatter.
///
/// Note: the *effective* trust of a bundle is decided by the load
/// path (R-skills-3 trust matrix), not by this field alone:
/// `extend(...)` is always quarantined regardless of what the
/// frontmatter says. Future stages enforce that matrix; the parser
/// only records what the author wrote.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Trust {
    /// Author asserts the bundle is safe to run without operator
    /// approval. Only honoured when loaded via `load_dir(...)`.
    ///
    /// Default per R-skills-3 row 1 ("load_dir(...) + frontmatter
    /// approved/absent → approved").
    #[default]
    Approved,
    /// Author asks the operator to approve before the bundle runs.
    /// The host **may** elevate this to "approved" via an
    /// `ApprovalStore` row; it may not silently downgrade.
    Quarantined,
}

/// Raw frontmatter as deserialised from the YAML block. Strings stay
/// as strings; validation (id shape, URI scheme, tool ids) runs in
/// [`parse_skill_md`] so the deny_unknown_fields YAML pass keeps a
/// single failure mode (`InvalidFrontmatter`).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Frontmatter {
    /// Reverse-DNS skill id; validated against
    /// [`starter_flow_spi::SkillId`].
    pub id: String,
    /// Free-form human-readable description (surfaced to the
    /// selector and to operator UIs).
    pub description: String,
    /// Tool-id allowlist. Each entry must parse as
    /// [`starter_flow_spi::node::KindId`].
    #[serde(default)]
    pub allowed_tools: Vec<String>,
    /// Documentary model preference (S-D3: best-effort
    /// pass-through, never blocks a run).
    #[serde(default)]
    pub model_hint: Option<String>,
    /// Author's trust hint; the load path may override this.
    #[serde(default)]
    pub trust: Trust,
    /// Relative paths or `file://` URIs of resource files the
    /// `ai-agent` body mounts at run time. V1: `file://` only.
    #[serde(default)]
    pub resources: Vec<String>,
}

/// Parsed `SKILL.md` — the frontmatter with every field validated,
/// plus the body string read verbatim.
#[derive(Debug, Clone)]
pub struct ParsedSkill {
    /// Validated reverse-DNS id.
    pub id: SkillId,
    /// Free-form description (unchanged from the YAML).
    pub description: String,
    /// Validated tool-id allowlist.
    pub allowed_tools: Vec<KindId>,
    /// Optional documentary model hint.
    pub model_hint: Option<String>,
    /// Author-declared trust (the load path can still override).
    pub trust: Trust,
    /// Resource URIs the body references. Each is guaranteed to
    /// start with a scheme in [`SUPPORTED_RESOURCE_SCHEMES`].
    pub resources: Vec<String>,
    /// Markdown body verbatim — *no* templating, *no* interpolation.
    /// The bytes the model will eventually see (R-skills-1).
    pub body: String,
}

/// Split a `SKILL.md` document into (raw frontmatter YAML, body).
///
/// The split rule is intentionally strict: the first line must be
/// exactly `---`, and the next `---` line on its own terminates the
/// frontmatter block. Everything after the closing fence (with any
/// single trailing newline consumed) is the body.
fn split_frontmatter<'a>(
    skill_path: &Path,
    src: &'a str,
) -> Result<(&'a str, &'a str), SkillParseError> {
    // Strip a leading UTF-8 BOM if the editor added one — we want
    // the structural check that follows to fire on real content.
    let src = src.strip_prefix('\u{feff}').unwrap_or(src);

    // The first line must be exactly "---" (optionally followed by
    // \r). Anything else is a malformed-delimiter error.
    let first_line_end = src.find('\n').unwrap_or(src.len());
    let first_line = src[..first_line_end].trim_end_matches('\r');
    if first_line != "---" {
        return Err(SkillParseError::MalformedFrontmatter {
            skill_path: skill_path.to_path_buf(),
            reason: "missing opening `---` on first line",
        });
    }
    let after_open = &src[first_line_end..]
        .strip_prefix('\n')
        .unwrap_or(&src[first_line_end..]);

    // Find the closing fence: a line containing exactly "---".
    let mut cursor = 0usize;
    loop {
        let rest = &after_open[cursor..];
        let line_end = rest.find('\n').unwrap_or(rest.len());
        let line = rest[..line_end].trim_end_matches('\r');
        if line == "---" {
            let yaml = &after_open[..cursor];
            let after_close_start = cursor + line_end;
            let mut body_start = after_close_start;
            // Consume one trailing newline so a body that starts
            // "Hello\n..." doesn't show up as "\nHello\n...".
            if after_open.as_bytes().get(body_start) == Some(&b'\n') {
                body_start += 1;
            } else if after_open.as_bytes().get(body_start) == Some(&b'\r')
                && after_open.as_bytes().get(body_start + 1) == Some(&b'\n')
            {
                body_start += 2;
            }
            let body = &after_open[body_start..];
            return Ok((yaml, body));
        }
        if line_end == rest.len() {
            // EOF before a closing fence.
            return Err(SkillParseError::MalformedFrontmatter {
                skill_path: skill_path.to_path_buf(),
                reason: "missing closing `---`",
            });
        }
        cursor += line_end + 1;
    }
}

/// Parse the bytes of a `SKILL.md` document at `skill_path`.
///
/// The `skill_path` argument is used only for error reporting — the
/// function does no I/O. The caller (typically
/// [`crate::load_bundle`]) is responsible for reading the bytes.
pub fn parse_skill_md(skill_path: &Path, src: &str) -> Result<ParsedSkill, SkillParseError> {
    let (yaml, body) = split_frontmatter(skill_path, src)?;

    let raw: Frontmatter = serde_yaml::from_str(yaml).map_err(|source| {
        SkillParseError::InvalidFrontmatter {
            skill_path: skill_path.to_path_buf(),
            source,
        }
    })?;

    let id = SkillId::new(raw.id.clone()).map_err(|e| SkillParseError::InvalidSkillId {
        skill_path: skill_path.to_path_buf(),
        reason: e.to_string(),
    })?;

    let mut allowed_tools = Vec::with_capacity(raw.allowed_tools.len());
    for tool in raw.allowed_tools {
        let kid = KindId::new(tool.clone()).map_err(|e| SkillParseError::InvalidAllowedTool {
            skill_path: skill_path.to_path_buf(),
            value: tool,
            reason: e.to_string(),
        })?;
        allowed_tools.push(kid);
    }

    for uri in &raw.resources {
        validate_resource_scheme(skill_path, uri)?;
    }

    Ok(ParsedSkill {
        id,
        description: raw.description,
        allowed_tools,
        model_hint: raw.model_hint,
        trust: raw.trust,
        resources: raw.resources,
        body: body.to_owned(),
    })
}

/// Ensure `uri` uses one of [`SUPPORTED_RESOURCE_SCHEMES`]. The
/// check is deliberately syntactic (split on `://`) — full URI
/// validation lands when the bundle walker resolves the file.
pub(crate) fn validate_resource_scheme(
    skill_path: &Path,
    uri: &str,
) -> Result<(), SkillParseError> {
    let (scheme, _rest) = match uri.split_once("://") {
        Some(parts) => parts,
        None => {
            return Err(SkillParseError::UnsupportedResourceScheme {
                skill_path: skill_path.to_path_buf(),
                resource_uri: uri.to_owned(),
                scheme: String::new(),
            });
        }
    };
    if SUPPORTED_RESOURCE_SCHEMES.contains(&scheme) {
        Ok(())
    } else {
        Err(SkillParseError::UnsupportedResourceScheme {
            skill_path: skill_path.to_path_buf(),
            resource_uri: uri.to_owned(),
            scheme: scheme.to_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn path() -> PathBuf {
        PathBuf::from("/tmp/test/SKILL.md")
    }

    #[test]
    fn happy_path_parses_full_frontmatter() {
        let src = "\
---
id: starter.example.greet
description: Greets the user.
allowed_tools:
  - starter.tool.echo
model_hint: claude-3-5-sonnet
trust: approved
resources:
  - file://greeting.txt
---
Hello {{name}} — this body is literal text.
";
        let parsed = parse_skill_md(&path(), src).expect("parse ok");
        assert_eq!(parsed.id.as_str(), "starter.example.greet");
        assert_eq!(parsed.description, "Greets the user.");
        assert_eq!(parsed.allowed_tools.len(), 1);
        assert_eq!(parsed.allowed_tools[0].as_str(), "starter.tool.echo");
        assert_eq!(parsed.model_hint.as_deref(), Some("claude-3-5-sonnet"));
        assert_eq!(parsed.trust, Trust::Approved);
        assert_eq!(parsed.resources, vec!["file://greeting.txt".to_string()]);
        // R-skills-1: body is verbatim, `{{name}}` is NOT expanded.
        assert!(parsed.body.contains("{{name}}"));
    }

    #[test]
    fn missing_trust_defaults_to_approved() {
        let src = "\
---
id: starter.example.x
description: x
---
body
";
        let parsed = parse_skill_md(&path(), src).expect("parse ok");
        assert_eq!(parsed.trust, Trust::Approved);
        assert!(parsed.allowed_tools.is_empty());
        assert!(parsed.resources.is_empty());
        assert!(parsed.model_hint.is_none());
    }

    #[test]
    fn quarantined_trust_round_trips() {
        let src = "\
---
id: starter.example.x
description: x
trust: quarantined
---
body
";
        let parsed = parse_skill_md(&path(), src).expect("parse ok");
        assert_eq!(parsed.trust, Trust::Quarantined);
    }

    #[test]
    fn deny_unknown_fields_rejects_extra_key() {
        let src = "\
---
id: starter.example.x
description: x
extra_key: nope
---
body
";
        let err = parse_skill_md(&path(), src).expect_err("must reject");
        match err {
            SkillParseError::InvalidFrontmatter { skill_path, .. } => {
                assert_eq!(skill_path, path());
            }
            other => panic!("wrong variant: {other:?}"),
        }
    }

    #[test]
    fn missing_id_is_rejected() {
        let src = "\
---
description: x
---
body
";
        let err = parse_skill_md(&path(), src).expect_err("must reject");
        match err {
            SkillParseError::InvalidFrontmatter { skill_path, .. } => {
                assert_eq!(skill_path, path());
            }
            other => panic!("wrong variant: {other:?}"),
        }
    }

    #[test]
    fn invalid_id_shape_is_rejected_with_path() {
        let src = "\
---
id: not a reverse dns
description: x
---
body
";
        let err = parse_skill_md(&path(), src).expect_err("must reject");
        match err {
            SkillParseError::InvalidSkillId { skill_path, .. } => {
                assert_eq!(skill_path, path());
            }
            other => panic!("wrong variant: {other:?}"),
        }
    }

    #[test]
    fn invalid_tool_id_is_rejected() {
        let src = "\
---
id: starter.example.x
description: x
allowed_tools: [\"not a kind\"]
---
body
";
        let err = parse_skill_md(&path(), src).expect_err("must reject");
        assert!(matches!(err, SkillParseError::InvalidAllowedTool { .. }));
    }

    #[test]
    fn unsupported_scheme_is_rejected_at_parse_time() {
        let src = "\
---
id: starter.example.x
description: x
resources:
  - s3://bucket/key
---
body
";
        let err = parse_skill_md(&path(), src).expect_err("must reject");
        match err {
            SkillParseError::UnsupportedResourceScheme { scheme, .. } => {
                assert_eq!(scheme, "s3");
            }
            other => panic!("wrong variant: {other:?}"),
        }
    }

    #[test]
    fn bare_relative_path_is_rejected() {
        let src = "\
---
id: starter.example.x
description: x
resources:
  - just/a/path.txt
---
body
";
        let err = parse_skill_md(&path(), src).expect_err("must reject");
        match err {
            SkillParseError::UnsupportedResourceScheme { scheme, .. } => {
                assert_eq!(scheme, "");
            }
            other => panic!("wrong variant: {other:?}"),
        }
    }

    #[test]
    fn missing_opening_fence_is_rejected() {
        let src = "no frontmatter here\n";
        let err = parse_skill_md(&path(), src).expect_err("must reject");
        assert!(matches!(err, SkillParseError::MalformedFrontmatter { .. }));
    }

    #[test]
    fn missing_closing_fence_is_rejected() {
        let src = "---\nid: starter.x.y\ndescription: x\n";
        let err = parse_skill_md(&path(), src).expect_err("must reject");
        assert!(matches!(err, SkillParseError::MalformedFrontmatter { .. }));
    }

    #[test]
    fn crlf_frontmatter_delimiters_parse() {
        let src = "---\r\nid: starter.example.x\r\ndescription: x\r\n---\r\nbody\r\n";
        let parsed = parse_skill_md(&path(), src).expect("parse ok");
        assert_eq!(parsed.id.as_str(), "starter.example.x");
    }
}
