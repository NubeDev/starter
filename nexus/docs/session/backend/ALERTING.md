# Nexus Alerting — Sub-Design

> Companion to [SCOPE.md](SCOPE.md) (§Phases M3) and [../../scope/NEXUS.md](../../scope/NEXUS.md)
> (§6 note, Risk #11). Read [README.md](../README.md) first — the coding rules apply here too.
> This note is the design the alerting code implements; it is a contributor doc, so the code
> does not cite it (README §3) — the code explains itself in present tense.

## Why this is not bare CRUD

`/alerts/rules` + `/alerts/events` are a surface, not the system. The system is the part the
endpoints don't show: a **scheduler** that wakes each rule on its cadence, an **evaluator** that
runs the rule's query and compares it to a threshold, a **state machine** that turns a stream of
evaluations into a small set of meaningful transitions (a rule does not "fire" every 30s — it
fires *once* and resolves *once*), **dedup/silences** so a flapping or maintenance-window rule
does not page a human repeatedly, and **notification channels** that deliver the transitions.
Shipping only CRUD would push all of that onto the client, which cannot do it (it isn't running
when the threshold trips).

## The mental model

An **alert rule** is a saved query + a threshold + a cadence, owned by a tenant. The
**evaluator** runs on a schedule, evaluates each due rule against its datasource (through the
same R4-guarded query path panels use), and feeds the boolean result into the rule's **state
machine**. A transition (`→ firing`, `→ resolved`) produces an **alert event** (the audit row)
and, unless silenced, a **notification** down each of the rule's channels. Direction is one-way
and server-side: nothing about alerting depends on a browser being open.

```
 scheduler tick ─▶ for each due rule ─▶ evaluator: run guarded query, compare threshold
                                              │ bool: breaching?
                                              ▼
                                    state machine (per rule)
                       ok ──breach──▶ pending ──breach×N──▶ firing ──clear──▶ resolved ──▶ ok
                                              │ on a transition only
                                              ▼
                                   write alert_event  +  (unless silenced) notify channels
```

## State machine

States: **ok** (initial, healthy), **pending** (breaching but not yet long enough to fire),
**firing** (confirmed, notified), **resolved** (recovered; a terminal annotation that returns to
ok for the next cycle).

Transitions, evaluated once per rule per tick:

| From | Condition | To | Side effect |
|---|---|---|---|
| ok | breach | pending | start the `for` timer |
| pending | breach, `for` elapsed | firing | event(`firing`) + notify |
| pending | clear | ok | (none — never fired, nothing to resolve) |
| firing | breach | firing | none (already firing — this is the dedup) |
| firing | clear | resolved | event(`resolved`) + notify |
| resolved | (next tick) | ok | none |

`for` (a per-rule duration, default 0) is the **pending dwell**: a rule with `for = 5m` must
breach for five minutes before it fires, which absorbs a single transient spike. `for = 0` fires
on the first breach. The dwell is the first line of dedup; "firing stays firing" is the second
(no repeat notification while a rule remains breaching).

State lives in an `alert_rule_state` row (one per rule): current state, `since` (when it entered
the state), and `last_eval_at`. The evaluator reads and writes it inside the tenant transaction,
so two evaluator runs cannot race a rule's state across tenants, and a restart resumes from the
persisted state rather than re-paging everything.

## Scheduler

A single in-process **tokio task** per `nexus-api` node ticks on a fixed cadence (e.g. every
10s) and asks the store for rules whose `next_eval_at <= now`. It evaluates them, advances their
state, and stamps `next_eval_at = now + interval`. This mirrors the FlowManager's single-node
posture (D8/R7): alerting is single-node for v1 — a multi-node deployment needs leader election
or a shared queue so a rule is evaluated once, not once per replica. **Stated, not discovered.**

The scheduler claims due rules with `SELECT … FOR UPDATE SKIP LOCKED` inside the tenant tx so
that even if a second evaluator is somehow running, a rule is taken by exactly one — the same
guard a multi-node version would lean on, written once now.

Evaluation cost is bounded by the same query guards as panels (read-only role, statement
timeout, row/byte caps): an alert query is just a query, so it inherits R4 for free.

## Threshold / condition

v1 keeps the condition deliberately small and serializable: the rule's query must return a single
numeric column (the evaluator reads the first row, first column), compared by an **operator**
(`gt`/`gte`/`lt`/`lte`/`eq`/`ne`) against a **threshold** float. This covers "CPU > 90",
"free_disk < 10", "error_count >= 1". Richer conditions (multi-series, `count_over_time`, label
matching) are a later expansion — the operator enum is add-only. A query that returns no rows is
treated as **no data**, a distinct non-breaching result that does not flap a rule to firing on a
transient empty read.

## Dedup & silences

**Dedup** is structural, from the state machine: a rule notifies on *transitions only*, so a
rule breaching for an hour pages once (on `→ firing`), not 360 times. There is no separate dedup
key to maintain.

**Silences** suppress *notification* (never evaluation — the event row is still written so the
history is honest) for a matching rule over a time window. A silence row is `{id, tenant_id,
rule_id (or null for tenant-wide), starts_at, ends_at, reason, created_by}`. Before notifying,
the evaluator checks for an active silence covering the rule; if one exists, the event records
`notified = false, silenced = true` and no channel is called. This is the maintenance-window
path: silence a rule before a deploy, the history still shows it fired, no one gets paged.

## Notification channels

A **channel** is a delivery target owned by a tenant: `{id, tenant_id, name, kind, config}`.
v1 ships **webhook** (POST a JSON payload to a configured URL) because it is the universal
integration — Slack, PagerDuty, and email gateways all accept an inbound webhook, and it needs no
provider SDK or secret beyond the URL. The `kind` is an enum and the dispatch is a trait
(`Notifier::notify(event) -> Result<()>`), so `email`/`slack`/`pagerduty` are added as new arms +
impls without touching the evaluator. A rule references zero or more channels; a transition fans
out to each. A channel failure is logged and recorded on the event (`notified = false` with the
error) but does not crash the evaluator or block the other channels — alerting must be robust to
a flaky downstream.

Webhook secrets (if a channel ever needs an auth header) reuse the R6 envelope, exactly like
datasource secrets — not plaintext in the config column. v1 webhook is URL-only, so this is noted
for when an authenticated channel lands, not built speculatively.

## Tables (all tenant-scoped, RLS like datasources/flows)

- `nexus_alert_rules` — `{id, tenant_id, name, datasource_id, query, op, threshold, for_secs,
  interval_secs, enabled, channel_ids uuid[], next_eval_at, created_at}`. Unique `(tenant_id,
  name)`.
- `nexus_alert_rule_state` — `{rule_id (pk, fk), tenant_id, state, since, last_eval_at,
  last_value}`. One row per rule; the state machine's memory.
- `nexus_alert_events` — `{id, tenant_id, rule_id, at, transition (firing|resolved), value,
  silenced, notified, detail}`. The append-only history `/alerts/events` reads.
- `nexus_alert_channels` — `{id, tenant_id, name, kind, config jsonb, created_at}`.
- `nexus_alert_silences` — `{id, tenant_id, rule_id (nullable), starts_at, ends_at, reason,
  created_by, created_at}`.

Every table carries `tenant_id`, `FORCE ROW LEVEL SECURITY`, the `app.tenant_id` policy, and the
runtime-role grant — the established pattern. Grants/refs key on the immutable rule id.

## REST surface (under `/api/v1`, behind the principal layer; gated on the `nexus.alert_rule` kind)

- `GET/POST /alerts/rules`, `GET/PUT/DELETE /alerts/rules/:id` — rule CRUD (view/edit/delete).
- `GET /alerts/events` — the firing/resolved history, tenant-scoped, newest first.
- `GET/POST /alerts/channels`, `DELETE /alerts/channels/:id` — notification targets.
- `GET/POST /alerts/silences`, `DELETE /alerts/silences/:id` — maintenance windows.

Handlers stay ≤20 lines (R10): extract → store/evaluator call → shape DTO → return. The evaluator
and state machine live in their own module (engine-side logic, not in a route file).

## Where the code goes

- **State machine + evaluator + scheduler**: a new `nexus-engine`-adjacent concern, but it needs
  the store (to read rules/state) and the query path — both of which live in `nexus-store`, and
  the evaluator must not pull the store into the engine (R2 layering). So the **evaluator and
  scheduler live in `nexus-api`** (the binary that already composes store + engine), under
  `src/alerting/` (`evaluate.rs`, `transition.rs` the pure state machine, `schedule.rs` the
  tick loop, `notify/` the channel dispatch). The **pure state-machine transition** is a free
  function with no I/O, unit-tested in isolation. The **store** (`nexus-store/src/alert/`) owns
  the tables. DTOs in `nexus-spi/src/dto/alert/`.
- This keeps R2 intact: the state machine is pure, the evaluator orchestrates store + the
  existing guarded query, and nothing new is forced into `nexus-engine`.

## What v1 deliberately defers

- Multi-node evaluation (leader election / shared queue) — single-node now, `FOR UPDATE SKIP
  LOCKED` already written so the upgrade is small.
- Conditions beyond single-scalar-vs-threshold (multi-series, ranges, no-data-as-alert policy
  toggle) — the operator enum and result handling are add-only.
- Channel kinds beyond webhook — the `Notifier` trait + `kind` enum make these additive.
- Notification retry/backoff queues — v1 logs a channel failure on the event; a durable retry
  queue is a later hardening, not a v1 correctness requirement.
