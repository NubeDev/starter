# Nexus Testing & Feedback Suite

> **You are an AI session picking this up cold. Read this file top to bottom, then
> jump to the doc that matches your task.** Every doc is a self-contained runbook:
> exact commands, exact files, exact pass/fail checks. Nothing here assumes prior
> context beyond what is written down.

This suite does three things:

1. **Stand up Nexus with realistic, generated data** — telemetry pumped over
   MQTT / Zenoh from [`testing/datapump`](../datapump), buffered through a flow,
   landed in Postgres.
2. **Exercise each feature** end-to-end with scripted scenarios (dashboards, nav,
   insights, alerts, users/teams, flows).
3. **Close a feedback loop** — capture a consistent evidence bundle (logs,
   metrics, row counts, errors) so an AI session can diagnose and fix Nexus, then
   re-run and confirm the fix.

---

## The data path (read this once, it explains everything)

```
 datapump ──MQTT/Zenoh──▶  broker  ──▶  Nexus ingest source  ──▶  flow buffer  ──▶  postgres sink
(generator)              (mosquitto/    (zenoh source OR        (bounded chan,    (bind-param
                          nats/zenoh)    http_ingest bridge)     backpressure)     inserts)
                                                                                       │
                                                                                       ▼
                                                              query ──▶ dashboards / insights / alerts
```

Key facts that shape all testing (verified 2026-06-10, see
[reference/ARCHITECTURE.md](reference/ARCHITECTURE.md) for file:line evidence):

- **No native MQTT *source* in the backend.** Nexus ingests via a `zenoh` source
  or `http_ingest` (`POST /api/v1/ingest/{flow_id}`). `datapump` pumps MQTT or
  Zenoh; an MQTT path therefore needs a bridge or the Zenoh transport.
- **Flows are JSON** (`input` / `pipeline` / `output`) stored in `nexus_flows`,
  validated when the flow starts. Sources/processors/sinks are named nodes.
- **Everything is tenant-scoped with Postgres RLS.** A row written under the
  wrong `app.tenant_id` is invisible — a common "where did my data go" trap.
- **Nav nodes (not dashboards) are the access-grant unit.** Pages mount onto nav
  nodes; grants attach to nodes.
- **Postgres is the only store** (separate metadata + datasource pools).

---

## Map of this suite

| Path | What it is | When to read it |
|------|-----------|-----------------|
| [`00_setup/QUICKSTART.md`](00_setup/QUICKSTART.md) | One-page bring-up: DB, backend, broker, datapump, first rows | **Start here, always.** |
| [`00_setup/STACK.md`](00_setup/STACK.md) | Every process, port, env var, and how to tear down cleanly | When something won't start |
| [`00_setup/DATAPUMP.md`](00_setup/DATAPUMP.md) | The generator: transports, payload, topics, determinism, knobs | When you need specific data shapes |
| [`features/`](features/) | One runbook per feature (see below) | When testing/fixing that feature |
| [`scenarios/`](scenarios/) | Cross-feature golden-path scripts ("ingest → dashboard → alert") | End-to-end / regression |
| [`feedback-loop/`](feedback-loop/) | Evidence capture + triage + fix loop for AI sessions | When something is broken |
| [`reference/ARCHITECTURE.md`](reference/ARCHITECTURE.md) | Grounded system map with file:line citations | When a doc's claim looks stale |
| [`reference/API_CHEATSHEET.md`](reference/API_CHEATSHEET.md) | curl-ready endpoint list, auth, tenant headers | Constantly |

### Feature runbooks (`features/`)

| Doc | Covers | Status |
|-----|--------|--------|
| [FLOWS_MQTT_INGEST.md](features/FLOWS_MQTT_INGEST.md) | datapump → broker → flow → postgres; buffering; metrics | scaffold |
| [DASHBOARDS.md](features/DASHBOARDS.md) | Create pages, queries, variables/context, assign to nav | scaffold |
| [NAV_USERS_TEAMS.md](features/NAV_USERS_TEAMS.md) | User/team CRUD, nav tree, per-node access grants | scaffold |
| [INSIGHTS_ALERTS.md](features/INSIGHTS_ALERTS.md) | Insight transforms + alert rules on the MQTT/PG data | scaffold |
| [EXTENSIONS_LIFECYCLE_AND_API.md](features/EXTENSIONS_LIFECYCLE_AND_API.md) | Extension lifecycle state machine, the full `/api/v1/extensions/*` admin API, install/enable/disable/restart/purge semantics, WS-17 data-access host methods | verified |

> **verified** = run live against a stack and the commands/outputs confirmed.
> **scaffold** = structure + acceptance criteria written; commands to be filled
> in as we knock off each feature one at a time. Each scaffold lists exactly what
> "done" looks like so the next session knows when to stop.

---

## How to work in this suite (the operating contract)

1. **Bring up the stack** via [`00_setup/QUICKSTART.md`](00_setup/QUICKSTART.md).
   Confirm the health checks pass before doing anything else.
2. **Pick a feature or scenario doc.** Run its steps top to bottom.
3. **If a step fails**, do not guess. Go to
   [`feedback-loop/CAPTURE.md`](feedback-loop/CAPTURE.md), produce the evidence
   bundle, then follow [`feedback-loop/TRIAGE.md`](feedback-loop/TRIAGE.md).
4. **When you fix Nexus**, re-run the failing step and the relevant scenario;
   record the before/after in the feature doc's "Known issues / fixes" section.
5. **If a doc's stated fact is wrong** (drifted code), fix the doc first, bump its
   `Verified:` line, then proceed — same discipline as the `WS-xx` scope docs.

Each doc carries a `> Verified: <commit> on <date>` header. Treat anything older
than the current branch tip as unverified — re-grep before trusting it.

---

## Conventions used throughout

- Commands assume CWD `nexus/` unless a doc says otherwise.
- Env defaults: backend on `127.0.0.1:4780`, MQTT broker `127.0.0.1:1883`,
  NATS `4222`, Zenoh `7447`. Overridable — see [`00_setup/STACK.md`](00_setup/STACK.md).
- `$BASE` = `http://127.0.0.1:4780`. `$TOKEN` = a logged-in bearer (the
  cheatsheet shows how to mint one). Tenant is carried by the token.
- Evidence lands in `testing/.evidence/<scenario>/<timestamp>/` (git-ignored).
- ✅ / ❌ checkboxes in runbooks are literal pass/fail gates — a scenario is green
  only when every ✅ holds.
