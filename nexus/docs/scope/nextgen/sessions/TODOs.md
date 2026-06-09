# Nexus Next-Gen — Blockers & Questions for the Human

> Sessions are forbidden to ask questions or hack/stub around problems. When a session hits
> something it genuinely cannot resolve (a real design ambiguity, a hard dependency on work a
> later session owns, a missing decision, a flaky/broken external dependency), it **writes a
> dated entry here, marks its WS `⛔ blocked` in STATUS.md, and the loop moves to the next
> unblocked workstream.**
>
> Triage these in the morning. When you resolve one, the next loop wake will see the WS is
> unblocked (status back to ⬜) and re-run it.

## How to write an entry (sessions follow this format)

```
### [YYYY-MM-DD HH:MM] WS-xx — <one-line blocker title>
- **What I was doing:** <the concrete task>
- **The blocker:** <why it can't proceed without a human decision — be specific>
- **Options I see:** <2–3 concrete options, with the trade-off of each>
- **My recommendation:** <which option, and why>
- **What I did instead:** <skipped / partial-landed X / marked WS blocked>
- **To unblock me:** <the exact decision or change you need to make>
```

---

## Open blockers

<!-- newest first -->

### [2026-06-09 12:40] WS-03 — pre-existing `pnpm typecheck` failure in `starter-ui-authz` (out of lane)
- **What I was doing:** running the DoD gate (`pnpm typecheck && pnpm build`) for WS-03.
- **The blocker:** `packages/starter-ui-authz/src/panels/authz-admin.tsx:305` declares
  `function TenantRail(...)` that is never used → `TS6133 'TenantRail' is declared but its value is
  never read`, which fails `pnpm typecheck` for the whole workspace. This file is **committed at
  HEAD (`90939747`)**, is **not touched by WS-03**, and is **outside WS-03's owned files** (ROADMAP
  §4 — WS-03 owns `features/query-editor/**` + query/query-history API). It fails on a clean base
  independent of my work.
- **Options I see:** (a) delete the unused `TenantRail` function (one line of dead code) — but it's
  another workstream's file; (b) leave it for the owning session / a human to clean up.
- **My recommendation:** (a) — it's a trivial dead-code removal and the gate is shared; but per the
  "stay in your lane / commit only your hunks" rule I did **not** edit it.
- **What I did instead:** WS-03's own code is fully green — `cargo test` passes (binder: 17/17),
  `pnpm build` is green, `pnpm test` is green (90 passed). Only `pnpm typecheck` trips on this
  unrelated file. Landed all WS-03 work; flagging this so it doesn't read as a WS-03 regression.
- **To unblock the shared typecheck gate:** remove the unused `TenantRail` in
  `starter-ui-authz/src/panels/authz-admin.tsx` (the session that owns that package, or a human).
