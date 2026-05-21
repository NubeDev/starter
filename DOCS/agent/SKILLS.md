# `starter-skills` — Scope

> ⚠ **Read these first.** This doc carves out one crate inside the
> agent SCOPE. If anything below contradicts them, they win.
>
> - [DOCS/flow/scope/SCOPE.md](../flow/scope/SCOPE.md) — the runtime
>   substrate. Skills bind to a flow *run*, not to a node; selection
>   is one of the engine's "once per run" inputs alongside the
>   trigger and the principal.
> - [DOCS/agent/SCOPE.md](./SCOPE.md) — owns the `SKILL.md` format
>   (Part B), the four skill scope rules, the `ai-agent` node kind
>   that consumes selections, and the content-hash quarantine rule
>   (R4). This doc describes the crate that *implements* that
>   contract; it does not redefine it.
> - [DOCS/frontend/ai-builder/SCOPE.md](../frontend/ai-builder/SCOPE.md)
>   — the first downstream consumer (two reference skills:
>   `starter.ai-builder.dashboards`, `starter.ai-builder.themes`).
>   Phase 5 of that work is blocked on this crate landing.

## One-line summary

`starter-skills` turns a directory of `SKILL.md` bundles into a
`SkillRegistry` that the flow engine drives via the existing
`SkillSelector` seam in [starter-flow-spi](../../crates/starter-flow-spi/src/skill.rs).
It owns the parser, the content-hash + quarantine state machine, the
approval store, and the default LLM-by-description selector. Zero
new seams; zero new wire formats.

## Why this exists

The seam is already wired and tested end-to-end:

- [crates/starter-flow-spi/src/skill.rs](../../crates/starter-flow-spi/src/skill.rs)
  defines `SkillId`, `SkillSelection`, `ResourceRef` (with
  `content_hash`), `SkillSelector` trait, `NullSkillSelector` default.
- The engine threads one `SkillSelection` per outer run into every
  `ai-agent` node via `NodeCtx` (proven by
  [crates/starter-flow/tests/stage5_skill_threading.rs](../../crates/starter-flow/tests/stage5_skill_threading.rs)).
- The quarantine invariant — a mid-flight bundle update does not
  perturb the in-flight run's selection — is also covered by the
  stage-5 smoke.

What does **not** exist yet:

- A `SKILL.md` parser.
- A `SkillRegistry` that can `load_dir(...)`.
- The content-hash algorithm specified by [agent R4](./SCOPE.md#r4--skills-are-static-metadata-quarantined-by-default).
- An `ApprovalStore` that persists `(skill_id, hash, approved_at, approved_by)`.
- The default LLM-by-description selector.

`starter-skills` fills exactly that gap. The skill.rs source comment
already names this crate by intention:

> the real content-hash-backed selector lives in a future
> `starter-skills` crate that is **not** a workspace member yet.

## Relationship to existing crates

```
starter-flow-spi          (SkillId, SkillSelection, ResourceRef,
                           SkillSelector trait — exists, unchanged)
   ▲
   │
   ├── starter-skills     (NEW — parser, registry, content-hash,
   │                       quarantine, default selector, approval
   │                       store trait. Implements SkillSelector.)
   │
   ├── starter-flow       (engine — exists, unchanged. Calls
   │                       SkillSelector::select once per run.)
   ├── starter-flow-nodes (ai-agent node — exists, unchanged.
   │                       Reads SkillSelection from NodeCtx.)
   │
   └── starter-store-sqlite / starter-store-postgres
                          (existing — gain one new table for
                           ApprovalStore, behind the same store
                           trait pattern as RunStore / SessionStore.)
```

No new SPI crate. `SkillSelector` already lives in
`starter-flow-spi`; `starter-skills` only ships the implementation.
There is no `starter-ext-skills` crate either — extensions will
contribute skills through the `starter-ext-flow` adapter's
`contributes.skills` field as specified in
[agent R-agent-4](./SCOPE.md#r-agent-4--extensions-contribute-agents-as-flows).
The `contributes.skills` field is *specified* in the agent SCOPE but
not yet wired in `starter-ext-flow`; wiring it is Phase 6 below. No
new adapter, no new wire format — one new branch in the existing
adapter that calls `SkillRegistry::extend(...)`.

## Hard rules

These are specific to `starter-skills`. Engine rules
([flow SCOPE](../flow/scope/SCOPE.md)) and agent rules
([agent SCOPE](./SCOPE.md)) apply transitively. Agent R4 in particular
*is* this crate's contract; the rules below are how that contract is
upheld.

### R-skills-1 — `SKILL.md` is parsed once, at load time

Every bundle is parsed when `SkillRegistry::load_dir(...)` runs or
when an extension contributes its `skills/` dir. Frontmatter is YAML,
deserialised with `serde` `deny_unknown_fields`. Body and
`resources/*` files are read into memory once and held by `Arc`.

**No templating, no interpolation, no environment expansion at any
point.** A `{{user_name}}` in a skill body is literal text the model
sees verbatim. This is the agent R4 anti-prompt-injection guarantee;
breaking it is a CVE-class bug.

A parse error fails `load_dir` with a structured error naming the
file and line; the registry refuses to come up rather than silently
skipping a malformed bundle.

### R-skills-2 — Content hash is computed by the exact procedure in agent R4

The hash algorithm is normative, not advisory. The implementation in
`starter-skills::approval::hash_bundle(path)` must produce a digest
identical to the steps in [agent R4](./SCOPE.md#r4--skills-are-static-metadata-quarantined-by-default):

1. Enumerate every file under `path` recursively.
2. Exclude paths matching the names in
   `starter_skills::approval::EXCLUDED` (`.DS_Store`, `Thumbs.db`,
   `*.swp`, `*.swo`, `*~`, `.git/`, `.idea/`, `__pycache__/`).
3. Normalise line endings on text files. **Text file** = filename
   extension in `{.md, .txt, .json, .yaml, .yml, .toml}`. The
   transform is exactly two replacements, applied in order, to the
   raw bytes:
   1. `\r\n` (0x0D 0x0A) → `\n` (0x0A)
   2. lone `\r` (0x0D not followed by 0x0A, after step 1) → `\n`
   No other transforms. **No BOM stripping.** **No UTF-16 / UTF-32
   handling** — a `.txt` file that isn't UTF-8 hashes as its raw
   bytes after the two CR transforms above (which are byte-level,
   not codepoint-level). Binary files (any extension not in the
   text set) hash as-is.

   *Rationale for "no BOM, no UTF-16":* a bundle author who commits
   a UTF-16 `.md` will get a different hash on every editor that
   re-saves it, which is the correct outcome — fix the file.
4. Sort entries by relative path bytes, lexicographic. Path
   separators are normalised to `/` before sort and before hashing.
5. For each entry, feed framed bytes into a single `blake3` hasher
   in sort order:

   ```
   u64_le(path_byte_len) || path_bytes ||
   u64_le(content_byte_len) || content_bytes
   ```

   Length-prefixed framing — no NUL separators. (NUL is a legal
   byte in file contents and a legal byte in paths on POSIX; a
   delimiter-based scheme is collision-prone.)
6. Hex-encode (lowercase) the 32-byte digest.

A property test pins this algorithm: a fixture skill dir produces a
specific hex digest; any single-byte change to a non-excluded file
changes the digest; a file at path `a/b` with content `c` does not
collide with a file at path `a` with content `b/c` (the framing
test); a file committed with CRLF and the same file with LF produce
the same digest (the line-ending test). Adding to `EXCLUDED` goes
via PR (agent R4).

> **Note for agent SCOPE R4 sync.** R4's step 3 currently says
> "normalise line endings" without spelling out the CR-only case or
> BOM handling, and R4's step 5 specifies `\0`-separated framing.
> Steps 3 and 5 above are the *normative* refinement; an
> agent-SCOPE-level edit to R4 to match (or to switch to byte-exact
> "commit LF" — see [S-D5](#s-d5--should-r4-drop-line-ending-normalisation-entirely))
> follows from this doc landing.

### R-skills-3 — Trust matrix: load path is authoritative, frontmatter is advisory

Trust is determined at load time by the table below. The
frontmatter `trust:` field is documentary (it tells a human reviewer
what the bundle author intended) but it can only *lower* trust,
never raise it.

| Load path                             | Frontmatter `trust:` | Result                                  |
|---------------------------------------|----------------------|-----------------------------------------|
| `load_dir(...)` (host dir)            | `approved` or absent | **approved**                            |
| `load_dir(...)` (host dir)            | `quarantined`        | **quarantined** (author opt-in to hold) |
| `extend(...)` (extension contribution)| any value, or absent | **quarantined**                         |

The frontmatter field exists for one reason: a host-dir bundle
author who is uncertain about a draft can ship it with
`trust: quarantined` so the operator must explicitly approve it.
Removing the field is tempting but loses that signal. The field
stays; the matrix above is the contract.

The `ApprovalStore` overrides the matrix only in one direction:
a row keyed on `(skill_id, hash)` flips a quarantined bundle to
approved at load time. A hash mismatch — for any reason — means the
matrix above applies again and the bundle is quarantined.

An extension cannot self-approve. There is no frontmatter value
that produces `approved` from an `extend(...)` load path.

### R-skills-4 — `select(...)` never returns a quarantined bundle

`SkillRegistry::select(query) -> Option<Skill>` filters quarantined
bundles out before passing the candidate set to the underlying
`SkillSelector` strategy. Quarantined bundles appear in
`list_quarantined()` for operator review and nowhere else. There is
no override flag, no `--allow-quarantined`, no env var.

A flow author who needs a quarantined skill to run goes through the
approval flow. This is non-negotiable: the quarantine guarantee is
the reason extensions can contribute skills at all.

### R-skills-5 — Default selector is "LLM picks by description"

The shipped `SkillSelector` impl makes one cheap call (Haiku by
default; configurable via `AiRunner` injection) against a
deterministically-ordered list of `(skill_id, description)` pairs and
returns the chosen `SkillId`. If the model returns an unknown id or
fails to choose, the result is `SkillSelection::None`.

**Failure semantics, normative:**

- **Timeout:** 2 seconds, hard. Selection budget is bounded; a slow
  provider does not slow down every flow run by an unbounded amount.
  Configurable on the selector builder; 2s is the default.
- **Retries:** none. Selection is on the hot path; retries multiply
  tail latency for a feature that gracefully degrades.
- **On timeout / 5xx / network error / parse error / unknown id:**
  return `SkillSelection::None` and record one
  `skill_selector_failed_total{reason="..."}` counter increment plus
  a `WARN`-level structured log. The flow run continues with no
  skill applied. This is intentional graceful degradation: a
  transient provider hiccup should never block a flow run.
- **Auditability:** every selector outcome (success or failure) emits
  one tracing span tagged with `skill.selector.outcome` and, on
  failure, `skill.selector.reason`. Operators can alert on the
  failure-rate metric if the silent-degradation default is too
  permissive for their environment.

Alternatives ship as separate selectors a consumer can plug in via
`Engine::with_skill_selector(...)`:

- `KeywordSkillSelector` — deterministic, matches against
  description keywords. No LLM call.
- `FirstSkillSelector` — picks the alphabetically-first matching
  skill. Useful for tests.
- `NullSkillSelector` — already in `starter-flow-spi`; returns
  `None`. Engine default until a real selector is registered.

A vector-similarity selector is **not** in v1. Embedding storage,
re-embedding on bundle update, and provider routing for embeddings
are a separate scope; revisit once a consumer demands it.

### R-skills-6 — Selection inputs are the run's input slot map + principal

`SkillSelector::select` already takes `(&SlotMap, &Principal)`. The
default selector treats one named slot (`prompt`, by convention) as
the query string. Other slots are ignored. **The principal is not
passed to the LLM.** It is available so a custom selector can scope
the candidate set ("admin can pick any skill; viewer is limited to
the read-only set") but the default selector ignores it.

This keeps the audit story honest: selection cost is bounded by
input size, and principal-scoping is opt-in per selector.

### R-skills-7 — `ApprovalStore` is a trait, not a crate

The store trait lives in `starter-skills` and matches the shape of
`RunStore` / `SessionStore`:

```rust
#[async_trait]
pub trait ApprovalStore: Send + Sync + 'static {
    async fn record(&self, row: ApprovalRow) -> Result<(), StoreError>;
    async fn lookup(&self, skill_id: &SkillId, hash: &str)
        -> Result<Option<ApprovalRow>, StoreError>;
    async fn list(&self) -> Result<Vec<ApprovalRow>, StoreError>;
    async fn revoke(&self, skill_id: &SkillId, hash: &str)
        -> Result<(), StoreError>;
}
```

`starter-store-sqlite` and `starter-store-postgres` each gain one
table (`skill_approvals`) and implement the trait. No new store
crate.

In-memory impl (`InMemoryApprovalStore`) ships in `starter-skills`
for tests and ephemeral hosts.

**`revoke` is operator-driven only.** A hash mismatch on `reload()`
does *not* mutate the store — the prior `(skill_id, H1)` row stays,
inert, and the new bundle at hash `H2` is quarantined because no
`(skill_id, H2)` row exists. The store is append-mostly: rows are
added by `approve`, removed by `revoke`, and never written from any
read path. This is what lets R-skills-8 (no I/O on `select`) hold
even after a hot bundle edit.

### R-skills-8 — No I/O during `select()` beyond the optional LLM call

`SkillRegistry::select` must not read from disk or the database. All
disk reads happen at `load_dir` time; all approval rows are cached
in memory at registry construction and refreshed via an explicit
`reload()` call (or a file-watcher in the host, out of this crate's
scope).

Reason: selection runs on the hot path at the start of every flow
run. A `db.read()` per run multiplies the engine's tail latency by
the approval store's tail latency, and there's no reason to pay that.

## Public API surface (v1)

```rust
// Load
SkillRegistry::builder()
    .with_approval_store(impl ApprovalStore)
    .with_default_selector(impl SkillSelector)        // optional
    .load_dir(path: &Path)                            // R-skills-3 row 1/2
    .load_dir_quarantined(path: &Path)                // staging dirs: always quarantined
    .extend(contributed: Vec<ContributedSkill>)       // R-skills-3 row 3
    .build() -> Result<SkillRegistry, LoadError>

// Query
registry.list() -> Vec<SkillSummary>
registry.list_quarantined() -> Vec<SkillSummary>
registry.get(id: &SkillId) -> Option<&Skill>

// Approve / revoke (operator path)
registry.approve(id: &SkillId, hash: &str, by: &Principal) -> Result<(), _>
registry.revoke(id: &SkillId, hash: &str, by: &Principal) -> Result<(), _>

// Select — implements SkillSelector from starter-flow-spi
impl SkillSelector for SkillRegistry { ... }
```

`Skill` carries: `id`, `description`, `allowed_tools: Vec<KindId>`,
`model_hint: Option<String>`, `trust: Trust`, `body: Arc<str>`,
`resources: Vec<ResourceRef>`, `bundle_hash: String`.

**Resource URI scheme is `file://` only in v1.** `ResourceRef.uri`
is a `file://` URL resolving to a path relative to the bundle dir.
Other schemes (`s3://`, `ext://`, `http://`) parse-fail at load
time. Broadening the scheme set is a future-version concern; v1 is
file-only so the API surface is fully defined.

`Skill` does **not** carry the resource file *contents* — only
`ResourceRef { uri, content_hash }`. The `ai-agent` node body
resolves URIs to bytes at mount time and **must verify the resolved
bytes' blake3 hash matches `ResourceRef.content_hash`**. On
mismatch (e.g. the bundle was edited on disk between flow-run start
and resource mount), the node fails the run with a
`SkillResourceHashMismatch` error rather than mounting drifted
bytes.

This is the load-bearing piece of the quarantine guarantee under
concurrent `reload()`:

1. `reload()` rebuilds the in-memory registry but **does not touch
   in-flight `SkillSelection` values** — those were cloned into the
   engine's run state at run start.
2. An in-flight run's `ai-agent` node mounts resources by reading
   the URI (which points to disk) and verifying the bytes against
   the frozen `content_hash`.
3. If disk has drifted, the hash check fails; the node aborts the
   run with a structured error.
4. The next run starts fresh against the new registry state and
   gets a fresh `SkillSelection` whose `content_hash`es match disk.

The alternative — eager-loading all resource bytes into memory at
`load_dir` — was rejected because bundles can contain large
binary resources (datasets, images) and most runs use one skill at
a time. The hash-on-mount check keeps the registry small *and*
preserves the quarantine invariant.

A smoke test ("Resource hash mismatch aborts the run") pins this
behaviour.

## How it plugs into the engine

```rust
// in a host binary's main.rs
let approvals = starter_store_sqlite::open_approval_store("data.db")?;

let skills = starter_skills::SkillRegistry::builder()
    .with_approval_store(approvals)
    .load_dir(host_skills_dir())              // approved
    .extend(ext_host.contributed_skills())    // quarantined
    .build()?;

let engine = starter_flow::Engine::builder()
    .with_runner(runner)
    .with_skill_selector(Arc::new(skills))    // implements SkillSelector
    .with_node_kinds(starter_flow_nodes::builtins())
    .with_flows(flows)
    .with_store(starter_store_sqlite::open("data.db")?)
    .build()?;
```

From the engine's perspective, nothing changes — it just calls
`SkillSelector::select` once per outer flow run. The selection
threads through to every `ai-agent` node via `NodeCtx` as it does
today.

## Skill bundle layout on disk

```
skills/
├── com.acme.refund-flow/
│   ├── SKILL.md
│   ├── refund-policy.md
│   └── examples.md
├── starter.ai-builder.dashboards/
│   ├── SKILL.md
│   ├── prompt.md
│   └── schema.json
└── starter.ai-builder.themes/
    ├── SKILL.md
    ├── prompt.md
    └── tokens.json
```

The directory name **must equal** the `id:` in the frontmatter.
Mismatch fails `load_dir` with a structured error.

Bundles without resources (single-file `SKILL.md` in a directory)
are valid; the `resources:` frontmatter field is optional and
defaults to empty.

## Reference skills shipped in-tree

Two skill bundles ship under [`skills/`](../../skills/) in the
workspace root. They are the canonical worked examples for both the
`SKILL.md` format and the `starter-skills` registry, and they are
the first non-test consumers of `SkillRegistry::load_dir(...)`.
Both target the [ai-builder](../frontend/ai-builder/SCOPE.md)
authoring mode over [SDUI](../frontend/sdui/SCOPE.md); the
[`examples/flow-agent`](../../examples/flow-agent/SCOPE.md) demo is
the host that wires them into a running flow.

### `starter.ai-builder.dashboards`

- Path: [`skills/starter.ai-builder.dashboards/`](../../skills/starter.ai-builder.dashboards/)
- Files: [`SKILL.md`](../../skills/starter.ai-builder.dashboards/SKILL.md),
  [`prompt.md`](../../skills/starter.ai-builder.dashboards/prompt.md),
  [`schema.json`](../../skills/starter.ai-builder.dashboards/schema.json)
- `allowed_tools`: `starter.mcp.call`, `starter.flow.transform`
- `trust`: `approved` (host-dir load path; see R-skills-3)
- Purpose: drafts, edits, and publishes ai-builder dashboards
  (pages, panels, layout grids, widget bindings) by driving the
  editor transport through the MCP tool surface. Selected when the
  user's request mentions panels, charts, layout, or publishing a
  page.
- `schema.json` is the source of truth for which panel kinds and
  binding shapes the agent may emit; `prompt.md` is the verbatim
  system prompt the `ai-agent` node feeds the runner.

### `starter.ai-builder.themes`

- Path: [`skills/starter.ai-builder.themes/`](../../skills/starter.ai-builder.themes/)
- Files: [`SKILL.md`](../../skills/starter.ai-builder.themes/SKILL.md),
  [`prompt.md`](../../skills/starter.ai-builder.themes/prompt.md),
  [`tokens.json`](../../skills/starter.ai-builder.themes/tokens.json)
- `allowed_tools`: `starter.mcp.call`, `starter.flow.transform`
- `trust`: `approved` (host-dir load path; see R-skills-3)
- Purpose: edits ai-builder theme tokens (colour, typography,
  spacing, radius, shadow) and component styles through the same
  MCP editor transport. Selected when the user's request mentions
  palette, dark mode, typography, or restyling.
- `tokens.json` is the canonical token-name list — the skill body
  forbids inventing tokens outside that set; `prompt.md` is the
  verbatim system prompt.

Both bundles are deliberately scoped narrowly so the default
`LlmSelector` (R-skills-5) can disambiguate them by description
alone: dashboards = structure, themes = styling. Adding a third
ai-builder skill that overlaps either domain is a signal the
selection prompt needs to be revisited, not that a new skill is
free.

> The dashboard builder itself (the surface that consumes
> `starter.ai-builder.dashboards`) is the next deliverable on top
> of this crate; see [DOCS/frontend/ai-builder/SCOPE.md](../frontend/ai-builder/SCOPE.md)
> Phase 5 and the [`examples/flow-agent`](../../examples/flow-agent/SCOPE.md)
> host for the integration target.

## What does NOT land in v1

- **No vector / embedding selector.** R-skills-5. Embedding storage
  and re-embedding on bundle update is a separate scope.
- **No live file-watcher.** The registry exposes `reload()`; running
  it on a `notify` event is the host's job, not this crate's. (Same
  policy as `RunStore`.)
- **No skill versioning beyond content hash.** A skill with the same
  `id:` and a different body is a different bundle; the operator
  re-approves. No semver field, no compatibility check. The hash
  *is* the version.
- **No cross-skill composition.** A skill cannot `include:` another
  skill's body. If two skills share boilerplate, the operator ships
  the boilerplate twice. Composition is a flow concern (chain two
  `ai-agent` nodes with different `skill_hint`s).
- **No skill-side tool definitions.** `allowed_tools` is a *filter*
  over the host's `ToolRegistry` (agent R3 / flow R8). A skill
  cannot define a new tool; it can only narrow what's already
  available.
- **No quarantine bypass mechanism.** R-skills-4. There is no
  `--allow-quarantined` flag.
- **No remote skill registry.** Skills come from disk paths and from
  loaded extensions. Fetching skills from an HTTP endpoint at
  runtime is not in scope.

## Smoke tests (before merging)

These are in addition to the agent SCOPE Part B and flow SCOPE
smokes already passing.

### "Bundle hash is stable across line endings" test

A fixture skill dir is hashed twice — once with `\n` line endings,
once with `\r\n`. Hashes match. A single-byte change to any
non-excluded file changes the hash. Adding a `.DS_Store` file does
not change the hash.

### "Extension-contributed skill is quarantined regardless of frontmatter" test

A `SKILL.md` with `trust: approved` in its frontmatter is contributed
via `extend(...)`. `registry.list_quarantined()` contains it;
`registry.select(matching query)` returns `None`. After
`registry.approve(id, hash, principal)`, `select` returns the skill.

### "Hash mismatch re-quarantines" test

A skill is approved at hash H1. The bundle is edited (one byte in
the body). On `reload()`, the new hash H2 does not match the approval
row; the bundle is quarantined again. The H1 approval row remains
in `ApprovalStore::list()` but is inert.

### "Selection is frozen per run" test

A flow with two `ai-agent` nodes runs against a registry whose
underlying bundle is edited between node A and node B. Node B sees
the *same* `SkillSelection` (same `bundle_hash`, same body) as node
A. (This is the [stage5_skill_threading.rs](../../crates/starter-flow/tests/stage5_skill_threading.rs)
smoke retargeted at a real registry.)

### "Quarantined skill never reaches the selector strategy" test

A custom `SkillSelector` strategy wraps `LlmSelector` and records
every candidate passed in. With one approved and one quarantined
skill, the recorded candidate set contains only the approved one,
regardless of which the LLM would have picked.

### "No I/O on select" test

A registry built against a deliberately-broken `ApprovalStore` (panics
on `lookup`) still serves `select(...)` calls without touching the
store. (Approvals are cached at build time; R-skills-8.)

### "Resource hash mismatch aborts the run" test

A flow run is started with a skill whose resource has hash H1.
Between run-start and the `ai-agent` node mounting the resource,
the on-disk file is edited (hash H2). The node's mount step
verifies the resolved bytes' hash against the frozen
`ResourceRef.content_hash`, finds H1 ≠ H2, and aborts the run with
`SkillResourceHashMismatch`. The error surfaces as a typed node
failure in the engine's run telemetry. Subsequent runs against the
edited bundle see the new content_hash and proceed normally (or
re-quarantine if the bundle hash drifted from an approval row).

### "Path framing prevents collision" test

Two fixture skill dirs differ only in path/content boundary:
bundle A has file `a/b` with content `c`; bundle B has file `a`
with content `b/c`. The two bundle hashes differ. (Pins the
length-prefixed framing in R-skills-2 step 5.)

### "Line-ending normalisation is stable" test

A fixture skill is committed once with CRLF endings and once with
LF endings on text files (`.md`, `.yaml`). Both produce the same
bundle hash. A CR-only (old Mac) line ending in the same files
also produces the same hash. (Pins R-skills-2 step 3.)

## Phasing

| # | Phase | Size | Output |
|---|---|---|---|
| 1 | Crate skeleton + `SKILL.md` parser | S | `starter-skills` member in workspace; parses fixture bundles; unit tests on frontmatter `deny_unknown_fields`. |
| 2 | Content-hash + `EXCLUDED` list + property test | S | `hash_bundle(path)` deterministic across line endings; smoke "Bundle hash is stable" passes. |
| 3 | `SkillRegistry` (load_dir, extend, list, get) + `ApprovalStore` trait + in-memory impl | M | Smoke "Extension-contributed skill is quarantined" passes; smoke "Hash mismatch re-quarantines" passes. |
| 4 | Default `LlmSelector` + `KeywordSelector` + `FirstSelector` | M | Engine `with_skill_selector(registry)` end-to-end; smoke "Selection is frozen per run" passes against a real registry; smoke "Quarantined skill never reaches strategy" passes. |
| 5 | `starter-store-sqlite` + `starter-store-postgres` ApprovalStore impls | S | Approval rows persist across process restart; existing store smokes extended with one more table. |
| 6 | `starter-ext-flow` wires `contributes.skills` (specified in agent R-agent-4, not yet implemented) through `extend(...)` | S | An extension's `skills/` dir lands quarantined; operator-approval CLI surfacing is the host's job, not this crate's. |
| 7 | ai-builder reference skills + end-to-end smoke | S | [`skills/starter.ai-builder.dashboards/`](../../skills/starter.ai-builder.dashboards/) and [`skills/starter.ai-builder.themes/`](../../skills/starter.ai-builder.themes/) already ship on disk (see ["Reference skills shipped in-tree"](#reference-skills-shipped-in-tree)); this phase loads them through `SkillRegistry::load_dir(...)` from [`examples/flow-agent`](../../examples/flow-agent/SCOPE.md), runs the end-to-end smoke, and unblocks ai-builder Phase 5. |

Phases 1–4 are the MVP. Phase 5 closes durability. Phase 6 closes
the extension story. Phase 7 closes the loop with the first
non-test consumer.

## Decisions made

- **One crate, no SPI split.** `SkillSelector` already lives in
  `starter-flow-spi`; `starter-skills` is the implementation. Adding
  a `starter-skills-spi` would mint a seam for no consumer.
- **Trust is determined by load path, not frontmatter.** R-skills-3.
  An extension cannot self-approve. The frontmatter `trust:` field
  is documentary (it tells a reviewer what the bundle author
  *expected*) but never authoritative.
- **`select()` is hot-path; no I/O.** R-skills-8. Approval state is
  cached at build time, refreshed on explicit `reload()`. Hosts that
  want auto-refresh wire a file watcher; that watcher is not this
  crate's concern.
- **Bundle hash *is* the version.** No semver, no compatibility
  matrix. An update is a new hash and re-approval. This matches the
  agent R4 model and avoids the "approved at v1.2.3 but the patch
  changes the body" trap.
- **Default selector is one LLM call, not vector search.** R-skills-5.
  Vector selection adds an embedding pipeline (storage, re-embed on
  change, embedding provider routing) that no current consumer needs.

## Open questions

### S-D1 — Approval surfaces (CLI + HTTP + UI)

The crate exposes `registry.approve(...)`. The user-facing surfaces
that drive it are out of scope for this crate but in scope for the
agent SCOPE to coordinate:

- **CLI** — `starter skills list --quarantined`, `starter skills
  approve <id> --hash <h>`. Lives in `starter-cli`.
- **HTTP** — `GET /api/v1/skills`, `GET /api/v1/skills/{id}`,
  `POST /api/v1/skills/{id}/approve { bundleHash }`,
  `DELETE /api/v1/skills/{id}/approve { bundleHash }`. Lives in
  `starter-server`, mirrored in `openapi.json` and
  `starter-client-ts`. Not implemented in v1.
- **UI** — [`@nube/starter-ui-skills`](../../packages/starter-ui-skills/README.md)
  consumes the HTTP surface via a `SkillsAdapter`. Until the HTTP
  routes land it ships only an in-memory adapter; consumers
  hand-roll one against whatever transport they have (REST, Tauri
  command, GraphQL).

The shapes are aligned: the HTTP body matches `SkillsAdapter` in
the UI package, which matches `registry.approve(id, hash)` here.
Adding the HTTP layer is mechanical; gating on a real operator
need.

### S-D2 — Resource URI scheme broadening (post-v1)

V1 commits to `file://` only (see "Public API surface"). Future
schemes (`ext://` for extension-served resources, `s3://` for large
binary assets) are deferred. The trigger for revisiting is an
extension that needs to ship a multi-GB asset it can't bundle on
disk — at which point a scheme registry is the right shape.

### S-D3 — `model_hint` semantics

The frontmatter allows `model_hint: claude-opus-4-7`. Does the
`ai-agent` node honour it unconditionally, or only when the
`AiRunner` exposes the named model? Default to "best-effort" — pass
through to the runner; if the runner doesn't know the model, fall
back to the runner's default and log. Locking the contract here is
the agent SCOPE's job, not this crate's.

### S-D4 — How does a host re-scan for new bundles?

`reload()` re-runs `load_dir` against the same paths and rebuilds
the in-memory state, holding the write lock briefly. In-flight runs
keep their frozen `SkillSelection` (their `content_hash` and
`bundle_hash` were cloned at run start). When those runs' `ai-agent`
nodes mount resources, the on-mount hash check (see "How it plugs
into the engine" / R-skills-3) is what enforces the quarantine
invariant: if disk drifted, the mount fails the run rather than
silently using new bytes. The host calls `reload()` from wherever
its config-reload signal arrives (SIGHUP, file watcher, admin
endpoint). Not specifying *when* to call is deliberate.

### S-D5 — Should agent R4 drop line-ending normalisation entirely?

R-skills-2 step 3 spells out the exact byte transforms (CRLF → LF,
lone CR → LF, nothing else) because agent R4 currently mandates
normalisation. A simpler alternative: drop normalisation entirely,
require bundle authors to commit LF via `.gitattributes`, and hash
files byte-for-byte. Pros: smaller normative algorithm, no
extension-list to maintain, no UTF-16 corner case. Cons: a Windows
contributor who forgets `.gitattributes` ships a different hash
from a Linux contributor for the same logical content, surfacing as
a spurious re-quarantine. Decision is agent-SCOPE-level (R4 owns
the algorithm); flagging here so the trade-off is in one place.

## Pointers

- Agent SCOPE — owns the `SKILL.md` format and R4:
  [DOCS/agent/SCOPE.md](./SCOPE.md)
- Flow SCOPE — owns the run lifecycle and the `SkillSelector` seam:
  [DOCS/flow/scope/SCOPE.md](../flow/scope/SCOPE.md)
- First downstream consumer — ai-builder Phase 5 is blocked on this
  crate:
  [DOCS/frontend/ai-builder/SCOPE.md](../frontend/ai-builder/SCOPE.md)
- Existing skill seam this crate implements:
  [crates/starter-flow-spi/src/skill.rs](../../crates/starter-flow-spi/src/skill.rs)
- Existing engine smoke proving selection threads correctly:
  [crates/starter-flow/tests/stage5_skill_threading.rs](../../crates/starter-flow/tests/stage5_skill_threading.rs)

## Bottom line

**`starter-skills` is the crate behind the existing `SkillSelector`
seam.** It parses `SKILL.md` bundles, hashes them by the algorithm
agent R4 specifies, runs the quarantine state machine, persists
approvals through an `ApprovalStore` trait, and ships a default
LLM-by-description selector. The engine doesn't change. The
`ai-agent` node doesn't change. Extensions don't get a new
adapter — `starter-ext-flow` already handles `contributes.skills`.
One crate, no new seams, no new wire formats — the last missing
piece between the wired seam and a usable skills system.
