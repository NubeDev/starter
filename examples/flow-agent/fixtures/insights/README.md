# Insights fixtures

Fake data shaped to match the eventual `starter-insights` schema (see
[`DOCS/Insights/SCOPE.md`](../../../../DOCS/Insights/SCOPE.md)). When
the real crate lands, these files are replaced by SQL queries with **no
UI / agent code changes** (I2 from the [mockup spec](../../INSIGHTS-MOCKUP.md)).

## Files

| File | Contents |
|---|---|
| `rules.json` | array of rule rows (id, kind, namespace, severity_default, tags, body, schema, timestamps) |
| `verdicts.json` | array of verdict rows matching `Verdict` (rule_id, at, tz, window, severity, coverage, tags, summary, evidence) |
| `pipelines.json` | pipeline graphs (nodes, edges) consumed by the canvas |
| `coverage.json` | per-rule per-day coverage timeseries (expected/present/confidence) |
| `tags-index.json` | tag → rule_ids reverse lookup |

## Scenarios covered

1. **IoT** — `device.online@1`, `sensor.has-recent-data@1`,
   `sensor.in-range@1`. Includes a `Severity::Error` row (sensor-07
   silent) and an over-range `Critical`.
2. **Energy** — `hq-london` with `meter.baseline-deviation@1` and
   `meter.weather-normalised-overrun@1` (custom Rhai). Includes a
   partial-onboarding gap day and a `starter.quality.retroactive-correction@1`
   row (D5 from SCOPE).
3. **Bills reconciliation** — `bills.reconcile@1` + `bills.ai-judge@1`
   (`rule.ai-check`) over hq-london April invoice, with an
   AI-judge verdict carrying model/cost evidence.

## Regenerating

These are hand-written. Edits should preserve the JSON shapes — if you
need a field that isn't here, check `DOCS/Insights/SCOPE.md` first. New
fields not in SCOPE are an I2 violation (flag SCOPE, then update both).

## Mutation

The `insights_mock.rs` backend writes back to these files via a
journal-style overwrite under `tokio::sync::RwLock`. Commit fixture
changes from the agent's "apply" path the same way you'd commit
generated test data.
