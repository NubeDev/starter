# SKILLS — how to author a rubix skill + the bundled six

> **Authoritative `SKILL.md` format:** starter's
> [DOCS/agent/SKILLS.md](../../../DOCS/agent/SKILLS.md) and the
> "skill scope rules" in starter's
> [DOCS/agent/SCOPE.md](../../../DOCS/agent/SCOPE.md). This doc is the
> rubix-specific overlay.
>
> Cites: SCOPE [R7](../../SCOPE.md#r7).

## The bundled six

Each lives in [rubix-skills/skills/<goal>/SKILL.md](../../crates/rubix-skills/skills/):

| Goal | id | trust |
|---|---|---|
| 1 | `com.rubix.dashboard-builder` | approved |
| 2 | `com.rubix.user-admin` | approved |
| 3 | `com.rubix.flow-programmer` | approved |
| 4 | `com.rubix.clickhouse-ruler` | approved |
| 5 | `com.rubix.system-checker` | approved |
| 6 | `com.rubix.analytics-reporter` | approved |

Rubix-bundled skills are **approved by default** (they live in the
host's own crate). Extension-shipped skills default to
**quarantined** — operator approves them by content hash.

## Skill-deny behaviour (R7)

When the agent attempts a tool not in the active skill's
`allowed_tools` intersection:

1. Dispatch fails with `Error::SkillForbidden { skill, tool }`.
2. An `agent.tool.error` event fires carrying both ids.
3. The agent receives a localized `MessageKey` response
   (`rubix.skill.denied`) so the next turn can self-correct.
4. **No auto skill-swap.** Changing skills mid-run would break
   starter's skill scope rule 4 (one selection per outer flow run).

## Hot reload

Operator-dropped skills in `$XDG_DATA_HOME/rubix/skills/` may
hot-reload — starter-flow-watch (with possible enhancement; see
[STARTER-CHANGES.md](./STARTER-CHANGES.md)) covers this. Rubix-
bundled skills are static (compiled in via `include_dir!`); a
rubix-agent rebuild + restart is required to change them.

## Skill observability

Every `agent.turn.start` event carries the active skill id (R13).
Querying "why did the agent do that?" means grepping traces by
turn id, finding the skill, and reading the skill body. This is
the difference between debuggable and opaque.
