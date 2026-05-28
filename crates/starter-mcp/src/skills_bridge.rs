//! Bridge from `starter-skills` to the MCP `Tool` surface.
//!
//! Behind `feature = "skills"`. Wraps every approved
//! [`starter_skills::Skill`] in a [`SkillTool`] and folds it into a
//! [`ToolRegistry`] via [`register_approved_skills`]. Quarantined
//! bundles are not exposed. The adapter re-checks the approval at
//! invoke time, so an operator `revoke` takes effect immediately
//! without restarting the server.
//!
//! Audit goes through [`SkillAuditSink`]. The default
//! [`TracingSkillAuditSink`] writes one `tracing::info!` per call;
//! consumers that want changelog-backed audit wire their own impl.
//!
//! `add_favorite` ([`AddFavoriteTool`]) writes a new `SKILL.md` into
//! a configured user-skills directory and **leaves it unapproved**.
//! There is no path by which the LLM can mint an approved skill.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{json, Value};
use starter_skills::{Skill, SkillRegistry};
use starter_spi::error::{Error, Result};
use starter_spi::tool::{Tool, ToolDefinition};

use crate::registry::{
    Prompt, PromptDefinition, PromptMessage, PromptResponse, PromptRole,
};
use crate::ToolRegistry;

/// One audit record emitted per `SkillTool::invoke` after the
/// approval check passes.
#[derive(Debug, Clone)]
pub struct SkillInvocation<'a> {
    /// Skill id whose body was returned.
    pub skill_id: &'a str,
    /// Bundle hash that was approved at call time.
    pub bundle_hash: &'a str,
    /// Caller-supplied input (v1 ignores it but audit still records).
    pub input: &'a Value,
    /// Unix milliseconds at the moment of invocation.
    pub at_unix_ms: u64,
}

/// Sink for `SkillTool::invoke` audit records. The default
/// [`TracingSkillAuditSink`] writes structured `tracing::info!`
/// events; consumers that want a changelog-backed audit row wire
/// their own impl (e.g. one that calls into `starter-changelog`).
pub trait SkillAuditSink: Send + Sync + 'static {
    /// Record one successful invocation. The sink must not panic;
    /// I/O errors are the sink's responsibility to surface or
    /// swallow.
    fn record(&self, invocation: SkillInvocation<'_>);
}

/// Default sink — writes one structured `tracing::info!` per call.
#[derive(Debug, Default)]
pub struct TracingSkillAuditSink;

impl SkillAuditSink for TracingSkillAuditSink {
    fn record(&self, invocation: SkillInvocation<'_>) {
        tracing::info!(
            target: "starter_mcp::skills_bridge::audit",
            skill_id = invocation.skill_id,
            bundle_hash = invocation.bundle_hash,
            at_unix_ms = invocation.at_unix_ms,
            "skill tool invoked"
        );
    }
}

/// MCP `Tool` adapter for an approved [`Skill`].
///
/// `definition()` exposes the skill id as the tool name (no
/// mangling — `SkillId` and `ToolDefinition.name` share the same
/// snake-case-dot-namespaced shape), the skill description as the
/// tool description, and an empty-object input schema. v1 is
/// argumentless; a frontmatter-driven arg schema is v2.
///
/// `invoke()` re-checks membership in `SkillRegistry::list()`
/// before returning the body — a `revoke()` or `reload()` that
/// re-quarantines the bundle takes effect on the next call without
/// a server restart. Going through the registry (rather than the
/// raw `ApprovalStore`) is what makes the re-check correct for
/// *both* frontmatter-approved bundles and store-row-approved
/// ones: the registry encapsulates the trust matrix.
pub struct SkillTool {
    skill: Arc<Skill>,
    skills: SkillRegistry,
    audit: Arc<dyn SkillAuditSink>,
}

impl SkillTool {
    /// Wrap an approved [`Skill`] with the registry used to
    /// re-check it at call time and an audit sink for invocation
    /// records. `skills` is the same registry the skill was sourced
    /// from; the adapter clones the `Arc`-backed handle so
    /// concurrent reloads remain visible.
    pub fn new(skill: Arc<Skill>, skills: SkillRegistry, audit: Arc<dyn SkillAuditSink>) -> Self {
        Self {
            skill,
            skills,
            audit,
        }
    }
}

#[async_trait]
impl Tool for SkillTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: self.skill.id.to_string(),
            description: self.skill.description.clone(),
            input_schema: empty_object_schema(),
        }
    }

    async fn invoke(&self, input: Value) -> Result<Value> {
        // Re-check that this exact `(skill_id, bundle_hash)` pair
        // is still in the approved set. Revoke or a re-quarantining
        // reload removes it; we fail closed in either case.
        let still_approved = self
            .skills
            .list()
            .iter()
            .any(|s| s.id == self.skill.id && s.bundle_hash == self.skill.bundle_hash);
        if !still_approved {
            return Err(Error::Forbidden);
        }
        self.audit.record(SkillInvocation {
            skill_id: self.skill.id.as_str(),
            bundle_hash: &self.skill.bundle_hash,
            input: &input,
            at_unix_ms: now_unix_ms(),
        });
        Ok(json!({ "body": self.skill.body.as_ref() }))
    }
}

/// Fold every approved skill from `skills` into `registry`. Uses
/// [`TracingSkillAuditSink`] as the audit sink — for a custom sink
/// see [`register_approved_skills_with_audit`].
///
/// `SkillRegistry::list()` already returns the approved set;
/// quarantined bundles are **not** registered.
pub fn register_approved_skills(registry: ToolRegistry, skills: &SkillRegistry) -> ToolRegistry {
    register_approved_skills_with_audit(registry, skills, Arc::new(TracingSkillAuditSink))
}

/// As [`register_approved_skills`], but with a caller-supplied
/// audit sink. Use this when the host wants changelog-backed audit.
pub fn register_approved_skills_with_audit(
    registry: ToolRegistry,
    skills: &SkillRegistry,
    audit: Arc<dyn SkillAuditSink>,
) -> ToolRegistry {
    skills.list().into_iter().fold(registry, |reg, skill| {
        reg.register_arc(Arc::new(SkillTool::new(
            skill,
            skills.clone(),
            audit.clone(),
        )))
    })
}

// ---------- prompts surface ----------

/// MCP `Prompt` adapter for an approved [`Skill`].
///
/// Mirrors [`SkillTool`] for the `prompts/*` surface: hosts that
/// map prompts to slash commands (Claude Code's
/// `/mcp__<server>__<name>`) only see prompts, not tools, so
/// shipping a skill as **both** is what gives users a slash entry
/// point without losing the model-driven tool path.
///
/// Argument schema is empty in v1 — skill bodies are static
/// markdown. When skills grow argv-style placeholders, this
/// adapter mirrors [`SkillTool`]'s frontmatter-driven schema work.
pub struct SkillPrompt {
    skill: Arc<Skill>,
    skills: SkillRegistry,
}

impl SkillPrompt {
    /// Wrap an approved [`Skill`] with the registry used to
    /// re-check it at call time.
    pub fn new(skill: Arc<Skill>, skills: SkillRegistry) -> Self {
        Self { skill, skills }
    }
}

#[async_trait]
impl Prompt for SkillPrompt {
    fn definition(&self) -> PromptDefinition {
        PromptDefinition {
            name: self.skill.id.to_string(),
            description: self.skill.description.clone(),
            arguments: Vec::new(),
        }
    }

    async fn render(&self, _arguments: Value) -> Result<PromptResponse> {
        // Same fail-closed re-check as `SkillTool::invoke`: a
        // revoke or quarantining reload removes the bundle from
        // the approved set, and this surface refuses to render.
        let still_approved = self
            .skills
            .list()
            .iter()
            .any(|s| s.id == self.skill.id && s.bundle_hash == self.skill.bundle_hash);
        if !still_approved {
            return Err(Error::Forbidden);
        }
        Ok(PromptResponse {
            description: Some(self.skill.description.clone()),
            messages: vec![PromptMessage {
                role: PromptRole::User,
                text: self.skill.body.as_ref().to_string(),
            }],
        })
    }
}

/// Fold every approved skill from `skills` into `registry` as an
/// MCP prompt. Quarantined bundles are not exposed. Pair with
/// [`register_approved_skills`] when the consumer wants both
/// `tools/*` and `prompts/*` surfaces — typical for desktop hosts
/// like Claude Code that only surface prompts as slash commands.
pub fn register_approved_skills_as_prompts(
    registry: ToolRegistry,
    skills: &SkillRegistry,
) -> ToolRegistry {
    skills.list().into_iter().fold(registry, |reg, skill| {
        reg.register_prompt_arc(Arc::new(SkillPrompt::new(skill, skills.clone())))
    })
}

// ---------- add_favorite meta-tool ----------

/// Arguments accepted by [`AddFavoriteTool`]. Validated by
/// `serde(deny_unknown_fields)`; transport layer has already
/// shape-checked against `input_schema`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AddFavoriteArgs {
    /// Reverse-DNS skill id (e.g. `starter.user.my_favorite`).
    id: String,
    /// Free-form description, surfaced to the operator approval UI.
    description: String,
    /// Verbatim markdown body the host LLM will eventually see.
    /// **No templating** — R-skills-1 / R4.
    body: String,
}

/// Built-in MCP tool that writes a new `SKILL.md` to a configured
/// user-skills directory and **does not approve it**.
///
/// The returned `status: "quarantined"` plus the `next_step` field
/// tell the operator exactly how to promote it. There is no
/// auto-approve path. Operators opt in to exposing this tool the
/// same way they opt in to any other; it is not enabled by default
/// in [`register_approved_skills`].
pub struct AddFavoriteTool {
    user_skills_dir: PathBuf,
}

impl AddFavoriteTool {
    /// Construct with the directory under which new bundles are
    /// written. Typically a `load_dir_quarantined(...)` source on
    /// the host's [`SkillRegistry`] so the bundle shows up as
    /// quarantined on the next reload.
    pub fn new(user_skills_dir: impl Into<PathBuf>) -> Self {
        Self {
            user_skills_dir: user_skills_dir.into(),
        }
    }
}

#[async_trait]
impl Tool for AddFavoriteTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "starter.add_favorite".into(),
            description:
                "Write a new SKILL.md to the user-skills directory. Always quarantined; \
                 an operator must approve before it becomes callable."
                    .into(),
            input_schema: json!({
                "type": "object",
                "additionalProperties": false,
                "required": ["id", "description", "body"],
                "properties": {
                    "id":          { "type": "string" },
                    "description": { "type": "string" },
                    "body":        { "type": "string" }
                }
            }),
        }
    }

    async fn invoke(&self, input: Value) -> Result<Value> {
        let args: AddFavoriteArgs =
            serde_json::from_value(input).map_err(|e| Error::Invalid {
                message: format!("add_favorite arguments: {e}"),
            })?;

        // Cheap shape check so we fail before touching disk. The
        // real validation runs when the SkillRegistry next reloads
        // and parses the bundle — operators see that error in the
        // quarantine UI.
        if args.id.trim().is_empty() {
            return Err(Error::Invalid {
                message: "id must not be empty".into(),
            });
        }

        let bundle_root = self.user_skills_dir.join(sanitize_id(&args.id));
        std::fs::create_dir_all(&bundle_root).map_err(|e| Error::Internal {
            source: Box::new(e),
        })?;
        let skill_md_path = bundle_root.join("SKILL.md");
        let contents = format_skill_md(&args.id, &args.description, &args.body);
        std::fs::write(&skill_md_path, contents.as_bytes()).map_err(|e| Error::Internal {
            source: Box::new(e),
        })?;

        let bundle_hash = starter_skills::approval::hash_bundle(&bundle_root)
            .map_err(|e| Error::Internal {
                source: Box::new(e),
            })?;

        Ok(json!({
            "skill_id":    args.id,
            "bundle_hash": bundle_hash,
            "status":      "quarantined",
            "path":        skill_md_path.display().to_string(),
            "next_step":   "Operator must approve this bundle hash before it is callable.",
        }))
    }
}

// ---------- helpers ----------

fn empty_object_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {}
    })
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Replace anything that is not `[A-Za-z0-9._-]` with `_` so the
/// id is a safe directory name. The on-disk dir name is opaque —
/// only the frontmatter `id` field matters for routing.
fn sanitize_id(id: &str) -> String {
    id.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn format_skill_md(id: &str, description: &str, body: &str) -> String {
    // YAML scalars: descriptions can contain colons or quotes, so
    // emit them as JSON strings (which are valid YAML flow scalars).
    let desc = serde_json::to_string(description).expect("string serializes");
    let id_yaml = serde_json::to_string(id).expect("string serializes");
    let trailing = if body.ends_with('\n') { "" } else { "\n" };
    format!(
        "---\nid: {id_yaml}\ndescription: {desc}\ntrust: quarantined\n---\n{body}{trailing}"
    )
}

