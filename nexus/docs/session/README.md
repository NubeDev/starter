# READ ME BEFORE CODING — Nexus session conventions

One page. Read it before you write a line, then read your scope doc:
- Backend (Rust, `nexus/backend/`) → [backend/SCOPE.md](backend/SCOPE.md)
- Frontend (TS/React, `nexus/ui/`) → [ui/SCOPE.md](ui/SCOPE.md)
- Architecture (why it's shaped this way) → [../scope/NEXUS.md](../scope/NEXUS.md) + [../scope/NEXUS_TOPOLOGY.md](../scope/NEXUS_TOPOLOGY.md)
- Layout law (Rust **and** TS) → [../../../rubix/FILE-LAYOUT.md](../../../rubix/FILE-LAYOUT.md)

These apply to **every** file a subagent writes in `nexus/backend/` and `nexus/ui/`.

---

## 1. One responsibility per file

≤400 lines hard, ~100 typical; one verb/component per file; folder-of-verbs over
file-of-nouns; names are concepts. **No `utils` / `helpers` / `common` / `misc` / `types`
files.** Full reference: [FILE-LAYOUT.md](../../../rubix/FILE-LAYOUT.md).

---

## 2. Comments explain *why* — never *what*, and **never stage/process/status**

This is the rule that gets broken first. Code is read for years; the session that produced
it is forgotten in a week. A comment that records *when* or *in what step* something was
written is noise the moment the step is over.

**Banned in source code — without exception:**

| Don't write | Why it's banned |
|---|---|
| `// STAGE-1 done`, `// Phase 0`, `// M0 seam`, `// step 3` | Process/stage markers. The phase is over; the comment lies. |
| `// per R4`, `// SCOPE M1`, `// see SCOPE.md` | Planning/milestone refs. Planning docs aren't the code's contract (§3). |
| `// FIXED:`, `// fix scan devices`, `// added scan`, `// updated` | Changelog-in-comments. That's what `git log` is for. |
| `// Previously this used X`, `// was Recharts` | History. The diff records it; the comment rots. |
| `// TODO` (bare), `// HACK`, `// XXX` | Orphan markers no one owns. |
| `// 🚀`, `// ====== SECTION ======`, ASCII banners, emoji | Decoration, not information. |
| `// returns the user` above `fn get_user() -> User` | Restates *what*. Adds nothing. |

**Write instead:**

- **Why**, not what: the non-obvious reason, the constraint, the edge case, the gotcha.
  - ✅ `// Postgres bypasses RLS for the table owner, so we connect under the runtime role.`
  - ✅ `// EventSource can't send an Authorization header — token rides the query string.`
  - ✅ `// Drain on stream completion; the collector buffers in-process, so caps are enforced upstream.`
- **Doc-comments** (`///` / TSDoc) on every public item: purpose, defaults, edge cases.
- **Owned TODOs only:** `// TODO(ap): cap result bytes once the limit is wired.` Never bare.

Smoke test before committing a comment: *will this still be true and useful in a year, to
someone who never saw this session?* If it's about a step, a fix, a phase, or a rule number —
delete it. The milestone labels (M0/M1, R4/R5, P0/P1) in the SCOPE docs are for **planning the
work**, not for annotating the code that results from it.

---

## 3. Code comments don't reference the planning docs

`SCOPE.md`, this README, and anything under `docs/session/` or `docs/scope/` are
**contributor/planning docs** — they describe how we're building and why we chose to, not what
the system *is*. A code comment that links them lies the moment scope shifts.

- ❌ `// implements SCOPE.md §5.2 two-layer SQL`
- ✅ a doc-comment that explains the behaviour in present tense, self-contained.

If a stable design doc exists for an area, reference *that*; otherwise the code explains
itself. Never cite a milestone, rule number, or session note from source.

---

## 4. Layer separation

- **Backend:** transport handlers are thin (≤20 lines) — extract input → call one
  engine/store function → shape the DTO → return. No SQL, no business logic, no ArkFlow calls
  in a route file. (SCOPE R10.)
- **Frontend:** widgets/components are pure — they render from props/hooks, never fetch or
  cause side effects inside the component. Data arrives via the `api/` layer. (SCOPE F6.)

Smoke test: *swap REST for gRPC (backend) / swap the chart lib (frontend) — does anything but
wiring change?* If yes, logic leaked into the wrong layer.

---

## 5. Test-driven — the test comes first

**Write the failing test, then the code that makes it pass.** Not test-after, not
test-eventually. Every work-unit lands as: red test → implementation → green → refactor.

- **Backend:** a failing `tests/<mirror>_test.rs` (or inline `#[cfg(test)]` for pure fns)
  before the impl. DB tests run against **real Postgres via testcontainers** — never a stubbed
  store. The M0 engine-seam test and the RLS pool-leak test are written first.
- **Frontend:** a failing test before the component/hook. Pure components are tested by passing
  **typed `Widget`/DTO props** (the component's contract — that is *not* mock telemetry).
  Integration tests run against a **real `nexus-api`** (dev instance / testcontainers /
  Playwright), not a faked network.

Tests live with the code: same PR, same diff; integration tests mirror `src/` one-to-one.

## 6. Real data only — no mocks, fakes, seeds, or stubs in app code

**The product never fabricates data. Ever.** This is absolute, especially on the frontend.

- **Frontend:** **NO** `fake.ts`, **NO** `seed.ts`, **NO** `localStorage` demo data, **NO**
  hardcoded series/dashboards, **NO** MSW/network-mock faking backend responses in the running
  app. Every value on screen comes from `nexus-api` via the real client. If an endpoint isn't
  ready yet, the UI shows **loading / empty / error** states — it does not invent rows. The
  `nexus-ui` mock's fake-data files are **not ported**.
- **Backend:** no stubbed datasources or canned result rows in app code; runners hit real
  engines/DBs. Test fixtures live in `tests/`, never in `src/`.

The line: **typed test inputs in `tests/` are fine; fabricated data anywhere in `src/` is a
bug.**

---

## One-line summary

**One responsibility per file. Comments say *why*, never *what* and never the
stage/fix/phase/rule that produced them. Planning docs stay out of source. Thin transport,
pure components. Test-first, against real dependencies. Zero mock/fake data in app code —
the frontend renders only what `nexus-api` returns.** Then go read your `SCOPE.md`.
