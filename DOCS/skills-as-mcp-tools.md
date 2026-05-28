# Skills as MCP tools

**Status:** design proposal. Not yet implemented.
**Owner:** ap@nube-io.com
**Date:** 2026-05-28

## Why this exists

I keep retyping the same prompts at Claude Code, Copilot, and Codex —
`/ship-it-check`, `/pr-review`, `/safe-refactor`, `/debug-rust`,
`/add-ai-favorite`. The fix is one source of truth for these
workflows on disk, surfaced to every AI client through MCP, with the
same approval gate the rest of `starter-skills` already uses.

I do **not** want a second skill format, a second registry, or a
parallel YAML-MCP layer (Pforge, rust-mcp-core, etc.). The starter
already has the pieces; what is missing is one adapter.

## What already exists

| Crate | What it does today |
|---|---|
| [starter-skills](../crates/starter-skills/) | Loads `SKILL.md` bundles (YAML frontmatter + Markdown body), content-hashes each bundle, gates them through an `ApprovalStore`. Quarantined-by-default for any contributed skill. |
| [starter-mcp](../crates/starter-mcp/) | Stdio + HTTP MCP server. Holds a `ToolRegistry` of `Arc<dyn Tool>` and dispatches `tools/list` and `tools/call`. |
| [starter-spi::tool](../crates/starter-spi/src/tool/) | The `Tool` trait (`definition() -> ToolDefinition`, `invoke(json) -> Result<json>`) every MCP-callable type implements. |

The seam between them — the thing that does not exist yet — is a type
that wraps an approved `Skill` and implements `Tool`. That type, plus
a small builder, is the whole feature.

## Non-goals

- No new crate. The adapter lives inside `starter-mcp` behind a
  `skills` cargo feature.
- No new on-disk format. `SKILL.md` is the format. The same files
  `starter-flow` and `starter-ai-agent` already consume.
- No new approval store. `starter_skills::ApprovalStore` is reused
  verbatim.
- No templating, no env expansion, no string interpolation inside the
  body. Same R-skills-1 / R4 guarantee the parent crate already
  enforces.
- No per-call destructive-action prompting from the MCP server. That
  policy lives in the host (Claude Code's permission system). The
  server can mark a tool high-risk in its description; it cannot force
  the host to prompt.

## How it fits together

```
       SKILL.md files on disk
       (~/.claude/skills, repo skills/, project .skills/)
                  │
                  ▼
       starter_skills::SkillRegistry
       - parse + hash bundles
       - ApprovalStore gates which bundles are "approved"
                  │
                  │  SkillTool adapter (this proposal)
                  ▼
       starter_mcp::ToolRegistry
       - one Arc<dyn Tool> per approved skill
                  │
                  ▼
       MCP transport (stdio / HTTP)
                  │
                  ▼
       Claude Code  /  Copilot  /  Codex
       (each shows the tools in its own slash-command-ish UI)
```

## The adapter

One type, in `crates/starter-mcp/src/skills_bridge.rs` (gated by
`feature = "skills"`):

```rust
use std::sync::Arc;
use async_trait::async_trait;
use starter_skills::{Skill, SkillRegistry};
use starter_spi::tool::{Tool, ToolDefinition};
use starter_spi::error::{Error, Result};

pub struct SkillTool {
    skill: Arc<Skill>,
    skills: SkillRegistry,
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

    async fn invoke(&self, _input: serde_json::Value) -> Result<serde_json::Value> {
        // Re-check membership in the approved set at call time, so
        // a revoke (or a re-quarantining reload) takes effect
        // immediately without restarting the server.
        //
        // We go through `SkillRegistry::list()` (not directly to
        // `ApprovalStore::lookup`) because the registry encapsulates
        // the trust matrix: a `load_dir(...)` bundle with
        // `trust: approved` in its frontmatter has no row in the
        // store, so a raw `lookup` would always return None and
        // every call would fail. The registry's approved set is the
        // single source of truth.
        let still_approved = self
            .skills
            .list()
            .iter()
            .any(|s| s.id == self.skill.id && s.bundle_hash == self.skill.bundle_hash);
        if !still_approved {
            return Err(Error::Forbidden);
        }
        Ok(serde_json::json!({ "body": self.skill.body.as_ref() }))
    }
}
```

The MCP `tools/call` result is the verbatim SKILL.md body. The host
LLM then reads those instructions and executes them — same model
`starter-flow` already relies on.

### Builder

```rust
pub fn register_approved_skills(
    registry: ToolRegistry,
    skills: &SkillRegistry,
) -> ToolRegistry {
    skills.list().into_iter().fold(registry, |reg, skill| {
        reg.register_arc(Arc::new(SkillTool {
            skill,
            skills: skills.clone(),
        }))
    })
}
```

`SkillRegistry::list()` already returns only approved skills;
`list_quarantined()` is the unapproved set and is **not** registered.

## Naming

`Skill.id` is a validated reverse-DNS id (e.g. `starter.ship_it_check`).
`ToolDefinition.name` is snake-case, dot-namespaced — same shape. We
pass it through as-is. No mangling.

Whether the host renders that as `/ship-it-check` is up to the host.
Claude Code surfaces MCP tools as `/mcp__<server>__<tool>`; Copilot
and Codex differ. Workflow sharing is the actual win; literal
slash-command parity is not the goal.

## The `add-favorite` meta-tool

One built-in `Tool` impl that writes a new `SKILL.md` to a configured
"user skills" directory and **does not approve it**. It returns the
new bundle's hash and a message telling the operator how to approve.

```
add_favorite(name, description, body, category, risk) -> {
    skill_id, bundle_hash, status: "quarantined",
    next_step: "run `starter approvals approve <id> <hash>`"
}
```

This is the only path by which the LLM can mint a new favorite, and
the result is inert until a human approves. Exactly matches the
existing R-skills-3 guarantee that contributed skills are always
quarantined.

## Audit

Every `invoke()` emits one record after the approval check passes,
through a small `SkillAuditSink` trait that ships in
`starter-mcp::skills_bridge`:

```rust
pub trait SkillAuditSink: Send + Sync + 'static {
    fn record(&self, invocation: SkillInvocation<'_>);
}
```

The default `TracingSkillAuditSink` writes one structured
`tracing::info!` per call (`target = "starter_mcp::skills_bridge::audit"`,
fields = `skill_id`, `bundle_hash`, `at_unix_ms`). Consumers that
want a durable, changelog-backed audit row implement the trait
themselves and pass the sink to `register_approved_skills_with_audit`.

`starter-audit` is a read-only projection over the changelog and
does not expose a "record a row" API, so the audit sink lives in
`starter-mcp` rather than `starter-audit`. A changelog-backed
`SkillAuditSink` impl (that calls into `starter-changelog::ChangeLog`)
is a small follow-up in the consumer crate, not a starter-mcp
concern.

## What changes in each crate

- **`starter-spi`** — nothing.
- **`starter-skills`** — nothing.
- **`starter-mcp`** —
  - Add optional dep on `starter-skills` behind `feature = "skills"`.
  - New file `src/skills_bridge.rs` with `SkillTool` and
    `register_approved_skills`.
  - One `tests/skills_bridge.rs` integration test:
    load two bundles, approve one, build the `ToolRegistry`, assert
    that `tools/list` shows the approved one and `tools/call` on the
    quarantined one returns `unapproved_error`.
- **`starter-audit`** — nothing, just consumed.
- **Examples** — extend [examples/minimal](../examples/) (or whichever
  example wires `starter-mcp` today) with a `skills/` dir containing
  one example `SKILL.md` so the docs have a runnable end-to-end.

## What this does **not** solve

Listed up front so I don't sell myself a fantasy:

1. **Cross-client UX parity.** Claude Code, Copilot, and Codex each
   render MCP tools their own way. Same workflow, different surface.
2. **Per-run destructive-action prompts.** Belongs in the host. The
   server can only describe risk; it cannot block the host from
   acting.
3. **Search / fuzzy match across favorites.** Out of scope for v1.
   `starter-skills` already ships keyword and LLM selectors
   ([selector.rs](../crates/starter-skills/src/selector.rs)); a future
   `/find-favorite` tool can call them. Not in this proposal.
4. **Hot reload of new SKILL.md files.** `SkillRegistry::reload()`
   exists; wiring it to a filesystem watcher and rebuilding the
   `ToolRegistry` on the fly is a follow-up, not v1.

## Open questions

- Should `SkillTool::invoke` accept structured arguments (a
  `JsonSchema` derived from frontmatter), or always be argumentless
  and return the body verbatim? v1 says **argumentless** — keeps the
  surface tiny and matches how the host LLM already consumes skill
  bodies elsewhere. Structured args become a v2 frontmatter field.
- Where do user-owned (not repo-owned) favorites live? Proposal:
  `~/.config/starter/skills/` as a default `load_dir_quarantined()`,
  with the path overridable via `starter-config`.
- Should `add_favorite` be enabled by default? Proposal: **no**.
  Operators opt in by listing it in the server's tool set, same as
  any other tool.

## Next step

If this design holds up under one more read, the implementation order is:

1. `SkillTool` + `register_approved_skills` + the integration test.
2. The `add_favorite` meta-tool.
3. The audit wiring.
4. Doc update in [examples/](../examples/) showing the full loop.

Each step is small and lands independently.
