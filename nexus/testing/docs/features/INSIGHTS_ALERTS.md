# Feature: Insights & Alerts

> Verified: nexus-rewrite tip on 2026-06-10. **Status: scaffold.**
> Reference: [WS-07_ALERTING](../../../docs/scope/nextgen/WS-07_ALERTING.md).

**What we're testing:** define an insight (post-query transform) over the ingested
meter data, and an alert rule that fires when a condition crosses a threshold —
all against the live MQTT→Postgres telemetry.

Architecture recap ([../reference/ARCHITECTURE.md §3](../reference/ARCHITECTURE.md)):
insights are sandboxed Rhai scripts over the result frame (bind `df` + `params`,
compose vectorized primitives, caps prevent row growth); alerts are
single/multi-condition rules with `interval_secs`/`for_secs`, firing to channels.

---

## Insights runbook (fill in as built)

1. [ ] List available primitives: `GET /api/v1/insights/functions`.
2. [ ] Author a script that, e.g., `resample`s meter `value` to 5-min buckets and
       flags `anomalies` / computes `zscore`.
3. [ ] `POST /api/v1/insights/preview` with the script + a sample query →
       inspect the transformed frame **without saving**.
4. [ ] `POST /api/v1/insights` to save; attach to a panel query via `InsightRef`.

### Insight acceptance

- ✅ Preview returns a transformed frame; row count stays within caps (no
  explosion).
- ✅ Error classes surface correctly: a syntax error → `Compile`; an oversized
  result → `LimitExceeded`; a bad op → `Runtime` (test each deliberately).
- ✅ A saved insight referenced from a panel changes the rendered result.
- ✅ Determinism: same input frame + same params ⇒ same output (use a fixed
  datapump seed for the source data).

### Example insight (TO FILL IN)

```jsonc
// confirm script API + primitive names against GET /insights/functions
{
  "script": "let r = resample(df, \"5m\", \"mean\"); anomalies(r, params.threshold)",
  "params": { "threshold": 3.0 }
}
```

---

## Alerts runbook (fill in as built)

1. [ ] Create a notification channel (stub/log sink for tests).
2. [ ] Create an alert rule (`POST /api/v1/alerts/rules`): a query over the meter
       data, `op` + `threshold`, `interval_secs`, `for_secs`, `channel_ids`.
3. [ ] Use datapump to drive `value` past the threshold (a fixed seed/scenario
       that deterministically crosses it).
4. [ ] ✅ Rule transitions pending → firing after `for_secs` dwell; channel
       receives the notification.
5. [ ] Drive value back under threshold → ✅ rule resolves.

### Alert acceptance

- ✅ Single-condition rule fires when the threshold is crossed and the `for_secs`
  dwell elapses (not before).
- ✅ Multi-condition rule honors `combinator` + `no_data_policy` +
  `exec_error_policy` (test a no-data window and a query error).
- ✅ A silence suppresses notifications without changing rule state.
- ✅ ⚠️ **Confirm the evaluator scheduler runs** — the API accepts rules, but
  auto-fire depends on a background evaluator that this doc has NOT re-verified.
  First thing to check if a rule never fires: is anything evaluating it on a
  schedule? (grep `nexus-insights` / alert routes for the scheduler.)

---

## Deterministic alert scenarios

To test fire/resolve reliably, drive the data, not the clock: pick a datapump
seed + meter whose value sequence is known to cross the threshold at a known
publish index. Then the alert's fire is reproducible across runs.

---

## Known issues / fixes

- ⚠️ Alert evaluator scheduler unverified in the 2026-06-10 sweep — verify it
  exists/runs before trusting auto-fire timing.
- _record fixes here_
