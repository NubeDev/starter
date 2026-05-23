# Starter changes — the upstream PR list

This doc is how rubix's [R2](../../SCOPE.md#r2--upstream-first-rubix-specific-stays-in-rubix-reusable-goes-to-starter)
("upstream first") becomes a deliverable, not a slogan. Every
starter capability rubix needs that doesn't yet exist is listed
here, ordered by which rubix phase blocks on it.

## How this doc is used

- **Before a phase starts:** check the items gated on that phase.
  Each item that isn't merged yet must have a draft PR or a filed
  issue with rationale.
- **During a phase:** when rubix code starts to look like a
  re-implementation of a starter capability, file the upstream
  issue *first*, link it here, then either wait for review or ship
  a temporary rubix impl with the issue link in a `TODO(upstream:
  <issue>)` comment.
- **At phase exit:** the phase's section below lists every upstream
  PR filed during the phase (merged, in review, or filed-with-
  rationale). A phase with zero PRs is a smell — the reviewer asks
  "what didn't get upstreamed and why?"

This doc lives in `rubix/` because it is rubix's planning artifact.
The actual code changes ship in starter. Linking is by PR / issue
URL.

## Format for each item

```
### <short title>
- **Crate(s):** starter-foo, starter-bar
- **Blocks rubix phase:** N
- **Why upstream:** one sentence on who else benefits
- **Status:** planned | issue-filed (#NNN) | pr-open (#NNN) | merged (vX.Y.Z)
- **Notes:** any rationale, alternatives considered, or rubix
  fallback if the PR slips
```

## Items by phase

### Phase 1 (gates)

#### `starter-i18n` — public param-aware render API

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

#### `starter-spi::i18n::DiagnosticParam::Quantity` variant

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

#### `MessageBundle::render_diagnostic` — one-call renderer

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

#### `ResolvedPreferences::language_tag()` — type bridge

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

#### `MessageBundle::render_diagnostic` — timezone-aware `Timestamp`

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

#### `starter-tool-sysdiag` — disk / db-size / flow-errors tools

- **Crate:** `starter-tool-sysdiag` (new; matches existing
  `starter-tool-*` pattern)
- **Blocks rubix phase:** 1 (Goal 5)
- **Why upstream:** every starter consumer with an operator
  presence needs these.
- **Status:** planned
- **Notes:** rubix consumes; rubix does not maintain a parallel
  copy. Without this, rubix-tools would carry it forever — exactly
  what R2 forbids.

#### Recorded-LLM-response harness

- **Crate:** `starter-server::testing` (extension) or new
  `starter-ai-record`
- **Blocks rubix phase:** 1 (R10)
- **Why upstream:** every consumer testing an agent loop needs
  this. Per-PR live LLM calls in CI are unaffordable; no consumer
  should have to invent this.
- **Status:** planned
- **Notes:** rubix Phase 1 is the first real consumer; the API
  shape is sharpened here.

#### `starter-flow-node-loop` — the `ai-agent` node body

- **Crate:** `starter-flow-node-loop` (new)
- **Blocks rubix phase:** 1
- **Why upstream:** every starter consumer that wants an LLM loop
  needs this; codeless already wrote one, rubix would write a
  third. See starter `DOCS/agent/SCOPE.md` D1.
- **Status:** planned
- **Notes:** starter `DOCS/agent/SCOPE.md` D1 leaves the choice
  between `-loop` and `-adk` open; rubix has no preference beyond
  "the simpler one". If `-adk` is picked, the rubix dep changes
  but nothing else.

#### `starter-skills` — SKILL.md parser + registry + content-hash quarantine

- **Crate:** `starter-skills` (new)
- **Blocks rubix phase:** 1
- **Why upstream:** any consumer with a `Tool` caller benefits;
  quarantine is a security primitive, not rubix-specific.
- **Status:** planned (specified in starter `DOCS/agent/SKILLS.md`)
- **Notes:** rubix-skills depends entirely on this. No fallback
  acceptable — rubix would not implement its own skill parser.

#### MCP prompts + resources surfaces in `starter-mcp`

- **Crate:** `starter-mcp`
- **Blocks rubix phase:** 1 (R12)
- **Why upstream:** every MCP consumer wants the three surfaces;
  shipping tools-only is a UX miss for the whole ecosystem.
- **Status:** issue-to-file (verify whether `starter-mcp` already
  exposes prompts/resources before opening)
- **Notes:** rubix Phase 1 ships one prompt + one resource for the
  system-check goal as the first real consumer.

#### Typed agent event taxonomy in `starter-flow`

- **Crate:** `starter-flow` (+ `starter-server` SSE helpers)
- **Blocks rubix phase:** 1 (R13)
- **Why upstream:** every flow consumer benefits from a typed
  event stream; without one, every UI client invents its own.
- **Status:** planned
- **Notes:** event list lives in rubix's R13 for now; promote to
  starter as the canonical schema. `MessageKey`-typed text fields
  are non-negotiable (anti-i18n-rot).

### Phase 2a (gates)

Any auth/authz rough edges discovered while wiring
`starter-auth-users` + `starter-auth-oauth` + `starter-authz` into
the rubix binary. Expect:

- Possible missing DTOs in `starter-spi`'s `auth` module.
- Possible missing helpers for tenant-scoped query filtering.
- Likely missing `Authenticator` impl shape for "MCP transport-
  level auth" (Claude Desktop authenticating to rubix).

(Items added here as they surface.)

#### `starter-auth-users` — Postgres store impls

**Why we need it.** rubix is Postgres-only (ADR 0001). PR 2 of the
thin slice wires `starter-auth-users` into `rubix-agent` so cookie
sessions gate the MCP tool calls. `starter-auth-users` originally
shipped only SQLite store impls (the `postgres` feature flag existed
in `Cargo.toml` but had no implementations behind it).

**Status: complete (in-tree).** All four Postgres store impls now
mirror their sqlite counterparts row-for-row, each with an `#[ignore]`'d
testcontainers test. PR 2 part 2 (rubix-side auth wiring) is fully
unblocked.

| Component | Status | Notes |
|---|---|---|
| `migrations_postgres/starter_auth_users/0001_users.sql` | ✅ landed | TEXT→TIMESTAMPTZ for `created_at`/`updated_at`; DEFAULT CURRENT_TIMESTAMP → DEFAULT NOW() |
| `migrations_postgres/starter_auth_users/0002_sessions.sql` | ✅ landed | Same timestamp translation; nullable `revoked_at` becomes TIMESTAMPTZ |
| `migrations_postgres/starter_auth_users/0003_tokens.sql` | ✅ landed | Timestamp translation + `scopes TEXT DEFAULT '[]'` → `scopes JSONB DEFAULT '[]'::jsonb` (Postgres has a real JSON type — index + query efficiently; the application still treats it as a JSON-encoded array) |
| `migrations_postgres/starter_auth_users/0004_users_email_verified.sql` | ✅ landed | `INTEGER NOT NULL DEFAULT 1` (sqlite bool) → `BOOLEAN NOT NULL DEFAULT TRUE` |
| `migrations_postgres/starter_auth_users/0005_tenants.sql` | ✅ landed | Sqlite `slug NOT GLOB '[0-9]*'` → Postgres `slug !~ '^[0-9]'` (POSIX regex). `RESERVED_SLUGS` CHECK constraint is straight string match. TIMESTAMPTZ translations as in 0001-0004. BEFORE UPDATE `RAISE(ABORT)` trigger translated to a plpgsql function `RAISE EXCEPTION ... USING ERRCODE = '23514'` + `CREATE TRIGGER ... EXECUTE FUNCTION ...` |
| `migrations_postgres/starter_auth_users/0006_teams.sql` | ✅ landed | Same trigger-translation pattern as 0005; teams `tenant_id` + `slug` immutability enforced by plpgsql function raising SQLSTATE 23514 |
| `src/migration.rs` exposing `sqlite_migration_source()` + `postgres_migration_source()` | ✅ landed | Mirrors the `starter-changelog-{sqlite,postgres}::migration_source()` pattern; both use source name `"auth_users"` |
| Refactor `src/store/tenant_store.rs` (590 lines) into `tenant_store/{mod.rs, sqlite.rs}` to fit R1 ≤ 400 lines | ✅ landed | No behavior change; all existing sqlite tests pass post-refactor |
| `PgUserStore` (mirrors `SqliteUserStore` row-for-row) | ✅ landed | Bind placeholders `?N` → `$N`; row type `sqlx::sqlite::SqliteRow` → `sqlx::postgres::PgRow`; `set_email_verified` passes a real `bool` instead of `bool as i32` |
| `tests/pg_user_store.rs` — `#[ignore]`d testcontainers test exercising every `UserStore` method against a real Postgres | ✅ landed | Uses `starter-store-postgres::testing::with_database`; the dev-dep was added to starter-auth-users' Cargo.toml |
| `PgSessionStore` (mirrors `SqliteSessionStore`) | ✅ landed | Sibling `postgres` module inside `session_store.rs`. `revoked_at` typed as nullable `TIMESTAMPTZ`; sqlx `chrono::DateTime<Utc>` carries it both ways |
| `PgTokenStore` (mirrors `SqliteTokenStore`) | ✅ landed | Sibling `postgres` module inside `token_store.rs`. `scopes` JSONB ↔ Rust `String` (JSON-encoded array) coerces at the sqlx type seam; behaviour identical to sqlite |
| `PgTenantStore` (mirrors `SqliteTenantStore`) | ✅ landed | Sibling `tenant_store/postgres.rs`. CHECK-constraint error matching uses the Postgres SQLSTATE `23514` via `e.as_database_error().and_then(\|d\| d.code())` instead of sqlite's text-match on `"CHECK constraint failed"` |
| `tests/pg_session_store.rs` / `tests/pg_token_store.rs` / `tests/pg_tenant_store.rs` | ✅ landed | All `#[ignore]`d testcontainers tests; same shape as `pg_user_store.rs`; use `with_database()` + `postgres_migration_source()` |

**Phase 2a is complete.** PR 2 part 2 (rubix-side cookie sessions + API
tokens + tenant-scoped authz wiring against `starter-auth-users`) is
fully unblocked. The next step is the rubix-side bootstrap-user
subcommand + `boot/migrations.rs` chaining described in the active
session handoff at
[docs/sessions/2026-05-23-next-steps-4.md](../../sessions/2026-05-23-next-steps-4.md).

**Tracked per R2** (upstream-first). Pattern locked, surface complete.

### Phase 2b (gates)

Any gRPC/CLI rough edges. Expect:

- Possible missing `starter-cli` building blocks for "subcommand
  per tool" auto-generation from a `ToolRegistry`.
- Possible missing `starter-grpc` helpers for streaming the R13
  event taxonomy.

### Phase 3 (gates)

#### `starter-tool-sdui` — page-builder primitives

- **Crate:** `starter-tool-sdui` (new; matches existing
  `starter-tool-github`, `starter-tool-slack` pattern)
- **Blocks rubix phase:** 3 (Goal 1 dashboards)
- **Why upstream:** any starter consumer building dashboards via
  SDUI wants this; it's not rubix-specific.
- **Status:** planned
- **Notes:** if upstream review takes too long, primitives stay in
  `rubix-tools::dashboard::sdui_primitives` with a tracking issue.
  *Never* "we'll do it later" without an issue link.

#### `starter-tool-flow-ops` — deploy / validate / lint / list

- **Crate:** `starter-tool-flow-ops` (new)
- **Blocks rubix phase:** 3 (Goal 3 flow-programmer)
- **Why upstream:** every starter consumer with `starter-flow`
  wants flow ops surfaced as tools. This is the cleanest example
  of "rubix's tools are mostly reusable."
- **Status:** planned
- **Notes:** rubix consumes; same fallback rules as above.

### Phase 4 (gates)

#### `cron-schedule` node kind in `starter-flow-nodes`

- **Crate:** `starter-flow-nodes`
- **Blocks rubix phase:** 4 (Goal 5 cron triggering)
- **Why upstream:** every flow consumer wants cron triggers.
- **Status:** planned (verify whether `starter-flow` Service
  surface already covers this before opening a new node kind)
- **Notes:** if Service surface covers it, the upstream item moves
  to "document how the Service surface implements cron" rather
  than a new node kind.

#### `starter-tool-clickhouse` — rule.write / mart.create / retention.set

- **Crate:** `starter-tool-clickhouse` (new)
- **Blocks rubix phase:** 4 (Goal 4 ClickHouse)
- **Why upstream:** any starter consumer with ClickHouse benefits.
- **Status:** planned
- **Notes:** rubix consumes; same fallback rules as Phase 3 tools.

#### `clickhouse-query` node kind in `starter-flow-nodes`

- **Crate:** `starter-flow-nodes` (optional)
- **Blocks rubix phase:** 4 (Goal 4 ClickHouse)
- **Why upstream:** any starter consumer with ClickHouse benefits.
- **Status:** conditional — only file if rubix has a flow YAML
  that calls ClickHouse directly. If all ClickHouse access stays
  inside `rubix-tools` Rust code, no upstream item needed.

### Phase 5 (gates)

#### `starter-ext-flow` — the flow/skill/tool/node adapter

- **Crate:** `starter-ext-flow` (new)
- **Blocks rubix phase:** 5
- **Why upstream:** required by starter's agent SCOPE for
  extensions to contribute `flows`, `skills`, `tools`, `nodes`. Not
  rubix-specific in any way.
- **Status:** planned (referenced in starter `DOCS/agent/SCOPE.md`
  but not yet in `starter-extensions/crates/`)
- **Notes:** rubix Phase 5 cannot ship without this. If the
  upstream PR slips, rubix Phase 5 slips — no fallback is
  acceptable (a rubix-only extension adapter would fork the
  starter extension framework).

#### Extension-author ergonomics (10-minute scaffold)

- **Crate(s):** `starter-extensions/*`, `starter-ext-sdk`,
  possibly a new `starter-ext-scaffold` CLI helper.
- **Blocks rubix phase:** 5 (ergonomic exit criterion)
- **Why upstream:** the rubix Phase 5 exit criterion is "a fresh
  extension author scaffolds a new tool/skill/flow in ≤10
  minutes." Anything that makes this fail is a starter ergonomics
  gap, not a rubix one.
- **Status:** measure first, file second. The walkthrough
  surfaces the gaps; each gap becomes an upstream issue.

## Items filed (rolling log)

When a PR or issue is opened against starter on rubix's behalf,
append it here with date + link:

```
- YYYY-MM-DD  [#NNN](https://github.com/.../pull/NNN)  short title
```

(Empty until Phase 1 starts.)
