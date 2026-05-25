# <stage-NN> — <short topic>

> Session note. **Not** a design doc. Date-stamped, narrow, scoped
> to one issue that came up while working a stage. Stage docs
> (`01-producer.md`, `02-ingest-l1.md`, …) stay clean; spillover
> lives in files like this one.
>
> Filename convention:
> `<stage-NN>-<short-topic>-YYYY-MM-DD.md`
> e.g. `01-producer-rhai-rng-seed-2026-05-26.md`

---

## Context

- **Stage:** 01 / 02 / 03 / 04 / 05  ← pick one
- **Started from:** commit `<short-sha>`
- **Trigger:** what stopped me from ticking the stage's success bar
  (paste the failing command + observed output, ≤ 10 lines).

## What I tried

Ordered list of things attempted, with the actual command and the
actual response. Keep it terse — this is for the next session, not
a story.

1. …
2. …
3. …

## What I found

The root cause (or "still unknown — best guess is X"). One short
paragraph. If you found a bug in a starter or rubix crate, name
the file + line.

## What I changed

- File: `…` — one-line summary.
- File: `…` — one-line summary.

If nothing was changed (investigation only), say so explicitly:
**No code change.**

## What's left

- [ ] open question / next step 1
- [ ] open question / next step 2

If this unblocks the stage's success bar, say so and link back to
the row you flipped in [PROGRESS.md](./PROGRESS.md).

## References

- Stage doc: [./0N-…md](./0N-stub.md)
- Design doc(s): …
- Prior session note(s): …
