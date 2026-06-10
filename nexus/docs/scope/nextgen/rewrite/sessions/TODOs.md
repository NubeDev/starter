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
  (3) single-output config shape needs the one-grep fan-out check before freeze,
  (4) [codex review] `Source::commit()` default-no-op ack hook — the pipeline calls it
      after each successful sink write (§6 delivery semantics; MQTT implements it later).
  Unaffected-but-frozen-later: schema stability rule (RW-02), source_on_error policy (RW-08).
- **Why:** §6 freezes when RW-02 starts; cheaper to align now than after the freeze.
- **Proposed:** Whoever gates RW-01: if it shipped `&self` or lacks the batch bound, do NOT
  fail the gate — spawn the same-charter fix pass (gate step 4 mechanism) or fold the
  alignment into RW-02's first action (it re-reads §6 anyway). Both are small mechanical
  changes while core/ has a single consumer.

## 2026-06-10 RW-02 — Built against RW-01's as-committed contract; three core deltas still open
- **Type:** follow-up
- **What:** The reconciliation above was NOT applied to `core/` before RW-02 ran. RW-02 ported
  every node against the committed `core::node.rs`: `Processor::process(&self)`, no
  `Source::commit()`, no `max_batch_rows` slicing in `core::pipeline.rs`. All three of those
  live in RW-01's lane (`core/**`), which RW-02 must not restructure (ROADMAP §4), so they
  remain open:
  (1) `&self` vs `&mut self` — RW-02's processors are `&self`; harmless to flip to `&mut self`
      later (none need shared access), but it is an RW-01 trait change.
  (2) `Source::commit()` ack hook — absent. RW-02's ports are pull-only (memory/generate/http
      poll/simulator) and need no ack; MQTT (later) is the first that does.
  (3) `max_batch_rows` source/processor-output slicing — absent from the pipeline. RW-02's
      sources emit small batches (1 doc, or batch_size small docs), so no OOM risk yet; the
      fat-batch case is RW-08's soak test, which needs the pipeline-side slice to exist.
- **Why:** These are `core/` (RW-01) changes; RW-02 stays in its `{source,sink,processor}` lane.
- **Proposed:** RW-03 (next to touch `core/`-adjacent runner wiring) or an RW-01 fix pass:
  add the slice + commit hook to `core::pipeline.rs` and flip the trait to `&mut self`. RW-02's
  nodes compile unchanged under `&mut self` and gain a default-no-op `commit()`.

## 2026-06-10 RW-02 — Native `sql` omits ArkFlow's JSON UDFs (confirm before vendor delete)
- **Type:** follow-up
- **What:** ArkFlow's vendored `sql` processor registers `datafusion_functions_json` + a custom
  `udf::init` set on its SessionContext. The native `processor/sql.rs` uses a plain
  `SessionContext` (no JSON UDFs) — no stored flow / existing test uses them, and §8 has not
  approved a JSON-UDF dep.
- **Why:** Adding the dep speculatively would violate §8 (no unapproved heavy deps); dropping it
  silently could break a tenant flow that used `json_get(...)` in its SQL.
- **Proposed:** RW-03, before deleting `vendor/arkflow-plugin/src/processor/sql.rs`: grep stored
  tenant flow configs for JSON-UDF usage in `sql.query`. If any exist, raise a blocker to get
  `datafusion-functions-json` approved as a direct dep; otherwise the omission is safe.
