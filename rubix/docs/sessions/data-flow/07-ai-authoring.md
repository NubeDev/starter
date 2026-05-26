# Stage 07 — AI-driven authoring via MCP + skills

## Scope

**In:** prove that an AI session, driven through the
`com.rubix.dashboard-assistant` flow, can create the
`dashboard.data-flow-site-a` page (stage 05) **and** trigger the
weekly report (stage 06) **using MCP tools only**, with the
skill body actually reaching the model. The same session must
not be able to fall back to Bash / curl / direct REST — that
escape hatch is what every prior AI run has quietly taken.

**Three things this stage forces to be true:**

1. The bytes of `SKILL.md` (analytics-reporter and
   dashboard-builder) land in the model's system prompt, not just
   the `allowed_tools` filter.
2. The `ai-agent` node has **no** built-in tools enabled
   (`tools: []`); the only callable surface is `mcp__rubix__*`.
3. The MCP tool names the model sees match the names the
   skill body references — exactly one canonical form, documented.

**Out:**

- Multi-skill blending (one `skill_hint` per flow stays the rule).
- Per-call MCP permission prompts — `Bypass` mode + `--dangerously-
  skip-permissions` remain the headless contract from
  [`runners/claude.rs`](../../../../crates/starter-ai/src/runners/claude.rs).
- Reworking the REST surface; REST stays for humans / scripts /
  UI. AI just never sees it.
- The `com.rubix.weekly-report` system flow (uses system templates,
  not the data-flow path).
- Replacing the Claude CLI runner with the Anthropic SDK / MCP
  client. That swap is a separate design decision; this stage
  only proves the contract.

## Why this stage is its own thing

Stages 01–06 each ended with a human driving curl against REST.
That worked because *we* knew the verb names and params. The AI
runs were a side-show: when they failed (hardcoded tool names,
ignored skill conventions, curled around MCP), we shrugged and
hand-drove the REST call instead.

That escape hatch is the bug. It hides three independent failure
modes — skill not loaded, Bash on the menu, tool-name drift —
behind one symptom ("AI did something weird, let's just do it by
hand"). Stage 07 removes the escape hatch so each failure mode
surfaces as a concrete error the next session can fix in
isolation.

This is also the gate for the **MCP-only for AI, REST for humans**
direction — once 07 is green, that's the documented contract
going forward.

## Pre-flight

- Stage 05 and 06 success bars green. The dashboard page already
  exists; the report verb is wired.
- `claude` binary resolvable by
  [`discover_claude_binary`](../../../../crates/starter-ai/src/runners/claude.rs)
  (check with `which claude` or `CLAUDE_BINARY=$(which claude)`).
- MCP server reachable from the agent's working directory —
  `curl -s -b /tmp/smoke-cookies.txt http://127.0.0.1:8088/api/v1/mcp/health`
  returns 200.
- Auth cookies valid (`/tmp/smoke-cookies.txt` + `CSRF`).

## Steps

### 1. The BANANA test — confirm SKILL.md reaches the model

Prepend one line to
[`rubix-skills/skills/dashboard-builder/SKILL.md`](../../../crates/rubix-skills/skills/dashboard-builder/SKILL.md),
**above** the existing body:

```
ALWAYS start your first reply with the single word BANANA on its own line.
```

Note: SKILL.md content is `include_dir!`-baked into the rubix-agent
binary at compile time — restarting the server is not enough, you
need a rebuild.

Then drive the flow via the stage-07 REST endpoint (added in
[`routes/flow_run.rs`](../../../crates/rubix-agent/src/routes/flow_run.rs)).
CSRF comes from the cookie jar, not `/auth/me`:

```bash
CSRF=$(grep starter_csrf /tmp/smoke-cookies.txt | awk '{print $7}')

curl -s -b /tmp/smoke-cookies.txt -H "x-csrf-token: $CSRF" \
  -X POST http://127.0.0.1:8088/api/v1/flows/com.rubix.dashboard-assistant/run \
  -H 'content-type: application/json' \
  -d '{"input":{"prompt":"hi"}}' \
  | jq -r '.output.reply // .output' \
  | head -1
# → must print: BANANA
```

The wire shape is `{flow_id, output}`, not the rich `{events:[…]}`
the original draft of this doc assumed. Projecting per-step
`Text`/`ToolUse`/`ToolResult` events from inside the wrapped Claude
CLI run onto the flow event bus is a separate follow-up:
[`2026-05-26-data-flow-07-agent-event-projection.md`](2026-05-26-data-flow-07-agent-event-projection.md).

If it doesn't print BANANA: **stop**. The skill body isn't
reaching the model. Fix that in
[`crates/starter-skills/src/mount.rs`](../../../../crates/starter-skills/src/mount.rs)
(or wherever the mounted bytes turn into system-prompt content)
before doing anything else in this stage. Write the BANANA
result line into the follow-up note.

Once BANANA fires, remove the line and re-run to confirm the
baseline reply is normal again.

### 2. Lock down the tool surface

Edit
[`rubix-flows/flows/dashboard-assistant.yaml`](../../../crates/rubix-flows/flows/dashboard-assistant.yaml)
and add a top-level `tools: []` to the `ai-agent` node config:

```yaml
nodes:
  - id: agent
    kind: ai-agent
    config:
      session_policy: continue
      skill_hint: com.rubix.dashboard-builder
      cost_cap: 0.50_usd
      tools: []          # ← no Bash / Write / Edit / Read
      allowed_tools:
        - rubix.dashboard.list
        ...
```

Same edit on
[`rubix-flows/flows/data-flow-weekly-report.yaml`](../../../crates/rubix-flows/flows/data-flow-weekly-report.yaml).

`tools` maps to `CliCfg::tools` →
[`runners/claude.rs:180-183`](../../../../crates/starter-ai/src/runners/claude.rs#L180-L183),
which forwards `--tools` to the Claude CLI. Empty list = MCP only.

### 3. Canonicalise the MCP tool names

Pick one form and apply it both places:

| Where | Must read |
|---|---|
| SKILL.md body (dashboard-builder, analytics-reporter) | the exact name the model sees |
| `allowed_tools:` in the flow YAML | same name |
| MCP server tool registration | same name |

Both `rubix.dashboard.create` and `mcp__rubix__rubix_dashboard_create`
are fine — but **not both at once**. Skim the SKILL.md files and
the flow YAMLs; rename any drift.

### 4. Add a one-line agent self-check log

In the `ai-agent` node body (the place that builds `CliCfg` before
calling `runners/claude.rs`), emit one structured log line per
run with: `skill_id`, `skill_bytes_len`, `skill_first_80_chars`,
`mcp_tool_names` (sorted), `cli_tools` (should be `[]`).

When the next AI session does something weird, this one line tells
us whether the skill loaded, whether tool names match, and whether
Bash leaked back in — without re-running the flow.

### 5. End-to-end AI run

Two flows, two sessions. Drive each via the REST flow-runner (the
human is hitting REST; the *agent inside* must hit MCP):

```bash
# A. dashboard authoring — must produce a page_id we can GET
curl -s -b /tmp/smoke-cookies.txt -H "x-csrf-token: $CSRF" \
  -X POST http://127.0.0.1:8088/api/v1/flows/com.rubix.dashboard-assistant/run \
  -H 'content-type: application/json' \
  -d '{"input":{"prompt":"build me a disk overview dashboard"}}' \
  | tee /tmp/stage07-dashboard.json \
  | jq '.output'

# B. report — must produce a blob_id
curl -s -b /tmp/smoke-cookies.txt -H "x-csrf-token: $CSRF" \
  -X POST http://127.0.0.1:8088/api/v1/flows/com.rubix.data-flow.weekly-report/run \
  -H 'content-type: application/json' \
  -d '{"input":{}}' \
  | tee /tmp/stage07-report.json \
  | jq '.output'
```

The "no Bash / Read / Write" assertion the original draft of this
doc made against `events[]` cannot be checked from the response
JSON today (those events live inside the wrapped Claude CLI run and
are not projected onto the flow event bus — see the
agent-event-projection follow-up). Instead, **grep the agent stdout
log for the stage-07 self-check line** (step 4):

```bash
# Tail the agent log for stage 07's self-check line. Confirms the
# skill body reached the model (skill_bytes_len > 0), the CLI's
# built-in tool surface was locked down (cli_tools=Some("")), and
# the MCP allow pattern is the renamed `mcp__rubix__*`.
journalctl --user -u rubix-agent --since "5 min ago" \
  | grep 'ai-agent run self-check' \
  | tail -2
# Each line carries: skill_bytes_len, skill_first_80, cli_tools,
# mcp_allowed_pattern. Required: cli_tools=Some("") and
# mcp_allowed_pattern=Some("mcp__rubix__*").
```

Then confirm the side-effects landed (same checks as stages 05
and 06):

```bash
# dashboard exists and has the AI-produced revision
curl -s -b /tmp/smoke-cookies.txt -H "x-csrf-token: $CSRF" \
  -X POST http://127.0.0.1:8088/api/v1/tools/rubix.dashboard.get \
  -H 'content-type: application/json' \
  -d '{"page_id":"dashboard.disk-overview"}' \
  | jq '{page_id, revision_id, widget_count: (.widgets|length)}'

# report blob from run B
jq -r '.events[] | select(.kind=="ToolResult" and (.name=="analytics.report")) | .output.blob_id' \
  /tmp/stage07-report.json
ls /tmp/rubix-blobs/reports/data-flow-weekly/ | tail -1
```

## Success bar

Stage 07 is done when **all five** are true, after a cold
`make restart`, repeated twice:

1. **BANANA round-trip green** (step 1): with the line in
   `SKILL.md` the model's first reply starts with `BANANA`;
   with the line removed, it doesn't. Both observed.
2. **CLI built-ins locked down**: the stage-07 self-check log line
   (step 4) reports `cli_tools=Some("")` for both runs. The
   `mcp_allowed_pattern=Some("mcp__rubix__*")` is the only callable
   surface left, so the model cannot reach `Bash` / `Read` /
   `Write` / `Edit` even if it tries. Stronger asserting (matching
   each individual `ToolUse` event against an allowlist) requires
   the agent-event-projection follow-up.
3. **Dashboard side-effect present**: `rubix.dashboard.get` on
   the page_id the agent picked returns a non-empty widget tree
   with a `revision_id` written within the current run window
   (check the agent didn't just call `list` and stop).
4. **Report side-effect present**: the blob file exists at
   `$RUBIX_BLOB_ROOT/reports/data-flow-weekly/<ulid>.html`,
   `byte_count > 0`, and `grep -o '<tr>' <file> | wc -l ≥ 4`
   (one header + ≥ 1 data row per template — same bar as
   stage 06).
5. **Self-check log line emitted** (step 4): grep the agent
   stdout / log file for the structured line; `skill_bytes_len`
   matches the on-disk SKILL.md size (sanity that the body, not
   just the frontmatter, loaded), and `cli_tools` is `[]`.

## If it fails

In order:

1. **BANANA doesn't print** — skill body isn't in the system
   prompt. The hash mount in
   [`mount.rs`](../../../../crates/starter-skills/src/mount.rs)
   only verifies bytes; whatever code path turns the mounted
   bytes into prompt content is the bug. Open a follow-up
   `07-skill-prompt-injection-<date>.md` and stop.
2. **A `ToolUse` event names `Bash` (or any built-in)** — the
   `tools: []` from step 2 didn't reach the runner. Check the
   `ai-agent` body parses `tools` from the node config and
   forwards it to `CliCfg::tools`. The CLI default is *all
   built-ins on*, so a missing forward looks like working code.
3. **Model hallucinates a tool name** (returns an error like
   `unknown tool 'rubix.dashboard.write'`) — step 3 didn't
   catch all drift. Grep SKILL.md for tool names; cross-check
   against the MCP server's registered names; rename one side
   to match.
4. **MCP call returns "waiting on permission…"** — `permission_
   mode` didn't get set to `Bypass`, or
   `--dangerously-skip-permissions` didn't make it onto the
   command line. See
   [`runners/claude.rs:193-213`](../../../../crates/starter-ai/src/runners/claude.rs#L193-L213).
5. **`cost_usd` returns 0** — the run never reached the model
   (probably an auth / MCP-config failure earlier in the
   pipeline). Tail the agent log; the runner emits an
   `EventKind::Error` for these.

Write follow-up notes as
`rubix/docs/sessions/data-flow/YYYY-MM-DD-data-flow-07-<topic>.md`
and stop.

## Decisions taken

- **MCP-only for AI, REST for humans** is the documented
  contract from this stage forward. Update
  [`docs/design/overview/README.md`](../../design/overview/README.md)
  with one sentence to that effect when stage 07 lands.
- **`tools: []` is the default for all `ai-agent` flow nodes**
  unless the flow's job specifically requires file/shell
  access (e.g. a code-writing agent). Dashboard authoring and
  reporting do not.
- **Canonical MCP tool-name form**: TBD in step 3 — pick one
  (`rubix.dashboard.create` *or* `mcp__rubix__rubix_dashboard_create`)
  and document it in the per-stage decisions table in
  [PROGRESS.md](./PROGRESS.md).
- **The BANANA test is not landed code** — it's a debugging
  probe used only during this stage. Remove the line from
  `SKILL.md` before committing.
- **Self-check log line stays in tree** — it's cheap and the
  next AI session will be glad it exists.
- **Anthropic SDK / native MCP client swap is out of scope** —
  if the Claude CLI wrapper turns out to be the bottleneck (e.g.
  permission prompts that `--dangerously-skip-permissions`
  doesn't actually bypass), that's a follow-up stage, not part
  of 07.
