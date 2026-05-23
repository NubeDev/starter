# Session handoff — 2026-05-23 → next session

> **Tier:** session note. Lifetime: days. Delete (or move to an
> archive) once each item below has either (a) landed in
> `docs/design/` or (b) been moved into `docs/scope/GAPS.md` with a
> clear phase. Per [HOW-TO-CODE.md §0a](../../HOW-TO-CODE.md) and
> [NEW-SESSION.md §2](../../NEW-SESSION.md), **source code must
> never reference this file**.

This note captures where Phase 0 left things and what the next
session should pick up. The prompt at the bottom is what to paste
to a fresh agent at the start of the next session.

---

## 1. Read first, in this exact order

The next session must internalise these before touching code.
**Do not skip; do not skim.**

1. [NEW-SESSION.md](../../NEW-SESSION.md) — non-negotiables, doc-tier rules, layer separation, smoke test.
2. [HOW-TO-CODE.md](../../HOW-TO-CODE.md) — the contributor entry point. Decision tree, crate map, MUST/MUST NOT.
3. [FILE-LAYOUT.md](../../FILE-LAYOUT.md) — Rule Zero in long form. Verb-per-file pattern. ≤400 lines hard, ~100 typical.
4. [SCOPE.md](../../SCOPE.md) — the thirteen rules + phases + non-goals.
5. The three load-bearing design docs already written:
   - [docs/design/agent/](../design/agent/README.md) — what the agent is.
   - [docs/design/i18n-prefs/](../design/i18n-prefs/README.md) — the four-transport translation contract.
   - [docs/design/starter-changes/](../design/starter-changes/README.md) — the upstream PR ledger.
6. [docs/scope/GAPS.md](../scope/GAPS.md) — every capability rubix has not yet wired up, severity-coded, mapped to phases.

After step 6, the next session can answer the NEW-SESSION §1
self-test:

- Which crate does my change belong in?
- Which design doc(s) describe the area I'm about to touch?
- Will my change require updating one of those design docs in the
  same PR?

If any answer is still "I don't know", ask the user before typing.

---

## 2. State of play at end of this session

### What's on disk

| Layer | Status |
|---|---|
| Workspace + 6 rubix crates (`rubix-{spi,tools,skills,flows,client,agent}`) | ✅ build green |
| Full verb-per-file layout for all 11 goals (~165 source files, ≤30 lines each) | ✅ |
| Bundled artefacts: 6 `SKILL.md`, 6 `flow.yaml`, EN + ES catalogues (2 keys to start) | ✅ |
| Integration test mirror — 30 placeholder `tests/*_test.rs` files | ✅ compile, all `#[ignore]` |
| 25 design-doc folders under `docs/design/<area>/README.md` | ✅ placeholders + 4 real docs |
| 3 ADRs seeded (`0001-postgres-only`, `0002-backend-only`, `0003-agent-is-starter-ai-agent`) | ✅ |
| Phase 0 binary: `cargo run -p rubix-agent` serves `GET /healthz` | ✅ |

### Starter changes that landed in-tree this session

All four are additive and feature-gated, so existing consumers are
unaffected. Tests pass: 67 in `starter-spi`, 6 in `starter-i18n`.

| # | Change | File | Feature gate |
|---|---|---|---|
| S1 | `MessageBundle::render(lang, key, &params)` | `crates/starter-i18n/src/bundle.rs` + new `interpolate.rs` | none (always on) |
| S2 | `DiagnosticParam::Quantity { canonical, quantity }` | `crates/starter-spi/src/i18n/diagnostic.rs` | `starter-spi/units` |
| S3 | `ResolvedPreferences::language_tag()` | `crates/starter-spi/src/preferences/resolved.rs` | `starter-spi/i18n` |
| S4 | `MessageBundle::render_diagnostic(lang, diag, prefs)` | `crates/starter-i18n/src/bundle.rs` + `interpolate_typed_with_prefs` | new `starter-i18n/preferences` |
| S5 | Timezone-aware `Timestamp` rendering in `render_diagnostic` (chrono + chrono-tz, date_format + time_format) | `crates/starter-i18n/src/interpolate.rs::write_timestamp_with_prefs` | extends `starter-i18n/preferences` |

The matching rows in
[docs/design/starter-changes/](../design/starter-changes/README.md)
are flipped to **landed (in-tree)** with file paths.

### What the next session should *not* re-litigate

- The doc-tier rule (sessions / scope / adr / design). Locked.
- The verb-per-file pattern (FILE-LAYOUT). Locked.
- The six-crate count. Locked (R1 may force a future split; not yet).
- Postgres only, backend only, agent = starter's `ai-agent` node
  kind. Three ADRs.
- The five-field tool descriptor contract (R12).
- The SSE event taxonomy (R13).

---

## 3. ACTIVE PLAN — the thin slice

The five A–E targets below remain *valid* but are now subordinate
to the **thin-slice plan in [docs/scope/THIN-SLICE.md](../scope/THIN-SLICE.md)**.
Read that file first; it defines five PRs that exercise every
architectural layer end-to-end:

| PR | Layer added | Status |
|---|---|---|
| 1 | disk tool standalone (DTO + dispatch + recorded-LLM test) | next up |
| 2 | auth + authz + audit + Postgres migrations | blocks on PR 1 |
| 3 | MCP exposure with EN/ES round-trip | blocks on PR 2 |
| 4 | ClickHouse history + insights rule + alert | blocks on PR 3 |
| 5 | extension contribution via `com.rubix.example` | **deferred** until `starter-ext-flow` lands |

PR 1 ≈ Target C from the old A–E list. PR 2 absorbs Target A's
remaining design-doc work. Targets B, D, E are still valid but
defer until after the thin slice lands.

## 3a. Old targets — pick one, finish it (LEGACY)

Each target below is **one PR-sized chunk**. Don't try to combine
them. Order is the recommended sequence but the user may pick a
different starting point.

### Target A — Phase 0 → Phase 1 entry gates

**What.** Tighten the Phase 0 binary so the Phase 1 entry gates
pass before any tool code is written.

**Why first.** Phase 1 starts in earnest only when these are met
(see SCOPE Phase 1 "Entry gate" block). Doing them as a separate
PR keeps the first tool PR small.

**Deliverables.**

1. **`docs/design/i18n-prefs/`** is already written — verify it
   matches reality after target B lands, and add a short
   "Localisation" section template that future SKILL.md files
   inherit.
2. **`docs/design/ai-providers/README.md`** — replace the
   placeholder with present-tense content describing how
   `rubix-agent` selects an `AiRunner` impl at boot. Resolve GAPS
   item #19.
3. **`docs/design/audit/README.md`** — present-tense description
   of the changelog + audit + agent-log wiring rubix uses (even
   if not yet wired — the design doc lets target C plug in).
4. **`docs/design/migrations/README.md`** — finalise the boot
   ordering rules + the cross-tree-FK rule.

**Files touched.** Four design-doc READMEs. Zero Rust.

**Exit signal.** Each design doc's "Status" line drops the word
"placeholder".

---

### Target B — The bundle loader + the first real `MessageKey` wiring

**What.** Wire `rubix-spi::i18n` into a real `MessageBundle` and
demonstrate one round-trip rendering through
`MessageBundle::render_diagnostic` against fixtures.

**Why second.** Every later tool emits keys; the loader has to
exist first. The starter capabilities are already landed (S1-S4),
so this is pure rubix wiring.

**Deliverables.**

1. New helper `rubix_spi::i18n::rubix_bundle() -> Result<MessageBundle, CatalogError>`
   that loads the two embedded catalogues into a `MessageBundle`
   with `"en"` as the fallback tag.
2. `rubix-spi` grows a dependency on `starter-i18n` (with the
   `units` + `preferences` features) so consumers don't have to
   wire it themselves.
3. A unit test next to the loader proving:
   - `rubix.skill.denied` renders correctly in EN and ES.
   - A `Quantity` param renders with the caller's preferred unit
     when run through `render_diagnostic`.
4. Update `crates/rubix-agent/src/main.rs` to call the loader
   once at boot and log the catalogue size alongside the existing
   `tools=0 skills=6 flows=6` line.

**File-layout check.** The loader is one function. It goes in
`crates/rubix-spi/src/i18n/load.rs` (verb file), re-exported
from `crates/rubix-spi/src/i18n/mod.rs`. Don't put it in `mod.rs`.

**Exit signal.** `cargo test -p rubix-spi` shows the round-trip
test passing. `cargo run -p rubix-agent` logs the bundle size.

---

### Target C — First real tool end-to-end (`rubix.system.disk`)

**What.** Implement the smallest real tool, hand to hand, so the
shape is locked in before the other 29 verbs are written.

**Why third.** Locks in the **canonical pattern** every later
verb copies. Get this right and the rest is mechanical.

**Deliverables.**

1. **Wire DTO** in `crates/rubix-spi/src/dto/system/disk.rs`:
   `DiskUsageRequest`, `DiskUsageResponse` (carrying `Quantity` +
   `percent`), and a `&'static ToolDescriptor` honouring R12's
   five fields (purpose / when / when-not / example / siblings).
2. **MessageKey entries** added to `en.json` + `es.json`:
   `rubix.system.disk.ok`, `rubix.system.disk.warn`,
   `rubix.system.disk.full`.
3. **Dispatch logic** in
   `crates/rubix-tools/src/system/disk.rs`: reads the local host's
   disk via a small `starter-tool-sysdiag`-shaped helper (or
   inline `sysinfo` — note the upstream candidate per GAPS #1).
   Returns a `Diagnostic` summary + structured data.
4. **Client method** in `crates/rubix-client/src/system/disk.rs`.
5. **Integration test** in
   `crates/rubix-tools/tests/system_disk_test.rs`: drop the
   `#[ignore]` and assert the round-trip.
6. **Update SKILL.md** for `system-checker` if the descriptor
   surface changes.

**File-layout check.** Every file ≤ 100 lines if possible. ≤ 400
hard. Tool file is **dispatch only**; DTO + descriptor live in
`rubix-spi`. Test mirrors source path.

**Exit signal.** `cargo test -p rubix-tools --test system_disk_test`
passes; `rubix-agent` boots and the tool appears in the (still
empty) registry surface.

---

### Target D — `starter-tool-sysdiag` upstream PR

**What.** Pull the disk + db + flow_errors helpers from Target C
out into a new starter crate per R2 (upstream first).

**Why fourth.** Locks in the upstream-first pattern. Any other
starter consumer with an operator presence wants these.

**Deliverables.**

1. New `crates/starter-tool-sysdiag` matching the existing
   `starter-tool-github`, `starter-tool-slack` shape.
2. Move the platform-agnostic disk/db/flow-error probes from
   `rubix-tools::system` into the new crate.
3. `rubix-tools::system` becomes a thin re-export.
4. Update [docs/design/starter-changes/](../design/starter-changes/README.md)
   row for `starter-tool-sysdiag` to **landed (in-tree)**.
5. Update [docs/scope/GAPS.md](../scope/GAPS.md) item #1 with
   "addressed in Phase 1 — see docs/design/starter-changes/".

**Exit signal.** `cargo build -p starter-tool-sysdiag` green.
`rubix-tools` still passes its tests after the move.

---

### Target E — Doc-tier lint (GAPS #22)

**What.** A simple grep-based pre-commit hook that fails if any
`rubix/crates/**/*.rs` references a forbidden doc tier.

**Why eventually.** Until the lint exists, doc-tier discipline is
honour-system. One short shell script is enough.

**Deliverables.**

1. `scripts/lint-doc-refs.sh` — greps every `*.rs` under
   `rubix/crates/` for forbidden substrings:
   - `SCOPE.md`, `HOW-TO-CODE.md`, `NEW-SESSION.md`,
     `FILE-LAYOUT.md`, `docs/scope/`, `docs/sessions/`,
     `docs/adr/` (the last one needs care — code may rarely link
     an ADR for non-obvious choices, so the rule is "warn, don't
     fail").
2. Add to `rubix/mani.yaml` as `mani run lint-doc-refs`.
3. Optional: a `.git/hooks/pre-commit` that calls it.

**Exit signal.** Running `mani run lint-doc-refs` on the current
tree returns clean (the linter pass earlier in this session
already normalised every code comment).

---

## 4. What to NOT do next session

These are easy to drift into. Don't.

- ❌ **Don't write more tool stubs.** The 30 verb stubs already
  exist with placeholder bodies. Adding more is busywork.
- ❌ **Don't add a seventh crate.** Stay at six until R1 (400
  lines) forces a split.
- ❌ **Don't translate skill bodies or descriptors.** EN canonical.
  Re-read i18n-prefs §"Skills and tool descriptors stay EN" if
  tempted.
- ❌ **Don't reach into starter without filing an upstream item
  first.** R2.
- ❌ **Don't add code comments that reference SCOPE / HOW-TO-CODE /
  sessions / scope.** Only `docs/design/<area>/README.md`.
- ❌ **Don't combine targets A–E into one PR.** Each is a
  reviewable chunk on its own.

---

## 5. Open questions still on the books

From SCOPE.md and GAPS.md. The next session may need to surface
one of these to the user.

| # | Question | Resolves before |
|---|---|---|
| Q1 | `starter-flow-node-loop` vs `-adk` (starter D1) | Phase 1 actual code |
| Q3 | Where dashboard SDUI pages physically live | Phase 3 |
| Q4 | Cron via `FlowAsService` vs new node kind | Phase 4 |
| Q5 | Multi-tenant ClickHouse isolation strategy | Phase 4 |
| Q6 | Does `rubix-client` justify existing as a separate crate? | When the OpenAPI codegen is real |

If Target A is picked, **none of these need answering yet**. They
become entry gates for later targets.

---

## 6. Reproduction commands

What the next session runs first to confirm the build is still
where this session left it:

```bash
cd /home/user/code/rust/starter

# Sanity: every rubix crate builds.
cargo build -p rubix-spi -p rubix-tools -p rubix-skills \
            -p rubix-flows -p rubix-client -p rubix-agent

# Sanity: every starter change still builds, all features on.
cargo build -p starter-spi --all-features
cargo build -p starter-i18n --features preferences

# Tests still pass.
cargo test -p starter-spi --all-features
cargo test -p starter-i18n --features preferences

# The Phase 0 binary boots.
RUBIX_BIND=127.0.0.1:8088 cargo run -p rubix-agent &
sleep 2
curl -sf http://127.0.0.1:8088/healthz   # expects {"status":"ok"}
kill %1
```

If any of the above fails, **stop and ask the user before
proceeding** — something has drifted since this session's commit.

---

## 7. The actual prompt for the next session

Paste the block below into a fresh agent. It is the entire
hand-off.

```
You are starting a new coding session on the rubix project.

First, read these files in order, fully, no skimming:

  1. /home/user/code/rust/starter/rubix/NEW-SESSION.md
  2. /home/user/code/rust/starter/rubix/HOW-TO-CODE.md
  3. /home/user/code/rust/starter/rubix/FILE-LAYOUT.md
  4. /home/user/code/rust/starter/rubix/SCOPE.md
  5. /home/user/code/rust/starter/rubix/docs/design/agent/README.md
  6. /home/user/code/rust/starter/rubix/docs/design/i18n-prefs/README.md
  7. /home/user/code/rust/starter/rubix/docs/design/starter-changes/README.md
  8. /home/user/code/rust/starter/rubix/docs/scope/GAPS.md
  9. /home/user/code/rust/starter/rubix/docs/sessions/2026-05-23-next-steps.md

The last file (the session handoff) is the most important — it
names the targets in the order I want them done and tells you
exactly what NOT to do.

After reading, run the reproduction commands in section 6 of the
handoff to confirm the build is still green. If anything fails,
stop and tell me before touching code.

Then ask me:

  "Which target do you want to start with — A, B, C, D, or E? Or
   do you want to discuss something not on the list first?"

Hard rules you must internalise before answering anything:

  - Doc tiers. Code comments reference docs/design/ only — never
    SCOPE, HOW-TO-CODE, NEW-SESSION, FILE-LAYOUT, docs/scope/,
    or docs/sessions/.
  - File layout. ≤ 400 lines per file hard, ~100 typical. One
    verb per file. No utils / helpers / common / misc.
  - Upstream first. If a capability could benefit any other
    starter consumer, file the upstream item before adding it to
    rubix.
  - Skills and tool descriptors stay EN canonical.
  - Tool outputs are Diagnostic + structured data, never strings.
  - Tests live with the code in the same PR.
  - Comments explain why, never what. No phasing markers, no
    emojis, no "previously this used X".

Do not start writing code until I confirm the target.
```

---

## 8. Anything else the next session needs to know

- **The linter has run.** Source files use the new
  `docs/design/<area>/README.md` cross-reference style. If you
  see a `docs/design/X.md` link in code, that's drift — fix it.
- **The catalogue files are the source of truth for MessageKeys.**
  Adding a `MessageKey::new("rubix.foo")` in Rust without a
  matching entry in both `en.json` and `es.json` fails review.
- **Phase 0's binary uses `RUBIX_BIND`** because the host
  development setup already has port 8080 occupied. Production
  config arrives with `starter-config` wiring in Phase 2a (GAPS
  #15).
- **`rubix-old/`** is the previous incarnation of the project
  (sibling tree). Don't read from it; don't copy from it. If
  something looks useful there, ask first.
