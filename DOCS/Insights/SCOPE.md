# The Insights Capability — Scope

> ⚠ **Read [DOCS/flow/scope/SCOPE.md](../flow/scope/SCOPE.md) and
> [DOCS/agent/SCOPE.md](../agent/SCOPE.md) first.**
> The flow engine is the runtime substrate. The `ai-agent` node kind is
> the agent primitive. This doc owns:
>
> - The **Insights** capability — what an "insight" is, how rules run,
>   what gets persisted, how an operator composes one.
> - **R-ins-1** — one root crate (`starter-insights`); no parallel
>   engine, no parallel registry, no parallel store.
> - **R-ins-2** — rules are reusable units, composed into pipelines.
>   The whole capability is built around this.
> - **R-ins-7** — every step in a pipeline is a rule. Cleaning,
>   resampling, joining, normalising are **derivation rules**
>   (`Dataset → Dataset`); checks are **assertion rules**
>   (`Dataset → Verdict`). One trait, two return shapes, one
>   registry. No parallel "transform" concept.
> - **R-ins-8 — Tags are first-class metadata.** Both `key:value`
>   and bare-tag forms; merged from rule + pipeline-node onto
>   every `Verdict`/`Dataset`; indexed in the verdict store;
>   read by routing, rollup grouping, and frontend filtering.
> - **R-ins-9 — Chaining is the flow engine's job.** Rules compose
>   by being nodes in a flow. Edges, fan-in/out, retry, error
>   routing, run persistence are flow-engine concerns. Insights
>   contributes node *bodies* and *values*, never a parallel
>   orchestrator.
> - **R-ins-10 — AI is a first-class rule kind, not just a meta
>   helper.** `rule.ai-check` (assertion: AI as judge in-line) and
>   `rule.ai-debug` (diagnostic: AI explains rule failures) sit
>   alongside `rule.rust`/`sql`/`rhai`/`derive`. The three skill
>   bundles (`rule-author`, `explain`, `tuner`) remain meta.
> - **R-ins-11 — Quality flags are extensible by pack.** The
>   dirty-data taxonomy (`Gap`, `Stuck`, `OutOfRange`, clock-skew,
>   unit-changed, ...) is contributed by extension packs via
>   namespaced `QualityFlagId`s, the same way rules are contributed.
>   The closed enum was a footgun; the registry is the seam.
> - The `Rule` trait, `Verdict`/`Dataset` shapes, and the six rule
>   authoring surfaces (`rule.sql`, `rule.rhai`, `rule.rust`,
>   `rule.derive`, `rule.ai-check`, `rule.ai-debug`).
> - Materialisation and read paths — verdict rollups, derivation
>   output persistence, the contract the frontend reads against.
> - The three AI-agent skill bundles (`rule-author`, `explain`,
>   `tuner`) and where they bind into a pipeline.
>
> Engine mechanics (graph store, propagator, slot writes, three-level
> stop, run persistence, extension contribution) live in the flow
> SCOPE. LLM-loop mechanics live in the agent SCOPE. This doc
> references those rules where they apply rather than restating them.

## One-line summary

**Insights** is starter's general-purpose analysis-on-data capability:
a library of small, reusable **rules** (SQL queries, Rhai scripts, or
premade Rust impls shipped as extensions) composed into **pipelines**
on the flow engine, with optional AI-agent nodes for authoring rules,
explaining verdicts, and tuning thresholds. It is intentionally
domain-agnostic — the same primitive serves finance anomaly detection,
IoT health monitoring, energy/water baselining, and HVAC optimisation
— because the domain knowledge lives in the rule packs, not in the
engine.

A consumer composing a starter binary gets, by adding **one crate**
(`starter-insights`) to `Cargo.toml`:

- A `Rule` trait + `Verdict` value type, both in `starter-spi`
  (re-exported) so any other crate can return or consume verdicts
  without depending on insights.
- Six rule node kinds (`rule.sql`, `rule.rhai`, `rule.rust`,
  `rule.derive`, `rule.ai-check`, `rule.ai-debug`) contributed to
  `starter-flow-nodes`'s `NodeKindRegistry`. The first three return
  `Verdict` (assertion); `rule.derive` returns `Dataset`;
  `rule.ai-check` returns `Verdict` produced by an LLM judge over
  upstream verdicts + data window; `rule.ai-debug` returns a
  diagnostic slot value triggered off a flow-engine error edge.
  Derivation variants of `sql`/`rhai`/`rust` are selected by the
  rule's declared output type — `rule.derive` is the marker for
  in-pipeline ad-hoc derivations whose body is inline.
- **Rule packs ship as extensions; a rule is a node.** Domain
  rule packs (IoT, energy, finance, HVAC) contribute `Rule` impls
  via the existing `contributes.tools`/`register()` extension
  mechanism. Each registered rule is addressable as a node in any
  flow by `RuleId`. No new contribution kind in `block.yaml` (per
  R-ins-1).
- Two windowing node kinds (`window.tumble`, `window.slide`), a
  multi-source `align` node (time-align + resample N input streams
  to one frame; see R-ins-7), and a `verdict.join` fan-in node — the
  minimum needed to write the pipelines the use cases below demand.
  **Chaining, branching, retry, and error routing between these
  nodes are flow-engine concerns** (R-ins-9); insights ships node
  bodies, not a sub-orchestrator.
- A `RuleRegistry` so a rule defined once is **referenced by `RuleId`
  from any pipeline**, in any flow, on this host or contributed by an
  extension.
- Persistence: verdict history, rule revisions, approval rows — all on
  `starter-store-sqlite`/`-postgres` behind a single `insights`
  feature. **No new database, no new migration runner.**
- AI-agent surfaces via the three skill bundles, dropped into any
  pipeline as ordinary `ai-agent` nodes per agent R-agent-1.

No fork of Windmill, no parallel job runner, no in-house dashboarding.
The flow engine owns topology and orchestration; the `ai-agent` node
kind owns the LLM loop; the tools SCOPE owns side-effecting actions.
Insights owns **the rule** — defined once, composed many times.

## Why this exists

Three forces converge:

1. **Every starter-based product that "does analysis" hits the same
   wall.** It needs to evaluate a condition over a window of data,
   record the verdict, optionally notify, and let a human or agent
   tune the condition over time. Every domain (finance, IoT, energy,
   HVAC) re-rolls this loop with bespoke storage, bespoke scheduling,
   and bespoke "is this thing OK?" semantics. Starter already owns
   scheduling (flow triggers), storage (stores), and orchestration
   (flow engine). Insights extracts the missing primitive: **the rule
   itself**, as a first-class, reusable, composable unit.
2. **Reusability is the whole point.** "Is this device online?" is the
   same question whether the device is a fridge, a boiler, a router,
   or a payment terminal. "Has the moving average crossed the
   baseline?" is the same question whether the metric is kWh, m³, USD,
   or °C. Today these get re-implemented per product with subtly
   different bugs. Insights is structured so that **a generic rule
   ships once, in an extension, and is consumed by reference** — the
   rule pack author owns correctness; the pipeline author owns
   composition; the operator owns thresholds. See R-ins-2.
3. **AI assistance fits naturally, but is not load-bearing.** An LLM
   is good at three things in this loop — proposing a SQL or Rhai
   rule from a schema + samples, narrating a verdict in plain
   language, and proposing threshold deltas from feedback. All three
   are **optional `ai-agent` nodes in the pipeline**, not engine
   features. A consumer who wants pure-deterministic analysis runs the
   pipelines without those nodes; the rules and the engine are
   unchanged. Per agent R2, every LLM call routes through `AiRunner`.

Starter ships this as **one crate** (R-ins-1) rather than a workspace.
The capability decomposes cleanly into rule kinds, windowing, and
verdict join — but those parts only justify their cost together.
Splitting now would invent seams the design does not need; the rule
packs (where the domain knowledge lives) ship as extensions, where
seam discipline is already settled.

## Relationship to existing crates

```
starter-spi                       (Tool, AiRunner, Cancel, Principal,
                                   SecretStore — exists, unchanged.
                                   Gains: Rule, Verdict, RuleId,
                                   RuleSchema, RuleError as re-exports
                                   from starter-insights via a thin
                                   pub-use stub OR inlined directly
                                   into starter-spi. See D1.)
   ▲
   │
   ├── starter-ai                 (5 providers — exists, unchanged)
   │
   ├── starter-flow-spi           (per flow SCOPE — contracts)
   │
   ├── starter-flow               (per flow SCOPE — the engine)
   │
   ├── starter-flow-nodes         (per flow SCOPE — built-in kinds.
   │                               Gains: rule.sql, rule.rhai,
   │                               rule.rust, window.tumble,
   │                               window.slide, verdict.join — each
   │                               behind a cargo feature, default off.
   │                               Implementations live in
   │                               starter-insights and are registered
   │                               by the host at boot.)
   │
   ├── starter-skills             (per agent SCOPE — unchanged.
   │                               Skill bundles ship at
   │                               skills/starter.insights.{rule-author,
   │                               explain, tuner}/SKILL.md per R4
   │                               quarantine rules.)
   │
   ├── starter-store-sqlite       (exists; gains ONE feature = "insights"
   │   starter-store-postgres      with VerdictStore, RuleStore,
   │                               ApprovalStore impls. No separate
   │                               crate.)
   │
   └── starter-insights           (NEW — ONE root crate. THE insights
          capability.)
            Module layout (one crate; modules are not crates):
              rule/             Rule trait, RuleId, RuleSchema, RuleError
              verdict/          Verdict, Severity, EvidenceRow
              window/           Window types + the two window node bodies
              nodes/            rule.sql, rule.rhai, rule.rust,
                                verdict.join — bodies that implement
                                starter-flow-spi::NodeBehavior
              registry/         RuleRegistry (one per host)
              rhai_sandbox/     locked Rhai engine profile (see R-ins-4)
              backfill/         RuleRunStore: replay rule over history
              prelude/          public re-exports
            Depends on: starter-spi, starter-flow-spi, starter-ai
              (only for the optional ai-agent integration helpers —
              feature-gated), starter-store-sqlite (feature-gated).
            Does NOT depend on: starter-server, starter-cli,
              starter-mcp. Surfaces come via flow R8/R9 (FlowAsTool,
              FlowAsService).
```

Domain-specific rule packs live in `starter-extensions/`:

```
starter-extensions/crates/
   ├── starter-ext-insights-iot       Rule impls: is-device-online,
   │                                  sensor-stuck, last-seen-ago, ...
   ├── starter-ext-insights-finance   Rule impls: z-score, isolation-
   │                                  forest-light, duplicate-tx, ...
   ├── starter-ext-insights-energy    Rule impls: baseline-deviation,
   │                                  weekday-vs-weekend, peak-detect, ...
   └── starter-ext-insights-hvac      Rule impls: pmv-comfort,
                                      setpoint-drift, short-cycle, ...
```

Each pack contributes `Rule` impls via the **existing**
`contributes.tools` extension mechanism — a `Rule` is registered with
the `RuleRegistry` by an extension's `register()` method, exactly the
way a `Tool` is registered today. **No new contribution kind in
`block.yaml`.** This is deliberate (R-ins-1): the extensions framework
already knows how to load, validate, quarantine, and version-pin
things; insights does not invent a parallel surface.

## Hard rules (load-bearing)

These rules are written down so future contributors have to argue
*against* them rather than around them. Engine rules (one write
chokepoint, three-level stop, etc.) live in [flow SCOPE](../flow/scope/SCOPE.md)
and apply transitively. Skill rules (content hash, quarantine) live in
[agent SCOPE](../agent/SCOPE.md) and apply to the three insights
skill bundles unchanged.

### R-ins-1 — One root crate

The entire insights capability ships as **one crate**:
`crates/starter-insights/`. There is no `starter-insights-spi`, no
`starter-insights-rhai`, no `starter-insights-sql`, no
`starter-insights-store`. The crate is internally modular (see the
tree above) but externally singular.

The seams that would normally justify a split are already owned by
other crates:

- **Contracts** live in `starter-spi` (the `Rule` trait, `Verdict`,
  `RuleId`). Anything that wants to *consume* a verdict or *return*
  one from a `Tool` depends on `starter-spi` only.
- **Persistence** lives in `starter-store-sqlite` and
  `starter-store-postgres` behind a single `insights` feature flag,
  per the established store pattern. No new store crate.
- **Extension contribution** uses the existing
  `contributes.tools`/`register()` mechanism. No new adapter crate.

**Revisit trigger:** a consumer surfaces a real need to take a strict
sub-slice of the capability (e.g. "I want `Verdict` and `Rule` types
but cannot link Rhai") — at which point the right move is to split
`starter-insights-spi` *out of* `starter-spi`'s re-exports, not to
split insights internals. The internals stay one crate.

### R-ins-2 — Rules are reusable units, composed by reference

**This is the load-bearing rule for the whole capability.** Without
it, insights degenerates into "yet another scripting node kind".

A `Rule` has a stable `RuleId` (namespaced, e.g.
`iot.device.online@1`), a typed input schema, and an `evaluate`
method. Once registered with the `RuleRegistry`, it is referenced by
`RuleId` from any pipeline, anywhere — on this host, in a flow
contributed by an extension, in an ad-hoc admin invocation. Rules are
**not copy-pasted between pipelines, and they do not embed their
thresholds.** Thresholds are inputs to the rule, supplied by the
pipeline that invokes it.

The canonical composition shape the engine is designed around:

```
flow ──► rule.rust(id="iot.device.online@1",  threshold=300s)
       └─► rule.rust(id="iot.sensor.has-recent-data@1", window=15m)
       └─► rule.rhai(id="org.acme.my-custom-check@1",   script_ref="…")
       └─► verdict.join(mode=all|any|weighted)
       └─► action.notify(tool="slack")          ← only fires on the joined verdict
```

The three rule nodes are **independent invocations of independent
reusable rules**, joined into one composite verdict. The first two
ship in `starter-ext-insights-iot`; an operator writes only the
third — *their own check, against their own thresholds, in their own
flow*. They never re-author "is the device online" because that rule
already exists in the registry.

This is the inverse of how scripting platforms typically work
(everyone re-writes the same checks slightly differently). It is also
the inverse of how monitoring systems typically work (rules live in a
central config that nobody owns). Insights' answer: **rule packs
publish; pipelines compose; thresholds are wiring.**

Mechanical enforcement (smoke-tests, not proofs — see caveat below):

- A `Rule` is `Send + Sync + 'static` and **carries no per-pipeline
  state**. Thresholds, baselines, lookback windows are all parameters
  to `evaluate(ctx, input)`. CI smoke (in `crates/starter-insights/
  tests/`) constructs a rule, invokes it twice with different
  thresholds, and asserts both produce coherent verdicts — a rule
  that captured thresholds at construction time would fail this test.
  This catches **captured** state; it does **not** catch implicit
  state (a `static mut`, a process-global cache, a Rhai script that
  mutates a closed-over map). A clippy gate forbids `static mut` and
  `lazy_static!`/`OnceCell` writeable from `evaluate` in
  `starter-insights` and its rule-pack extensions; beyond that, this
  rule is honour-system for rule authors. Reviewers of rule packs
  should treat any per-call mutation visible across calls as a bug.
- `RuleId` is a `(namespace, name, semver)` triple. The registry
  rejects duplicate `(namespace, name, major)` registrations: a
  breaking change requires a new major. Pipelines pin the major they
  rely on.
- `rule.sql`, `rule.rhai`, and `rule.derive` instances are **also
  registered with a `RuleId`**. A SQL or Rhai rule written by an
  operator becomes a first-class registry entry — it can be
  referenced from a second pipeline by id, and it shows up in
  `RuleRegistry::list()` next to the Rust-coded rules from extension
  packs. Inline scripts that are *not* registered are allowed
  (one-shot use), but get a generated anonymous id and a warning in
  the run log; the rule-author agent (skill
  `starter.insights.rule-author`) proposes promotion to a named entry
  as part of its output.

The three use cases below are deliberately framed in terms of this
rule:

> *"Is the device online?"* — common, reusable.
>   Ships in `starter-ext-insights-iot` as
>   `iot.device.online@1`. Inputs: `device_id`, `threshold_secs`.
>   Output: `Verdict::healthy` or `Verdict::flagged(reason, …)`.
>   **Never** re-implemented per product.
>
> *"Check its data is sensible."* — common, reusable.
>   Ships in `starter-ext-insights-iot` as
>   `iot.sensor.has-recent-data@1` (and friends:
>   `iot.sensor.in-range@1`, `iot.sensor.not-stuck@1`).
>   **Never** re-implemented per product.
>
> *"…no, do **my** rule."* — the operator's specific check, the
>   one nobody else has written, the reason the product exists.
>   Authored as `rule.rhai` or `rule.sql`, registered as
>   `org.acme.my-custom-check@1`, composed into the same pipeline
>   alongside the reusable rules above via `verdict.join`. **The
>   operator writes only this.**

If the design ever forces the operator to re-implement the first two
to get the third, R-ins-2 has been violated and the design is wrong.

### R-ins-3 — Six rule authoring surfaces, one Rule trait

Authoring shapes:

- **`rule.rust`** — premade, compiled, fastest path. Ships in
  extension packs. Dispatch by `RuleId` to a `Rule` impl registered
  at extension load. This is where domain knowledge lives. Can be
  either an assertion (→ `Verdict`) or a derivation (→ `Dataset`);
  declared in the rule's `RuleSchema`.
- **`rule.sql`** — parameterised SELECT against the host's store or a
  registered read-only `SqlSource`. Parameters bound, never
  string-concatenated (OWASP A03). Result rows map to
  `Verdict::EvidenceRow` (assertion) or `Dataset::Rows` (derivation),
  selected by the rule's declared output type. Useful for joins
  against historical state and for operators who think in SQL.
- **`rule.rhai`** — sandboxed Rhai script with a typed `Ctx`
  (metrics, prev window value, thresholds, no I/O). AST compiled at
  registration time and cached. Can return either `Verdict` or
  `Dataset`. Useful for thresholding, arithmetic, small state
  machines, and lightweight resampling/filling.
- **`rule.derive`** — the marker node kind for in-pipeline,
  anonymous, one-shot derivations whose body is `rule.rhai` or
  `rule.sql`. Surfaces in a pipeline as a clearly-labelled
  data-transform step. Registered derivations declared in extension
  packs use `rule.rust` or `rule.rhai` directly — this node kind
  exists for authoring ergonomics, not for a parallel runtime.
- **`rule.ai-check`** — *assertion* rule whose body is an LLM call
  routed through `AiRunner` (R-ins-5). Input: one or more upstream
  `Verdict`s + the underlying `Dataset` window + a bound skill
  bundle that constrains its tool access. Output: a `Verdict` like
  any other (severity, coverage, summary, evidence). Use case:
  upstream rules flagged an anomaly; the AI judge inspects the raw
  window and decides "yes, real incident" vs "no, defrost cycle /
  scheduled maintenance / known noise pattern." Has a stable
  `RuleId`; pipeline pins a model. **Cannot declare `persist: true`**
  in `RuleSchema` — AI outputs are non-deterministic and would
  violate the backfill-determinism smoke. Logged but not cached.
- **`rule.ai-debug`** — *diagnostic* rule, not an assertion. Placed
  downstream of a flow `branch` that routes `Severity::Error`
  verdicts. Input: the `Error` verdict + the failing rule's body
  (Rhai/SQL text, or Rust schema) + the inputs that were present.
  Output: a structured `RuleErrorDiagnosis` slot value
  (`likely_cause`, `suggested_fix`, `confidence`, human summary).
  Never emits a `Verdict`; never gates an action. Feeds
  `action.notify` (ops channel) or drafts a revision via the
  `tuner` skill.

All six implement the same `Rule` trait. **Pipelines do not branch
on authoring surface** — `verdict.join` consumes `Verdict` values
and `align`/derive consumers consume `Dataset` values regardless of
where they came from. This is the test of whether the surfaces are
the same primitive: if a consumer node would have to know it was
reading from an LLM vs Rust vs Rhai, the design has drifted.

### R-ins-4 — The Rhai sandbox profile is locked in this SCOPE

`rule.rhai` is the highest user-input surface in the capability.
Profile, frozen here, enforced in `starter-insights::rhai_sandbox`,
smoke-tested in CI:

- `Engine::new()`, then **deny** `eval`, `import`, modules, and the
  filesystem package.
- Default `set_max_operations(1_000_000)`, `set_max_expr_depth(32, 32)`,
  `set_max_string_size(64 * 1024)`,
  `set_max_array_size(10_000)`, `set_max_map_size(10_000)`.
  The operation cap is **per-rule overrideable** (registry entry
  carries `max_operations: Option<u64>`); pipelines composing a rule
  do not get to raise it. The default is sized for a 24h × 1-min
  window at ~50 ops/sample with headroom (~1.44M × 50 = 72M is
  well-clear-of-default-budget territory, i.e. that workload belongs
  in `rule.rust`; the default supports up to a 24h × 5-min window of
  similar arithmetic in Rhai). Complex math, ML, or long-window
  state belongs in `rule.rust`; the cap is the forcing function.
- No registration of any Rust function that does I/O, time mutation,
  or capability acquisition. The `Ctx` exposed to scripts is a
  read-only typed view of the rule's input slots plus a `now()`
  bound to the engine's clock seam.
- A CI test loads a fixture file of known-bad scripts (DoS, escape
  attempts, package imports) and asserts each one fails to compile
  or fails at runtime within the operation budget.

**Revisit trigger:** a consumer needs scripted *side effects* (write
back to a store, call a tool). That is **not a Rhai change** —
side effects belong to downstream nodes (`tool-call`, `http-out`)
whose dispatch the engine governs per flow R3. The Rhai sandbox
stays read-only.

### R-ins-5 — AI-agent assistance is optional and route-through-AiRunner

Insights ships three skill bundles, all under
`skills/starter.insights.*/`. Each is dropped into a pipeline as an
ordinary `ai-agent` node (agent R-agent-1); the skill restricts its
tool access per agent R4. Operators with no LLM provider configured
omit the nodes — pipelines run unchanged.

- **`starter.insights.rule-author`** — proposes a `rule.sql` or
  `rule.rhai` body from a schema + sample rows. Dry-runs against
  history via `RuleRunStore::backfill`. **Output is a draft, never
  an active rule.** Promotion to active goes through the same
  content-hash approval flow as skills (agent R4 reused verbatim).
- **`starter.insights.explain`** — narrates a `Verdict` in plain
  language given the window of data that produced it. Triggered
  downstream of `verdict.join` when severity ≥ configured threshold.
  Output is a slot value (text + structured `recommended_actions`),
  not a side-effect. A downstream `tool-call` or `action.notify`
  may emit the side-effect.
- **`starter.insights.tuner`** — scheduled trigger, reads
  false-positive/false-negative feedback (a `feedback` node kind
  writes them through the engine chokepoint), proposes threshold
  deltas, opens a draft rule revision. **Never auto-applies.**

Every LLM call from these agents routes through
`starter-ai::AiRunner`. The same CI dep-tree gate that agent R2
applies to `starter-flow-node-loop` applies to `starter-insights`:
no provider SDK in the dep tree.

### R-ins-6 — Verdict is the only currency between rule and action

Downstream side-effecting nodes (notify, gate, tool-call, sub-flow)
consume `Verdict`, not the rule's internal state. A `Verdict` is:

```rust
#[non_exhaustive]
pub struct Verdict {
    pub rule_id:    RuleId,
    pub at:         OffsetDateTime,        // UTC, always
    pub tz:         TimeZoneId,            // IANA, e.g. "Europe/London"
    pub window:     Window,                // window.start/end carry tz too
    pub severity:   Severity,              // see below
    pub coverage:   Coverage,              // see below — first-class
    pub tags:       Tags,                  // per R-ins-8
    pub summary:    String,                // short, machine + human readable
    pub evidence:   Vec<EvidenceRow>,      // typed columns; bounded size
    pub correlation_id: Option<Uuid>,      // joins to the run that emitted it
}

#[non_exhaustive]
pub enum Severity {
    Healthy, Info, Warn, Critical,
    Error,    // rule could not produce an opinion — see "Failure is a verdict"
}

#[non_exhaustive]
pub struct Coverage {
    pub raw:              RawCoverage,        // immutable, set at align/source
    pub effective:        EffectiveCoverage,  // mutable across derivations
    pub quality_flags:    Vec<QualityFlag>,   // see R-ins-11; extensible
}

#[non_exhaustive]
pub struct RawCoverage {
    pub samples_expected: u64,
    pub samples_present:  u64,    // original samples, never synthetics
    pub confidence:       f32,    // 0.0..=1.0; set at the source/align,
                                  // NEVER mutated downstream
}

#[non_exhaustive]
pub struct EffectiveCoverage {
    pub confidence:       f32,    // 0.0..=1.0; raw.confidence × product of
                                  // confidence_penalty values from every
                                  // derivation rule in the chain
    pub penalty_chain:    Vec<(RuleId, f32)>,  // audit: who applied what
}
```

**Coverage mutation contract (load-bearing):**

- `raw` is **immutable** once set (at the source node, or at `align`
  when multiple sources are combined). Downstream nodes copy it
  through unchanged. A rule body that touches `raw` is a bug.
- `effective.confidence` is mutated **only via the engine**, not by
  the rule body. Derivation rules declare a
  `confidence_penalty: f32` in `[0.0, 1.0]` in their `RuleSchema`;
  the engine multiplies `effective.confidence` by the penalty
  before passing the `Dataset` downstream, and appends
  `(rule_id, penalty)` to `penalty_chain` for audit. A rule body
  that writes `effective` directly is a bug — caught by the
  derivation determinism smoke (R-ins-2).
- **Derivations can only lower or preserve confidence**, never
  raise it. A `confidence_penalty > 1.0` is rejected at registry
  registration time. A denoiser that genuinely believes its output
  is "cleaner than the input" should emit a `Verdict` (assertion)
  saying so, not claim higher `effective.confidence` on a
  `Dataset`. This invariant means `gate(min_confidence=0.8)`
  cannot be gamed by an upstream derivation chain.
- `fill-gaps@3 strategy=linear` declares `confidence_penalty: 0.8`
  (or similar, tuned per pack); `despike@2` declares `0.95`;
  `weather.resample.15m-to-1m@1` declares `0.9`. Operators see the
  penalty chain in the run log and can argue with it.

`Coverage` is **first-class, not optional**. A rule evaluating a
window where 40% of samples were missing returns a `Verdict` with
`coverage.raw.samples_present / samples_expected = 0.6` and a
`Gap` quality flag — separately from whether the *measured* value
breached its threshold. Downstream nodes (and the explainer agent)
use `coverage` to distinguish "the building is misbehaving" from
"the data is misbehaving." Notification rules that fire on
`severity >= Warn` typically also require
`coverage.effective.confidence >= 0.8` before alerting a human;
the threshold is pipeline config. **`gate` reads `effective`, not
`raw`** — by the time a verdict reaches a gate, it has already
been discounted by every derivation in the chain.

`tz` is mandatory because energy/HVAC/finance pipelines that compare
"this Tuesday vs last Tuesday" are wrong by an hour twice a year if
DST is implicit. Windowing nodes (`window.tumble`, `window.slide`)
carry a `tz` config and emit window boundaries in that zone; all
timestamps in `Verdict` and `Dataset` are UTC instants paired with
the `tz` they were *computed against*.

`verdict.join` produces a `Verdict` whose `rule_id` is a synthetic
`(pipeline_namespace, pipeline_name, semver)` triple, so a joined
verdict is itself a first-class registry citizen — addressable,
explainable, tunable. `coverage.effective.confidence` on a joined
verdict is the weighted-min of its non-`Error` inputs (a join
across rules cannot claim higher confidence than its weakest
contributing input); `coverage.raw` on the join is the union of
the inputs' `raw` blocks. `penalty_chain` is concatenated across
inputs so the audit trail is complete.

`verdict.join` modes:
- `all` — `Critical` if any input is `Critical`; else max severity.
  An `Error` input propagates as `Error` (the joined verdict cannot
  claim health it doesn't have).
- `any` — fires if any input is `>= Warn`. `Error` inputs propagate
  as `Error`, not as `Warn`.
- `weighted` — each input has a `weight: f32` declared in **the
  pipeline config** (the node that invokes `verdict.join`), not on
  the rule. Rules are reusable across pipelines; their weight in a
  composite is a property of the composite, not the rule. `Error`
  inputs are **excluded** from the weighted sum and their weight is
  redistributed proportionally across the remaining inputs; the
  joined verdict's `quality_flags` records which inputs errored.
  The tuner agent (skill `starter.insights.tuner`) proposes weight
  deltas the same way it proposes threshold deltas — as drafts,
  gated by approval. Per-rule "default weight" hints are explicitly
  out of scope; reviewers should reject any PR adding them.

**Degenerate cases of `verdict.join`:**

- **All inputs errored.** Emit `Severity::Error`,
  `effective.confidence = 0.0`,
  `quality_flags: [JoinAllInputsErrored, ...input_errors]`.
  No "fallback to healthy", no "fallback to last good join" — the
  join cannot synthesise an opinion it doesn't have.
- **Zero inputs.** Configuration error; the engine rejects this at
  flow validation time (per flow R3, the engine reads policies).
  Never silently emit a `Healthy` verdict from an empty fan-in.
- **Single input.** Pass-through, with the join's `rule_id` still
  applied so the joined verdict remains addressable.

**Failure is a verdict, not an exception.** A rule that cannot
produce an opinion — body errored, input slot missing, Rhai/SQL
budget exhausted — emits a `Verdict` with `Severity::Error` and a
`QualityFlag::RuleError(kind)`. Rules **never** `panic!` and
**never** return `Err` from `evaluate`; the rule node body catches
internal failures and converts them. This is load-bearing: a flaky
rule must not short-circuit the pipeline, because that would
silently take downstream alerting offline. Low coverage is **not**
failure — it is a normal verdict with low `confidence`, suppressed
by `gate` per its config. The two are distinct quality_flags
(`Gap`/`Stuck`/`OutOfRange` vs `RuleError`) so downstream consumers
can tell "the data is bad" from "the rule is broken."

Chaining, retry, and error routing are **flow-engine concerns**
(R-ins-9). Rule nodes emit `Severity::Error` on their normal
output edge; flow authors wire downstream `branch`/`gate` nodes
that route on `severity`. The canonical error pattern is:

```
rule.rust → branch(on=severity)
              ├─► (Healthy|Info|Warn|Critical) → verdict.join → gate → notify
              └─► (Error)                       → rule.ai-debug → notify(ops)
```

Engine-level retry policy (`retry: { max: N, backoff: ... }`) is
configured on the rule node per flow SCOPE; the rule itself does
not retry. A rule that internally retries hides flakiness from the
operator.

Mechanical: `Verdict`, `Severity`, `Coverage`, `Dataset`, and
`Tags` are `#[non_exhaustive]`; consumers match on the fields they
care about. New fields/variants are minor-version additions.
Removing or repurposing a field is a major.

### R-ins-7 — Derivation rules and assertion rules share one trait

`Rule` returns one of two shapes:

```rust
pub enum RuleOutput {
    Assertion(Verdict),
    Derivation(Dataset),
}

#[non_exhaustive]
pub struct Dataset {
    pub schema:   DatasetSchema,           // typed columns
    pub rows:     Arc<dyn DatasetRows>,    // streamable; bounded
    pub coverage: Coverage,                // same Coverage as Verdict
    pub tz:       TimeZoneId,
    pub window:   Option<Window>,
}
```

The declared output is part of the `RuleSchema` at registration
time; pipelines bind by `RuleId` and the engine type-checks the
wire-up before run. **Cleaning, resampling, gap-filling,
normalising, joining multiple sources — all of these are
derivation rules.** They live in the same registry, are versioned
the same way (semver `RuleId`), are quarantined the same way when
contributed by an extension, and are tested the same way (backfill
over history, assert determinism per R-ins-2).

Canonical building-energy chain (the ugly example R-ins-2 needs to
survive):

```
trigger (1h schedule, tz="Europe/London")
   ├─► source.meter        (sqlite,  1-min raw)
   ├─► source.weather      (http,   15-min, may 503)
   ├─► source.tariff       (sqlite,  static-ish + retroactive corrections)
   └─► source.occupancy    (events,  sparse)
         │
         ▼
   align(tz="Europe/London", frame=15m, on=missing_input=mark_gap)
         │
         ▼
   rule.derive(id="energy.meter.despike@2")
   rule.derive(id="energy.meter.fill-gaps@3", strategy="linear", max_gap=2)
   rule.derive(id="weather.resample.15m-to-1m@1")        ← reusable, ships in pack
   rule.derive(id="energy.normalise.weather@2",          ← reusable
               inputs=["meter_filled", "weather_resampled"])
         │
         ▼  (Dataset: cleaned, normalised, with Coverage carried through)
         ├─► rule.rust(id="energy.baseline.deviation@1", baseline="prev-week")
         ├─► rule.rust(id="energy.peak.detect@1")
         └─► rule.rhai(id="org.acme.tariff-window-overrun@1", … bill data …)
              │
              ▼
   verdict.join(mode=weighted, weights=…)                ← weights in pipeline cfg
         │
         ▼
   rule.ai-check(id="org.acme.energy-judge@1")           ← AI as in-line judge (R-ins-10)
         │
         ▼
   branch(on=severity)                                    ← flow built-in (R-ins-9)
   ├─► (Healthy|Info|Warn|Critical)
   │     │
   │     ▼
   │   gate(min_severity=Warn, min_confidence=0.8)        ← suppresses low-coverage
   │     │
   │     ▼
   │   ai-agent(skill="starter.insights.explain")
   │     │
   │     ▼
   │   action.notify(tool="email", tag_route="building")  ← route by tag (R-ins-8)
   │
   └─► (Error)
         │
         ▼
       rule.ai-debug(id="org.acme.energy-debugger@1")     ← AI explains the failure
         │
         ▼
       action.notify(tool="slack", channel="#ops-rules")
```

Notes on this shape:

- The operator authored **one** rule (`org.acme.tariff-window-overrun@1`).
  Every other rule is reusable, from a pack. R-ins-2 holds.
- `align` is a **node**, not a rule — it's the multi-source
  primitive every derivation chain needs. It does not return a
  `Verdict` or a single `Dataset`; it produces a *frame* (an
  ordered tuple of co-time-indexed datasets) that the next
  derivation rules consume. Frames are an internal slot value type;
  they are not surfaced as a public registry concept. **`align` is
  the most domain-loaded node** (timezone, frame size, gap policy,
  reorder buffer), so it carries a `NodeId` with the same
  `(namespace, name, semver)` shape as `RuleId`, surfaced in run
  logs and propagated to `Verdict.evidence` provenance. It is not
  a rule (returns a frame, not a `Verdict`/`Dataset`); the audit
  treatment matches anyway. `align` also **sets `raw.confidence`**
  for the rest of the chain — typically `samples_present /
  samples_expected` adjusted by the configured gap policy.
- `Coverage` is computed at `align` (gap = missing input in a frame)
  and is propagated by every well-behaved derivation rule. The CI
  smoke for `starter-ext-insights-energy` asserts coverage
  propagation; a derivation that drops coverage is a bug.
- `gate` is not new — it's already a flow built-in. Its config here
  reads `min_severity` and `min_confidence` (read against
  `coverage.effective.confidence`) to suppress low-confidence
  alerts. This is the load-bearing payoff of making `Coverage`
  first-class **and** of the raw/effective split — the gate cannot
  be gamed by an upstream derivation laundering confidence.

**Iteration is out of scope.** Real-world cleaning sometimes wants
fixed-point semantics ("despike, then fill, then re-despike because
filling exposed new spikes"). Insights does **not** ship an
`until_stable(max_iters=N)` node — that would be a flow-engine
concern (R-ins-9), useful well beyond insights (tool-use loops,
agent ReAct cycles). If the flow engine grows iteration, insights
will consume it. Until then, operators hand-unroll: chain N copies
of the derivation rule in the pipeline. Two-pass cleaning is
common; three-pass is rare; if you want N-pass, the answer is
"write a `rule.rust` that does the iteration internally with a
bounded inner loop." Either way, insights does not invent its own
iteration surface.

**Revisit trigger:** a consumer needs an unbounded-stream
derivation (event-time semantics, watermarks, late-arriving data
beyond `align`'s reorder buffer). That is out of scope for Insights
as non-goals state; the right move is to ingest from a dedicated
stream processor and treat the materialised result as a `source.*`
in the diagram above.

### R-ins-8 — Tags are first-class metadata

Every `Verdict` and `Dataset` carries a `Tags` value. Tags support
two forms:

- **`key:value`** — `building:hq-london`, `tenant:acme`,
  `cost-centre:facilities-emea`.
- **bare tag** — `critical`, `regulated`, `ai-verified`. Sugar for
  presence-as-signal.

```rust
#[non_exhaustive]
pub struct Tags(pub BTreeMap<String, TagValue>);

pub enum TagValue {
    Flag,                 // bare tag — presence is the signal
    Value(String),        // key:value
}
```

YAML surface (in a flow definition):

```yaml
- node: rule.rust:energy.baseline.deviation@1
  tags:
    - critical                           # bare → TagValue::Flag
    - building:hq-london                 # key:value
    - tenant:acme
    - cost-centre:facilities-emea
```

**Where tags live:**

- **On the rule definition** (`RuleSchema`) — describes *what the
  rule is*. Set by the pack author. Examples: `domain:energy`,
  `kind:assertion`, `stability:stable`. Immutable within a `RuleId`
  version.
- **On the pipeline node** — describes *this use of the rule*. Set
  by the operator. Examples: `building:hq-london`, `tenant:acme`,
  `critical`.
- **On the emitted `Verdict`/`Dataset`** — `value.tags = rule.tags
  ∪ pipeline_node.tags`, union-merged at emit time. If the same
  key appears in both, **the pipeline node wins** (operator intent
  overrides pack defaults). Stored alongside the verdict in the
  log; indexed.

**What tags are for:**

1. **Routing.** `action.notify(tag_route="building")` reads
   `tags["building"]` to pick a Slack channel; no per-pipeline
   if/else. `tag_filter=critical` to only fire on tagged-critical
   verdicts.
2. **Rollup grouping.** `rollup.day` groups by `tags["building"]`
   or `tags["tenant"]` to produce per-tenant aggregates without
   bespoke SQL per pipeline.
3. **Frontend filtering.** "All `Warn+` verdicts tagged
   `building:hq-london` in the last 7 days" is one SELECT against
   the verdict log with a tag predicate. The materialisation read
   contract (below) honours tags as first-class filters.
4. **Registry search.** `RuleRegistry::list_by_tag("domain:energy")`
   so the rule-author agent can scope its proposals.
5. **Cost attribution.** Tag pipelines with `cost-centre:foo` and
   the run log aggregates LLM/compute spend per tenant.

**What tags are NOT for:**

- **Not for thresholds.** Thresholds are inputs (R-ins-2). A tag
  like `threshold:300` would be a regression.
- **Not for authorisation.** Tenant isolation is per `Principal`
  (in `starter-spi`), not per tag. Tags are metadata; principals
  are policy.
- **Not for routing logic inside a rule.** A rule that branches on
  `tags["building"]` is back to "domain knowledge in the
  pipeline" — wrong direction.
- **Not for unbounded identifiers.** Tag values are bounded value
  spaces (`building:hq-london` yes, `transaction-id:<uuid>` no).
  Unbounded identifiers go in `evidence`, not in `tags`. The tag
  index assumes low-cardinality values; an unbounded value space
  poisons the index and breaks the frontend filter contract.
  Rules whose tag values are unavoidably high-cardinality declare
  `starter.high-cardinality: true` on themselves, which excludes
  their tags from the index (still readable in the verdict log,
  but not filter-indexed). The default lint flags any rule whose
  emitted tag values exceed a configured cardinality budget over
  a rolling window.

**Mechanical:**

- Keys: lowercase, `[a-z0-9-]+`, max 64 chars. Reserved namespace:
  `starter.*` for built-in tags (`starter.severity`,
  `starter.rule-error`, `starter.tags-truncated`).
- Values: max 256 chars, UTF-8.
- Max 32 tags per `Verdict`/`Dataset`. Exceeding → first 32 kept,
  `starter.tags-truncated` flag added.
- Indexed in the verdict store. Sqlite: a separate
  `verdict_tag(verdict_id, key, value)` table with composite
  index. Postgres: GIN on a jsonb column. Without an index, point 3
  above is a lie.

### R-ins-9 — Chaining is the flow engine's job

Rules compose by being placed as nodes in a flow. **Edges,
fan-in, fan-out, retry, error routing, branching, and run
persistence are flow-engine concerns** (per flow R1 "Everything is
a Node" and flow R3 "engine is the policy reader"). Insights
contributes node *bodies* (`rule.*`, `verdict.join`, `align`,
`window.*`, `rollup.*`) and the `Verdict`/`Dataset` *values* that
flow on the edges between them. It does not contribute a parallel
graph, a parallel scheduler, or a parallel retry surface.

Concretely:

- A "rule pipeline" is a flow. There is no separate "rule chain"
  config file.
- An "error route" is an edge wired off a `branch(on=severity)`
  node — a flow built-in.
- A "retry" is a flow-node retry policy
  (`retry: { max: N, backoff: ... }`), configured on the rule
  node, applied by the engine.
- A "fan-out" (one source feeding many rules) is just multiple
  edges out of a node — the engine handles it.
- A "fan-in" (multiple rules feeding `verdict.join`) is the engine
  firing the join when all inputs have written their slots.

Insights' contribution at this layer is exactly two things:

1. The convention that **rule failures become `Severity::Error`
   verdicts**, not node-level errors. This keeps failures on the
   normal output edge, where they are queryable, joinable, and
   routable by the same `branch`/`gate` machinery the engine
   already ships.
2. `verdict.join`'s mode semantics (`all`/`any`/`weighted`) — how
   to *combine* verdicts. The engine fires the join; the body
   decides the result.

**Revisit trigger:** any future feature proposal that introduces a
parallel graph, a parallel scheduler, or a "rule sub-flow" concept
has violated R-ins-9. Reviewers should reject; the right fix is to
add a node body, not a parallel orchestrator.

### R-ins-10 — AI is a first-class rule kind

AI participates in pipelines at **three distinct layers**, each
with different audit, persistence, and trust properties. Conflating
them is a design smell.

| Layer | Surface | Where it sits | Output | Audited as |
|---|---|---|---|---|
| **In-line judge** | `rule.ai-check` | A node in the rule chain | `Verdict` | A rule (registry, RuleId, semver) |
| **In-line diagnostic** | `rule.ai-debug` | Off the `Error` branch | `RuleErrorDiagnosis` slot value | Diagnostic (never gates an action) |
| **Meta** | Three skill bundles | `ai-agent` nodes used out-of-band (author, explain, tune) | Drafts / narrations / proposals | Approval-flow per agent R4 |

**AI is KEY** to the value the capability delivers, but the way
it is key matters:

- `rule.ai-check` is what lets a pipeline say "the deterministic
  rules flagged something; the AI judge weighed the raw window
  against domain context and decided this is/isn't a real incident."
  Without it, AI is forever the *narrator* and never the *judge*,
  and operators end up writing brittle Rhai to encode judgement
  that an LLM does better. With it, AI verdicts are first-class:
  joined, gated, tagged, and rolled up alongside Rust/Rhai/SQL
  verdicts.
- `rule.ai-debug` is what closes the loop on rule failures.
  Per R-ins-6, failures are `Severity::Error` verdicts; the
  canonical pattern wires those to an `ai-debug` node that
  inspects the body, the inputs, and the recent run history, then
  emits a `RuleErrorDiagnosis` to ops. This is **the** answer to
  "what if a rule fails": a deterministic error verdict + an AI
  explanation, side by side.
- The three skill bundles (`rule-author`, `explain`, `tuner`) stay
  out-of-band, doing what they do today — drafting rules,
  narrating verdicts after the fact, proposing threshold deltas
  for human approval.

**Constraints on AI rule kinds (R-ins-10):**

- All AI rule bodies route through `AiRunner` (R-ins-5). The same
  CI dep-tree gate applies: no provider SDK in the dep tree.
- `rule.ai-check` carries a `RuleId` and pins a **model family**
  (e.g. `claude-opus-4.x`) + skill bundle in its `RuleSchema`. The
  model *family* is part of the rule's identity (changing it is a
  major bump); the *exact* model that ran is recorded on each
  `Verdict.evidence` for audit. **Audit ≠ identity.** This is
  deliberate: provider patch-level deprecations should not force a
  registry churn across every pipeline using the judge, but every
  emitted verdict must be reproducible enough to defend in a post-
  mortem. A cross-family upgrade (Claude → GPT, Opus → Sonnet) is
  always a major because the behavioural envelope changes.
- `rule.ai-check` **cannot** set `persist: true` (non-deterministic
  outputs would violate the backfill-determinism smoke). Its
  `RuleSchema` carries `non_deterministic: true`, which is the
  signal the determinism smoke uses to skip it; the skip is logged
  per-rule in the smoke output so it cannot silently widen.
  Verdicts are written to the verdict log like any other rule, but
  are not cached as a derivation. A run is re-runnable, but it is
  not bit-reproducible.
- `rule.ai-debug` outputs are stored in the run log (for ops
  audit) but **not** in the verdict log (it never emits a
  `Verdict`). It cannot drive an `action.*` that has side effects
  on the monitored system; it can only drive `action.notify` and
  `tuner`-style draft proposals.
- Tags `starter.ai-check`, `starter.ai-debug`, and
  `starter.ai-model:<name>` are auto-applied by the engine to
  outputs of AI rule kinds, so downstream filtering can isolate
  "things an AI decided" from "things only deterministic code
  decided." This is load-bearing for regulated domains where
  human-attestation requirements differ.

**Revisit trigger:** a consumer needs an AI rule to *modify* the
system (write to a store, call a tool). That is **not an AI rule
change** — side effects belong to downstream nodes (`tool-call`,
`http-out`) per flow R3, gated by an approval node (per the HVAC
use case). The AI judge proposes; deterministic gates dispose.

**`rule.ai-check` vs the `explain` skill bundle — when to use
which.** Both are LLM-driven and both look at a verdict + the
underlying window. The distinction is load-bearing:

> **`rule.ai-check` decides. `explain` describes.**
> **Never reverse them.**

- If the LLM's output *gates an action* — feeds `verdict.join`,
  changes the joined severity, decides whether `action.notify`
  fires — it must be `rule.ai-check`. It has a `RuleId`, lands in
  the verdict log, is audited, is tunable.
- If the LLM's output *follows* an already-decided verdict — adds
  human-readable narration, recommends remediation steps, drafts
  a Slack message — it must be the `explain` skill bundle. Its
  output is text, not a verdict; it cannot change what fires.

A frequent design smell: an `explain` agent whose narration is
parsed downstream to decide routing. If you find yourself doing
this, you needed `ai-check`. Promote it.

### R-ins-11 — Quality flags are extensible by pack

The dirty-data taxonomy is too domain-loaded to fit in a closed
`enum QualityFlag` in `starter-spi`. The space includes — at
minimum — gaps, stuck sensors, out-of-range, clock skew, duplicate
timestamps, unit changes mid-stream (kW vs kWh), sensor swaps
(same device_id, different calibration), retroactive corrections,
late-arriving data, and N more per domain. A closed enum forces
every new pattern through a `starter-spi` PR — exactly the
extension-vs-core anti-pattern R-ins-1 was written to avoid.

The right shape mirrors `RuleId`:

```rust
#[non_exhaustive]
pub struct QualityFlag {
    pub id:       QualityFlagId,        // (namespace, name, semver)
    pub severity: QualityFlagSeverity,  // Info | Warn | Critical
    pub detail:   Option<String>,       // bounded; for evidence rows
}

pub struct QualityFlagId {
    pub namespace: String,   // e.g. "starter.quality", "energy.quality"
    pub name:      String,   // e.g. "gap", "stuck", "unit-changed"
    pub major:     u32,
}
```

**Built-ins in `starter-spi`:**

- `starter.quality.gap@1` — samples missing in a window.
- `starter.quality.stuck@1` — N consecutive samples identical.
- `starter.quality.out-of-range@1` — value outside declared bounds.
- `starter.quality.rule-error@1` — emitted with `Severity::Error`
  verdicts; carries `kind` (BodyFailed | InputMissing |
  BudgetExhausted) in `detail`.
- `starter.quality.join-all-inputs-errored@1` — see `verdict.join`
  degenerate case.
- `starter.quality.tags-truncated@1` — over the 32-tag cap.

**Pack-contributed examples:**

- `energy.quality.unit-changed@1` (kW ↔ kWh mid-stream).
- `energy.quality.retroactive-correction@1` (tariff fixup landed).
- `iot.quality.clock-skew@1` (sample timestamps drifting).
- `iot.quality.sensor-swap@1` (calibration changed under same id).
- `finance.quality.duplicate-timestamp@1`.

**Registration:** quality flags ship via the same
`contributes.tools`/`register()` extension mechanism as rules. A
pack registers its `QualityFlagId`s with a `QualityFlagRegistry`
that lives next to `RuleRegistry`. Registration carries a
short human description and a remediation hint, both rendered by
the explainer agent and the frontend without per-domain code.

**Mechanical:**

- `QualityFlagId` is registry-validated at flow load time.
  Unregistered ids are accepted but auto-namespaced under
  `anon.<blake3-prefix>` and warned (per D4's pattern for
  unregistered rules), so a typo doesn't silently mint a new flag.
- The `RuleErrorDiagnosis` slot value from `rule.ai-debug` reads
  the registry's descriptions/remediations as part of its context,
  so AI explanations stay coherent with pack-author intent.
- Determinism smoke (R-ins-2) covers flag emission: a derivation
  rule must emit the same flags on the same input. Non-determinism
  in flag emission is a bug.

This is the same R-ins-1 pattern (registry + extension
contribution) applied to flags instead of rules. Without it,
the taxonomy ossifies in `starter-spi` and every pack invents its
own ad-hoc strings, defeating the explainer agent and the
frontend.

## Materialisation and read paths

The frontend cannot compute a 24-hour weather-normalised baseline
from raw verdicts on every page load. Insights makes the
materialisation contract explicit so this never accidentally
happens.

**Three persisted tiers, all on the existing `insights` feature of
the store crates:**

1. **Verdict log.** Every emitted `Verdict` is appended.
   `(rule_id, at)` indexed. Bounded retention; default 90 days,
   per-rule overrideable. This is the source of truth for audits
   and for the explainer agent.
2. **Verdict rollups.** A `rollup.{hour, day, week}` node kind
   reads the verdict log on a scheduled trigger and writes
   aggregates (count by severity, p50/p95 of measured values, joined
   coverage, tag-grouped) to a separate `verdict_rollup` table.
   Rollups are themselves rules (`rule.derive` returning a
   `Dataset`); they get `RuleId`s and ride the same backfill/replay
   machinery. **Rollups are incremental by default** — each tick
   reads only verdicts since the last rollup checkpoint
   (`(rule_id, last_at)` stored alongside the table) and merges
   into the existing aggregate. A full rebuild is admin-only
   (`rollup.rebuild` call) and budgeted off the D3 batched path.
   A schema change to a rollup is a `RuleId` major bump, which
   forces a rebuild; nothing implicit. Tag-grouped aggregates use
   the indexed tag namespace from R-ins-8.
3. **Derivation cache.** A derivation rule's output `Dataset` may
   declare `persist: true` in its `RuleSchema`. When set, the engine
   writes the dataset to a per-rule table keyed on `(rule_id, window)`
   on every successful invocation, and downstream nodes (including
   the next pipeline run, and the frontend) read from that table
   instead of re-deriving. The cache is invalidated by rule version
   bump or by an explicit `cache.invalidate` admin call. A derivation
   rule with `persist: true` whose body is non-deterministic is a
   bug — caught by the same backfill determinism smoke as R-ins-2.

**Frontend read contract:**

- Frontends **never invoke rules** to render a page. They read from
  the verdict log (recent N), verdict rollups (timeseries panels),
  and the derivation cache (charts that need cleaned/normalised
  data). All three are tables; all three are accessed via
  `starter-server` REST endpoints whose query shape mirrors the
  table schema. The handler is a thin SELECT, not a rule
  invocation.
- A page that *would* need a fresh rule invocation is a page that
  needs a button, not a render. The button posts to a flow run; the
  run emits verdicts; the user refreshes (or subscribes via SSE).
  This keeps the read path predictably fast.

**Interaction with the backfill cap (D3):** the 100k-row cap on
`RuleRunStore::backfill` applies to **ad-hoc** backfills (e.g. the
rule-author agent's dry-run). Scheduled rollup jobs use a separate,
batched path with no per-invocation cap because they run on a
budgeted schedule and their cost is bounded by the schedule. If the
rollup table is absent or stale, a frontend page load is **not**
allowed to trigger a backfill — the page renders with the data it
has (with a `stale_since` marker) and the rollup catches up on its
next scheduled tick. **Page loads never block on rules.**

**Cache warming and onboarding.** When a pipeline is deployed for
the first time (or a new building/tenant/device is added to an
existing pipeline), the rollup and derivation-cache tables are
empty. Insights handles this explicitly rather than implicitly:

- On first-deploy of a pipeline, the engine triggers a **bounded
  onboarding backfill**, capped at D3's 100k rows per derivation
  rule, across the configured initial window (default 30 days,
  per-pipeline overrideable). Rollups feed off the resulting
  verdict log entries on the next scheduled tick.
- If the onboarding window exceeds the cap, the partial result is
  marked with a `starter.quality.partial-onboarding` flag on each
  affected verdict and a `BackfillTruncated` event on the run
  stream; the tuner agent reads it and proposes a narrower window
  or a paged rebuild via `rollup.rebuild`.
- The frontend reads `stale_since` and `partial-onboarding` as
  advisory markers on responses, never as a blocking error. A
  partially-warm cache renders with banners, not 500s.
- Rule version bumps (`RuleId` major change) invalidate the
  derivation cache for that rule only; **nothing auto-rewarms**.
  The next scheduled tick repopulates it, or an operator triggers
  `rollup.rebuild` for the affected pipelines. This is deliberate:
  auto-rewarm on a busy host can stampede the store.
- **Page loads never trigger backfills.** This invariant is the
  load-bearing one: a deployed pipeline with a cold cache yields
  banners + sparse data on the page, and warms in the background.
  The alternative (page-load-triggered backfill) means the first
  user after a deploy waits 30 minutes for a render, which is the
  exact failure mode this whole section exists to prevent.

## Use-case fit (sanity check the design)

| Use case | Trigger | Window | Reusable derivations | Reusable assertions | AI in-line | Custom rule | Tags | Action (via flow branch) |
|---|---|---|---|---|---|---|---|---|
| **Finance — anomalous transactions** | event (webhook) | sliding 5m | — | `finance.tx.duplicate@1`, `finance.tx.outside-region@1` | `org.acme.fraud-judge@1` (`rule.ai-check`) | `org.acme.large-refund@1` (`rule.rhai`) | `domain:finance`, `tenant:<id>`, `pii` | `notify` → Slack; `Error` → `ai-debug` → ops |
| **IoT — device health** | schedule 30s | tumbling 1m | `iot.sensor.despike@1` | `iot.device.online@1`, `iot.sensor.has-recent-data@1`, `iot.sensor.in-range@1` | — | `org.acme.fleet-quorum@1` (`rule.rhai`) | `domain:iot`, `site:<id>`, `critical` | extension `tool-call`; `Error` → `ai-debug` → ops |
| **Energy/water — baseline deviation** | schedule 1h | sliding 24h vs prev-week | `energy.meter.fill-gaps@2`, `weather.resample.15m-to-1m@1`, `energy.normalise.weather@2` | `energy.usage.baseline-deviation@1`, `energy.peak.detect@1` | `org.acme.energy-judge@1` (`rule.ai-check`) | `org.acme.tariff-window@1` (`rule.sql`) | `domain:energy`, `building:<id>`, `cost-centre:<x>` | explainer → email gated by `coverage>=0.8`; `Error` → `ai-debug` |
| **HVAC — comfort vs cost** | reactive (slot change) | sliding 15m | `hvac.sensor.despike@1`, `hvac.occupancy.fill-forward@1` | `hvac.pmv.comfort@1`, `hvac.setpoint.drift@1`, `hvac.short-cycle@1` | `org.acme.hvac-judge@1` (`rule.ai-check`) | `org.acme.tenant-policy@1` (`rule.rhai`) | `domain:hvac`, `building:<id>`, `tenant:<id>` | `tool-call` → BMS write, gated by approval; `Error` → `ai-debug` |
| **Building bills reconciliation** (the ugly one) | schedule 1h | sliding 30d, tz-aware | `align(meter+weather+tariff+occupancy)`, `energy.meter.despike@2`, `energy.meter.fill-gaps@3`, `weather.resample.15m-to-1m@1`, `energy.normalise.weather@2`, `energy.tariff.apply-retroactive@1` | `energy.baseline.deviation@1`, `energy.peak.detect@1`, `energy.bill.reconcile@1` | `org.acme.bill-judge@1` (`rule.ai-check`) | `org.acme.tariff-window-overrun@1` (`rule.rhai`) | `domain:energy`, `building:<id>`, `bills`, `cost-centre:facilities` | explainer → PDF report; daily rollup keyed by `building` tag; chart reads cache; `Error` → `ai-debug` |

All five pipelines share the same shape: trigger → (optional) align
→ derivation chain → assertion fan-in → `verdict.join` → optional
`rule.ai-check` → `branch(on=severity)` → {gate → action} or {
`rule.ai-debug` → ops}. The operator's authoring work is *one*
custom rule per pipeline; everything else is reused. Domain
correctness is owned by the rule pack maintainers. Coverage
propagates through every derivation step, and the gate uses it to
suppress low-confidence alerts. Chaining and error routing are
**flow-engine** mechanics (R-ins-9), not insights mechanics. Tags
flow through every step and drive routing, rollup grouping, and
frontend filtering (R-ins-8). The bills-reconciliation row is the
explicit stress test of R-ins-2 + R-ins-7 + R-ins-10: ugly,
multi-source, gappy, retroactive, long-window, with an AI judge
in-line and an AI debugger on the error path, and still expressible
without the operator authoring anything but their tariff-window
check. **That is the test of whether R-ins-2 + R-ins-7 + R-ins-10
are real.**

## Non-goals

- **A general-purpose stream processor.** Insights windows are bounded
  (max size and max age configurable per `window.*` node). For
  unbounded streaming, use a dedicated system upstream and feed
  verdicts in via a `rule.sql` against the result table.
- **A dashboarding system.** Verdicts are persisted; rendering belongs
  to the frontend SCOPE. `starter-ui-*` may ship a verdict-list panel,
  but it is consumer-owned UI, not part of the capability.
- **A replacement for monitoring systems** (Prometheus, OpenSearch,
  Datadog). Insights complements them: scrape metrics in via existing
  tools; emit verdicts out via existing notification tools. The
  capability is the **rule library and the composition**, not the
  metric pipeline.
- **A workflow language.** Pipelines are flows. Authoring is
  YAML/visual per the flow SCOPE. Insights adds node kinds, not a
  parallel DSL.
- **A "Windmill clone".** What we lift from Windmill: scripts as
  first-class composable units with typed I/O and a job history —
  which the flow engine already provides. What we **do not** lift: a
  parallel worker pool, parallel auth, parallel UI, the polyglot
  runtime, the workspaces concept. Importing Windmill's surface area
  would violate `starter`'s root SCOPE R0.

## Open decisions (D-list)

- **D1 — Contracts placement. DECIDED.** `Rule`, `Verdict`,
  `Coverage` (incl. `RawCoverage` / `EffectiveCoverage`), `Dataset`,
  `RuleId`, `RuleSchema`, `RuleError`, `Severity`, `QualityFlag`,
  `QualityFlagId`, `Tags`, `Window`, `TimeZoneId` live **in
  `starter-spi`** (not re-exported — defined there). This is
  load-bearing because Phase 1 rule packs ship as extensions that
  cannot depend on `starter-insights` (it would invert the dep
  arrow). They depend on `starter-spi` only, exactly as `Tool`
  impls do today.

  **`Dataset` dep-arrow honesty.** `Dataset::rows` is
  `Arc<dyn DatasetRows>` (trait object) to keep the `starter-spi`
  type light. `starter-spi` ships **one** concrete impl —
  `VecDatasetRows` — suitable for assertion packs that return tiny
  evidence rows and small-dataset derivation packs (rough cap:
  ~10k rows / ~1MB). Packs that need to stream larger datasets
  depend on `starter-insights`, which ships
  `StreamingDatasetRows` (chunked, backed by the store or an
  in-memory ring). D1 makes this explicit so nobody thinks "depend
  on `starter-spi` only" is universal:

  | Pack returns | Sufficient dep |
  |---|---|
  | `Verdict` only | `starter-spi` |
  | Small `Dataset` (< ~10k rows) | `starter-spi` (`VecDatasetRows`) |
  | Large/streamed `Dataset` | `starter-insights` |

  **Revisit trigger:** a field legitimately requires a heavy dep
  (e.g. arrow buffers in a `Dataset` variant) — at which point the
  heavy bit moves behind a trait object so the `starter-spi`-level
  type stays light. The `DatasetRows` pattern is the template;
  honour it for any future field.
- **D2 — `rule.sql` datasource seam.** Phase 1 runs `rule.sql` against
  the host's primary store only (cheap, narrow, no new SPI trait).
  Phase 2 introduces `SqlSource` for read-only attached datasources
  if a consumer surfaces the need. The Phase 1 shape ships first;
  Phase 2 is opt-in and additive.
- **D3 — Backfill cost cap.** `RuleRunStore::backfill` over long
  history is the dominant CPU cost in the capability. Phase 1 hard-
  caps at 100k rows per backfill invocation and surfaces a
  `BackfillTruncated` event on the run stream. The tuner agent reads
  this event and proposes a narrower window. **Revisit trigger:** a
  consumer needs unbounded backfill — at which point a dedicated
  `backfill.batched` node kind is added, not a config dial on the
  existing seam.
- **D4 — Inline-script `RuleId` policy.** Inline (unregistered)
  `rule.rhai`/`rule.sql` nodes get a content-hash-derived anonymous
  id (`anon.<blake3-prefix>`). They are addressable in run logs but
  excluded from `RuleRegistry::list()`. The rule-author agent
  proposes promotion. Confirmed; revisit only if anonymous ids leak
  into a public surface that needs stable identity.
- **D5 — Retroactive correction semantics.** Rollups are incremental
  by default with a monotonic `(rule_id, last_at)` checkpoint
  (Materialisation tier 2). That checkpoint is correct for
  append-only verdict history but **wrong when underlying data
  mutates** — a tariff fixup or a meter re-export silently leaves
  pre-fixup aggregates in `verdict_rollup`. The bills row exhibits
  this every billing cycle; ignoring it means the operator UI
  displays known-stale totals.

  **Decision.** A derivation rule whose inputs may be mutated
  declares `retroactive: true` in its `RuleSchema`. The engine then:

  1. Emits a `starter.quality.retroactive-correction@1` flag on
     every `Verdict` produced from a window that overlaps a mutated
     input (detected via the source's `mutated_at` watermark; sources
     that don't expose one are treated as immutable).
  2. Invalidates the affected rollup rows by `(rule_id, window)` —
     not the whole table — and re-enqueues those windows on the next
     scheduled tick. The rollup checkpoint becomes a **per-window
     watermark**, not a single `last_at`. Storage: a
     `rollup_invalidation` table, `(rule_id, window_start,
     window_end, reason)`, drained by the scheduled rollup job.
  3. Frontend reads see the stale row marked `stale_since` until the
     next tick rewrites it. Page-load behaviour is unchanged
     (R-ins-9 + materialisation contract): no synchronous rebuild.

  Non-retroactive rules (most IoT, all `rule.ai-check`) keep the
  monotonic `last_at`. The schema flag is the seam; everything else
  is mechanical. **Revisit trigger:** a consumer needs *streaming*
  late-arriving correction (per-row, not per-window). That belongs
  in a stream processor upstream of insights, per the R-ins-7
  non-goal at line 762.
- **D6 — Iteration / fixed-point cleaning.** Iteration is out of
  scope for insights as a node kind (line 749) — the engine, not
  insights, would own it. But the doc doesn't tell operators *how*
  to hand-unroll cleaning safely, and "two-pass despike-then-fill"
  is the common case for energy and IoT.

  **Decision.** Hand-unrolled multi-pass cleaning is the documented
  pattern. Three rules of thumb, enforced by lint and smoke:

  1. **Idempotence is the contract.** A derivation rule used in a
     multi-pass chain must be idempotent on its own output: running
     it twice in succession on the same dataset must produce a
     dataset equal to running it once (modulo the audit trail in
     `penalty_chain`, which strictly grows). The R-ins-2 backfill
     determinism smoke is extended with an `idempotent: true`
     marker on `RuleSchema`; the smoke runs the rule twice and
     compares output.
  2. **Coverage and `samples_present` only decrease.** Hand-unrolled
     chains must not inflate either across passes. This is already
     enforced by the raw/effective split (line 494); the lint just
     refuses a chain whose declared `confidence_penalty`s sum
     positive.
  3. **Bound the unroll at three.** A pipeline with more than three
     copies of the same `RuleId` is a lint error. If you genuinely
     need N-pass, write a single `rule.rust` whose body iterates
     internally with its own bounded inner loop — one rule, one
     `RuleId`, one audit row.

  **Revisit trigger:** the flow engine ships
  `until_stable(max_iters=N)` as a generic node. Insights consumes
  it unchanged; this D entry is then superseded.
- **D7 — Node-kind extensibility (`align`, `window.*`, `rollup.*`,
  `verdict.join`).** Today insights ships these as built-ins.
  Domain packs contribute *rule impls* via
  `contributes.tools`/`register()` (R-ins-1) but have **no path** to
  contribute a new node kind. A finance pack wanting a custom join
  semantic (e.g. trade-leg matching), or an HVAC pack wanting a
  domain-specific alignment (degree-day frames), is stuck.

  **Decision.** Extension packs may contribute new node kinds into
  `starter-flow-nodes`'s `NodeKindRegistry` via the existing
  extension mechanism, subject to **three constraints**:

  1. The new node must implement `starter-flow-spi::NodeBehavior`
     and return one of: `Verdict`, `Dataset`, or `Frame`. No new
     slot value types from packs — `Frame` stays an internal,
     stable shape owned by insights (line 728).
  2. The node id is namespaced (`acme.finance.match-legs@1`) and
     carries the same audit weight as a `RuleId`. Run logs and
     `Verdict.evidence` provenance treat pack-contributed nodes
     identically to insights' built-ins.
  3. A pack contributing a node kind **may not** ship a parallel
     orchestrator or runtime; R-ins-9 binds packs too. The CI
     dep-tree gate (R-ins-5) extends to pack node kinds.

  Concretely: `align`, `window.tumble`, `window.slide`,
  `verdict.join`, and the `rollup.*` family are insights built-ins
  because they're load-bearing for every domain; packs add variants
  alongside them, not replacements. **Revisit trigger:** two packs
  ship competing `align`-like nodes with overlapping config — the
  fix is to lift the common bits into a shared trait in
  `starter-insights`, not to ban competition.
- **D8 — `align` boundary: rule-kind vs flow-node.** D7 leaves
  `align` as a flow-node, not a rule. The doc admits it is "the
  most domain-loaded node" (line 728) and gives it a `NodeId` to
  patch the audit gap. The question: should `align` *be* a rule
  (`rule.align`), so packs can ship domain alignments via the rule
  registry?

  **Decision.** `align` stays a flow-node. Reasons:

  1. Its output is a `Frame` (an ordered tuple of co-time-indexed
     datasets), not a `Verdict` or single `Dataset`. Forcing it
     through the `Rule` trait either widens `RuleOutput` (bad — the
     two-shape rule is load-bearing for R-ins-7) or hides multi-
     dataset output behind a synthetic single-dataset shape (worse
     — destroys the audit story).
  2. Multi-input fan-in is a topology concern. `align` taking N
     inputs is naturally a node-with-N-inputs; the `Rule` trait is
     `Dataset → Dataset` / `Dataset → Verdict`, single-input by
     design.
  3. Domain alignments compose by being **alternative node kinds**
     under D7's extensibility seam, not by being rules. A pack
     wanting degree-day alignment ships
     `acme.hvac.align-degree-day@1` as a node kind alongside
     `align`, not as a `rule.align` impl.

  **Revisit trigger:** a third pack ships a third `align`-shaped
  node and the config surface is 80% shared. At that point, lift a
  shared `Aligner` trait into `starter-insights` and let the three
  nodes wrap impls of it — still flow-nodes, not rules.
- **D9 — Materialisation SLOs.** "Predictably fast" (line 1128) is
  not regression-testable. Without numbers, the read-path contract
  can drift silently and the 30-minute-render footgun returns.

  **Decision.** Insights publishes p95 latency targets per read-path
  class, enforced by a smoke in CI that runs the IoT + Energy +
  HVAC reference pipelines against a synthetic 90-day dataset:

  | Read path | p95 target | Source table |
  |---|---|---|
  | Verdict list (recent N, single rule) | 50 ms | `verdict_log` |
  | Verdict list (filtered by tag) | 150 ms | `verdict_log` + tag index |
  | Rollup timeseries (1 rule, 1 window class, 90 days) | 100 ms | `verdict_rollup` |
  | Rollup timeseries (tag-grouped, 90 days) | 250 ms | `verdict_rollup` + tag index |
  | Derivation cache fetch (1 rule, 1 window) | 50 ms | per-rule cache table |
  | Onboarding cold page (no rollup yet) | 100 ms with `partial-onboarding` banner | empty / sparse |

  Numbers are budgets, not promises. Three rules:

  1. The smoke runs on the CI worker profile, not on a developer
     laptop. Consumer hardware is not in scope.
  2. A regression above target fails CI with the offending query
     plan attached. The fix is to tighten the query, the index, or
     the rollup schema — never to relax the budget.
  3. Targets are revised on a `RuleId`-major-bump cadence per
     rollup, not silently. A new rollup whose shape cannot hit the
     target is rejected at review.

  **Revisit trigger:** a consumer ships on hardware materially
  different from the CI profile (e.g. an edge appliance) and needs
  a separate budget. Add a second column, not a second smoke.

## Phasing

- **Phase 1.** `starter-insights` crate skeleton, `Rule` +
  `Verdict` + `Severity` (incl. `Error`) + `Coverage` (raw +
  effective) + `Dataset` (+ `VecDatasetRows`) + `RuleOutput` +
  `Tags` + `QualityFlag`/`QualityFlagId` (R-ins-11) in `starter-spi`
  (per D1), `rule.rust` + `verdict.join` node kinds (incl. all-Error
  degenerate case), `RuleRegistry` + `QualityFlagRegistry`, sqlite
  persistence behind the `insights` feature (verdict log + tag
  index only — no rollups, no derivation cache yet). **R-ins-9
  invariant:** rule chaining and error routing use existing flow
  engine nodes (`branch`, `gate`, `retry`); insights ships node
  bodies only. One extension pack (`starter-ext-insights-iot`)
  with three rules contributing as nodes per R-ins-1:
  `device.online@1`, `sensor.has-recent-data@1`,
  `sensor.in-range@1`, plus the `iot.quality.*` flags it needs.
  **Phase 1 pipelines are point-in-time** — no windowing nodes
  ship yet, so the IoT smoke evaluates each rule on the latest
  sample only. No AI rule kinds yet; pipelines run
  pure-deterministic. Smoke test reproduces the IoT row, modulo
  windowing and AI, including the `Severity::Error` → `branch` →
  `notify(ops)` path with a forced rule failure.
- **Phase 2.** `rule.rhai` + the locked sandbox (R-ins-4),
  `window.tumble` + `window.slide` (with `tz` config), `rule.sql`
  against the host store (D2 Phase 1 shape). Backfill with the D3
  cap. Verdict rollups (tier 2 materialisation), **incremental
  by default**, with tag-grouped aggregates per R-ins-8.
  `confidence_penalty` enforcement in the engine for derivation
  rules. Second extension pack (`starter-ext-insights-energy`)
  with derivation rules and `energy.quality.*` flags. Reproduces
  the Energy row.
- **Phase 3.** `rule.derive` + `align` node (with `NodeId` audit
  identity) + derivation cache (tier 3 materialisation) +
  `StreamingDatasetRows` impl. **`rule.ai-check` + `rule.ai-debug`
  (R-ins-10)** alongside the three skill bundles + agent
  integration helpers in `starter-insights` (feature-gated on
  `ai-agent`). Model-family pinning + per-Verdict exact-model
  evidence. CI dep-tree gate added per R-ins-5. Auto-tagging
  (`starter.ai-check`, `starter.ai-debug`, `starter.ai-model:*`)
  wired in. Onboarding-backfill machinery (the cache-warming
  contract). Reproduces the HVAC row and the bills-reconciliation
  row end-to-end (custom Rhai rule + derivation chain + AI judge
  + explainer agent + coverage-gated notification + AI debugger
  on the error edge).
- **Phase 4.** Finance pack. Performance pass on `verdict.join`
  and derivation cache. Operator UI panel in `starter-ui-*`
  (consumer-owned, not part of this SCOPE) — reads verdict log +
  rollups + derivation cache, filters by tag.

## See also

- [DOCS/flow/scope/SCOPE.md](../flow/scope/SCOPE.md) — the engine
  insights pipelines run on. R1 (Everything is a Node), R3 (engine is
  reader of policies), R8/R9 (FlowAsTool / FlowAsService) all apply
  transitively.
- [DOCS/agent/SCOPE.md](../agent/SCOPE.md) — the `ai-agent` node kind
  insights agents are. R2 (`AiRunner` is the only LLM seam) and R4
  (skills are static metadata, quarantined by default) apply to the
  three insights skill bundles unchanged.
- [DOCS/extensions/scope/SCOPE.md](../extensions/scope/SCOPE.md) —
  the substrate domain rule packs ship on. R1 (one trait, three
  flavours) means the same rule pack can ship builtin, WASM, or
  child-process without code change.
- [DOCS/tools/scope/SCOPE.md](../tools/scope/SCOPE.md) — the `Tool`
  trait downstream actions (`action.notify`, `tool-call`) implement.
