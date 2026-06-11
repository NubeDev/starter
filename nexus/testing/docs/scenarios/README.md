# Scenarios — Cross-Feature Golden Paths

> Feature docs ([../features/](../features/)) test one feature in isolation.
> Scenarios chain them into the user-visible flows that must *always* work — the
> regression set. **Status: scaffold** — the golden path is defined; per-step
> commands fill in as each feature lands.

Each scenario is a numbered, top-to-bottom runbook with ✅ gates. A scenario is
green only when every gate holds. On first failure → capture + triage + fix
([../feedback-loop/](../feedback-loop/)).

---

## S1 — Telemetry to Dashboard (the headline path)

The end-to-end proof that the platform works: generated data becomes a live page.

1. Stack up + broker ([../00_setup/QUICKSTART.md](../00_setup/QUICKSTART.md)).
2. Ingest flow running ([../features/FLOWS_MQTT_INGEST.md](../features/FLOWS_MQTT_INGEST.md)).
3. datapump pushing (fixed seed + count for a known dataset).
4. Dashboard + panel querying the ingested table ([../features/DASHBOARDS.md](../features/DASHBOARDS.md)).
5. Variable filters the panel by `site_id`; nav node mounts the page.
6. ✅ The page shows the data; counts match what datapump produced; the variable
   filters correctly.

## S2 — Insight + Alert on Live Data

Builds on S1's flowing data.

1. Insight resamples + flags anomalies; preview matches expectation.
2. Alert rule on the meter value; datapump seed drives it across threshold.
3. ✅ Insight transforms the panel; alert fires after dwell and resolves when the
   value drops. ([../features/INSIGHTS_ALERTS.md](../features/INSIGHTS_ALERTS.md))

## S3 — Multi-User Access

Proves the access model on top of a populated tenant.

1. Non-admin user + team; grant `viewer` on one nav node.
2. ✅ Non-admin sees only the granted node/page; denied everything else;
   cross-tenant isolation holds. ([../features/NAV_USERS_TEAMS.md](../features/NAV_USERS_TEAMS.md))

## S4 — Determinism / Regression

The repeatable baseline for catching regressions.

1. Fixed datapump `--seed --count`; ingest; record exact aggregates (count per
   `kind`, per `site`).
2. ✅ Re-running from clean DB reproduces the same aggregates bit-for-bit. Any
   drift is a regression to triage.

---

## Running scenarios

Until a driver script exists, run the runbook by hand. The intended automation:
`testing/scripts/run-scenario.sh S1` → stack up, execute the gates, auto-capture
an evidence bundle on first ❌. Build it once S1's manual steps are stable (see
[../feedback-loop/FIX_LOOP.md](../feedback-loop/FIX_LOOP.md) "scripted driver").

## Adding a scenario

Keep them few and high-value — golden paths, not exhaustive coverage (that's the
feature docs' job). Each new scenario: a numbered list of steps, explicit ✅
gates, and a pointer to the feature docs it composes.
