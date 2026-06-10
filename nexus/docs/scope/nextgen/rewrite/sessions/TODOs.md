# Nexus Rewrite — TODOs / Blockers

> Sessions append here when blocked (per the AGENT CHARTER no-questions rule) or when deferring
> a follow-up. The human resolves blockers by adding a `✅ RESOLVED:` line under the entry;
> the loop then resets the blocked row to ⬜.

Format per entry:

```
## YYYY-MM-DD <RW-xx> — <one-line title>
- **Type:** blocker | follow-up
- **What:** <what is needed / what was deferred>
- **Why:** <why the session could not proceed / why deferred>
- **Proposed:** <the session's recommended resolution>
```

---

## 2026-06-10 RW-01 — Peer-review contract updates landed mid-flight (reconcile at gate / RW-02)
- **Type:** follow-up
- **What:** Human peer review updated roadmap §6 + RW-01/02/04/05/06/08 specs WHILE RW-01's
  subagent was already building. Deltas affecting RW-01's lane:
  (1) `Processor::process` is now `&mut self` (was `&self`),
  (2) new `max_batch_rows` slicing contract at source/processor output boundary,
  (3) single-output config shape needs the one-grep fan-out check before freeze.
  Unaffected-but-frozen-later: schema stability rule (RW-02), source_on_error policy (RW-08).
- **Why:** §6 freezes when RW-02 starts; cheaper to align now than after the freeze.
- **Proposed:** Whoever gates RW-01: if it shipped `&self` or lacks the batch bound, do NOT
  fail the gate — spawn the same-charter fix pass (gate step 4 mechanism) or fold the
  alignment into RW-02's first action (it re-reads §6 anyway). Both are small mechanical
  changes while core/ has a single consumer.
