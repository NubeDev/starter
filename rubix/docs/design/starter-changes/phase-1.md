# Starter changes — Phase 1 gates

Phase 1 of rubix consumes a number of starter capabilities that
either didn't exist or were private at the time the rubix thin
slice started. Each item below is either already landed in-tree
(during the load-bearing Phase 1 design work) or is planned for the
upstream PR window once Phase 1 broadening begins.

See [README.md](./README.md) for the index and per-item format.

## `starter-i18n` — public param-aware render API

- **Crate:** `starter-i18n` (extend)
- **Blocks rubix phase:** 1 (the i18n + prefs end-to-end demo, see
  [docs/design/i18n-prefs/](../i18n-prefs/README.md))
- **Why upstream:** the `interpolate()` function that applies
  `{name}` substitutions is **private** to the HTTP
  `diagnostics_layer` (`crates/starter-i18n/src/diagnostics.rs`).
  The MCP transport renders server-side (the documented R5
  exception); the CLI renders at the call site. Neither path runs
  through the HTTP middleware, so neither can reach `interpolate`
  today.
- **Proposed shape:**
  ```rust
  impl MessageBundle {
      /// Lookup + interpolate. Returns the localised string, or
      /// the fallback-language equivalent, or the key as-is.
      pub fn render(
          &self,
          lang: &LanguageTag,
          key: &MessageKey,
          params: &BTreeMap<String, DiagnosticParam>,
      ) -> String;
  }
  ```
- **Status:** **landed (in-tree)** — `crates/starter-i18n/src/bundle.rs::MessageBundle::render` + shared `crates/starter-i18n/src/interpolate.rs`.
- **Notes:** purely additive. Existing `lookup` / `render_or_key`
  stay. The legacy `diagnostics_layer` JSON path is unchanged; the
  new typed path lives in `interpolate.rs::interpolate_typed`.

## `starter-spi::i18n::DiagnosticParam::Quantity` variant

- **Crate:** `starter-spi` (extend `i18n/diagnostic.rs`)
- **Blocks rubix phase:** 1
- **Why upstream:** rubix tool outputs carry `Quantity`-typed
  params (disk usage in GB, throughput in MB/s, temperature in
  °C/°F). Today `DiagnosticParam` is `String | I64 | F64 | Bool |
  Timestamp` — none of which carry the unit. Without a `Quantity`
  variant, every tool that returns a measured value has to
  pre-format strings, which defeats the whole locale-at-the-edge
  story.
- **Proposed shape:**
  ```rust
  pub enum DiagnosticParam {
      String(String),
      I64(i64),
      F64(f64),
      Bool(bool),
      Timestamp(i64),
      /// Canonical SI value plus the quantity dimension. The
      /// renderer consults ResolvedPreferences to pick the
      /// caller's target unit and to format.
      Quantity { canonical: f64, quantity: Quantity },
  }
  ```
- **Status:** **landed (in-tree)** — `crates/starter-spi/src/i18n/diagnostic.rs::DiagnosticParam::Quantity`, gated on the `units` feature.
- **Notes:** additive on the JSON wire. The variant is feature-
  gated so consumers that don't enable `starter-spi/units` see the
  pre-existing five variants exactly as before — no breaking
  change to their build.

## `MessageBundle::render_diagnostic` — one-call renderer

- **Crate:** `starter-i18n` (extend)
- **Blocks rubix phase:** 1
- **Why upstream:** combines the two items above. Given a
  `Diagnostic` and a `ResolvedPreferences`, produce the final
  string. MCP, CLI, server-side-render error paths, and any
  future consumer all want this — without it every transport
  reimplements the same loop.
- **Proposed shape:**
  ```rust
  impl MessageBundle {
      pub fn render_diagnostic(
          &self,
          lang: &LanguageTag,
          diag: &Diagnostic,
          prefs: &ResolvedPreferences,
      ) -> String;
  }
  ```
- **Status:** **landed (in-tree)** — `MessageBundle::render_diagnostic` on `crates/starter-i18n/src/bundle.rs`, gated on the new `preferences` feature (implies `units`).
- **Notes:** depends on the two items above. Keeps the `Quantity`
  conversion in one place (the renderer); domain code never
  formats.

## `ResolvedPreferences::language_tag()` — type bridge

- **Crate:** `starter-spi` (extend `preferences/resolved.rs`)
- **Blocks rubix phase:** 1 (nice-to-have, not blocking)
- **Why upstream:** `ResolvedPreferences.language: String` vs
  `LanguageTag(String)` in `spi/i18n`. Every consumer hand-rolls
  the conversion.
- **Proposed shape:**
  ```rust
  impl ResolvedPreferences {
      pub fn language_tag(&self) -> LanguageTag { /* ... */ }
  }
  ```
- **Status:** **landed (in-tree)** — `crates/starter-spi/src/preferences/resolved.rs::ResolvedPreferences::language_tag`, gated on the `i18n` feature.

## `MessageBundle::render_diagnostic` — timezone-aware `Timestamp`

- **Crate:** `starter-i18n` (extend the existing renderer)
- **Blocks rubix phase:** 1 (same i18n + prefs demo)
- **Why upstream:** without this, `render_diagnostic` writes raw
  epoch ms (`1764892800000`) into a tool result instead of
  rendering for the caller's timezone + date/time format. MCP +
  CLI need the formatted output; today's behaviour is hostile.
- **Proposed shape:** extend `write_param_with_prefs` so the
  `DiagnosticParam::Timestamp(ms)` arm consults
  `prefs.timezone` (IANA), `prefs.date_format`, and
  `prefs.time_format` and writes e.g. `15/01/2024, 13:00`
  (EU operator, 24h) or `01/15/2024, 7:00 AM` (US, 12h).
  Conversion failures fall through to the canonical UTC RFC 3339
  rendering so the operator still sees something readable.
- **Status:** **landed (in-tree)** — `crates/starter-i18n/src/interpolate.rs::write_timestamp_with_prefs`, gated on the new `chrono` + `chrono-tz` deps inside the existing `preferences` feature. Round-trip test
  (`timestamp_renders_in_caller_timezone_and_format`) asserts
  both EU (`Europe/Paris`, DD/MM/YYYY, 24h) and US
  (`America/New_York`, MM/DD/YYYY, 12h) renderings of the same
  epoch.

## `starter-tool-sysdiag` — disk / db-size / flow-errors tools

- **Crate:** `starter-tool-sysdiag` (new; matches existing
  `starter-tool-*` pattern)
- **Blocks rubix phase:** 1 (Goal 5)
- **Why upstream:** every starter consumer with an operator
  presence needs these.
- **Status:** planned
- **Notes:** rubix consumes; rubix does not maintain a parallel
  copy. Without this, rubix-tools would carry it forever — exactly
  what R2 forbids.

## Recorded-LLM-response harness

- **Crate:** `starter-server::testing` (extension) or new
  `starter-ai-record`
- **Blocks rubix phase:** 1 (R10)
- **Why upstream:** every consumer testing an agent loop needs
  this. Per-PR live LLM calls in CI are unaffordable; no consumer
  should have to invent this.
- **Status:** planned
- **Notes:** rubix Phase 1 is the first real consumer; the API
  shape is sharpened here.

## `starter-ai-agent` — runner-agnostic `AgentLoop` primitive

- **Crate:** `starter-ai-agent` (new)
- **Blocks rubix phase:** 1
- **Why upstream:** the loop is the same shape for every consumer.
  Decoupling it from any flow-engine concern means CLI / REST / MCP
  callers reuse the exact same primitive that the `ai-agent` node
  wraps.
- **Status:** **landed (in-tree)** — see
  `crates/starter-ai-agent/`. Five long-term concerns (multi-turn
  session persistence, cost cap, cooperative cancellation, tool-call
  streaming, skill enforcement) are scoped in
  `crates/starter-ai-agent/LONG-TERM.md`.

## `starter-flow-node-loop` — the `ai-agent` node body

- **Crate:** `starter-flow-node-loop` (new)
- **Blocks rubix phase:** 1
- **Why upstream:** every starter consumer that wants an LLM loop
  needs this; codeless already wrote one, rubix would write a
  third. See starter `DOCS/agent/SCOPE.md` D1.
- **Status:** **landed (in-tree)** — see
  `crates/starter-flow-node-loop/`. Thin `NodeBehavior` wrapper
  around `starter-ai-agent::AgentLoop`; registers under
  `KIND_ID = "com.starter.ai-agent"`.
- **Notes:** starter `DOCS/agent/SCOPE.md` D1 left the choice
  between `-loop` and `-adk` open; rubix picked `-loop` as the
  simpler surface. The crate name is the only spot where the
  decision is visible.

## `starter-skills` — SKILL.md parser + registry + content-hash quarantine

- **Crate:** `starter-skills` (new)
- **Blocks rubix phase:** 1
- **Why upstream:** any consumer with a `Tool` caller benefits;
  quarantine is a security primitive, not rubix-specific.
- **Status:** planned (specified in starter `DOCS/agent/SKILLS.md`)
- **Notes:** rubix-skills depends entirely on this. No fallback
  acceptable — rubix would not implement its own skill parser.

## MCP prompts + resources surfaces in `starter-mcp`

- **Crate:** `starter-mcp`
- **Blocks rubix phase:** 1 (R12)
- **Why upstream:** every MCP consumer wants the three surfaces;
  shipping tools-only is a UX miss for the whole ecosystem.
- **Status:** issue-to-file (verify whether `starter-mcp` already
  exposes prompts/resources before opening)
- **Notes:** rubix Phase 1 ships one prompt + one resource for the
  system-check goal as the first real consumer.

## Typed agent event taxonomy in `starter-flow`

- **Crate:** `starter-flow` (+ `starter-server` SSE helpers)
- **Blocks rubix phase:** 1 (R13)
- **Why upstream:** every flow consumer benefits from a typed
  event stream; without one, every UI client invents its own.
- **Status:** planned
- **Notes:** event list lives in rubix's R13 for now; promote to
  starter as the canonical schema. `MessageKey`-typed text fields
  are non-negotiable (anti-i18n-rot).
