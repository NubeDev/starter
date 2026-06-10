# Fix Loop — The AI Working Contract

> You've captured evidence ([CAPTURE.md](CAPTURE.md)) and identified a root cause
> ([TRIAGE.md](TRIAGE.md)). This is how to change Nexus and **prove** the fix.

---

## Before you change anything

1. **State the root cause in one sentence**, citing the file:line and the
   evidence artifact that proves it. If you can't, you're guessing — go back to
   triage.
2. **Decide: is this a Nexus bug, a test/doc bug, or expected behavior?**
   - Test/doc bug → fix the doc, bump `Verified:`, done. Don't touch Nexus.
   - Expected behavior → record it in the feature doc's "Gotchas", done.
   - Nexus bug → proceed.
3. **Find the smallest change** that addresses the cause, not the symptom. A
   symptom patch (e.g. widening a type to swallow bad data) usually moves the bug.

---

## Making the change

- Match the surrounding code's idioms (this is a Rust workspace —
  `backend/crates/nexus-*`). Run `cargo fmt` / `cargo clippy` expectations.
- One logical fix per change. If you find a second bug, capture it separately.
- If the fix touches a DTO, follow the project flow:
  `nexus-spi` DTO → register in `openapi.rs` → regenerate `openapi.json` →
  (UI) `pnpm codegen`. The FE contract must stay in sync.
- If it touches a table, it needs its own numbered migration (don't edit applied
  ones).

---

## Proving the fix (mandatory)

A fix is not done until **all** hold:

1. ✅ `cd backend && cargo test` green (and `cargo build`).
2. ✅ The originally-failing ✅ check now passes — re-run that exact step.
3. ✅ The full scenario it belonged to is green end-to-end (no new red).
4. ✅ A fresh evidence bundle in a new timestamp dir shows the symptom gone
   (before/after comparable).
5. ✅ No silent regression: the metric/log line that was wrong is now right *for
   the right reason* (you can explain it), not just absent.

---

## Record it

In the relevant `features/<X>.md` "Known issues / fixes" section, append:

```md
### <date> — <one-line symptom>
- **Symptom:** <expected vs actual>
- **Evidence:** testing/.evidence/<scenario>/<ts>/
- **Root cause:** <file:line + why>
- **Fix:** <what changed> (commit <hash>)
- **Verified:** re-ran <step/scenario> → green; before/after bundle <ts2>
```

This turns each fix into institutional memory the next session can search.

---

## When to stop and ask

- The fix requires a design decision (new DTO shape, new migration semantics,
  changing the access model) → surface it, don't unilaterally reshape contracts.
- The "bug" is actually a missing feature (e.g. no MQTT source) → that's a
  feature task, not a fix; note it and pick it up deliberately.
- Two fixes conflict / the cause is in a shared `starter-*` crate other apps use
  → flag the blast radius before changing it.

---

## Optional: scripted driver

For repeated regression runs, a `testing/scripts/run-scenario.sh <name>` that
brings up the stack, runs a scenario's checks, and auto-captures on first failure
makes the loop one command. Build it when the manual loop stabilizes; reference
it from [../scenarios/README.md](../scenarios/README.md). Keep the human/AI in the
loop at the fix step — autonomous code changes still get the full "proving the
fix" gate above.
