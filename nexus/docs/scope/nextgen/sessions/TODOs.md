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
