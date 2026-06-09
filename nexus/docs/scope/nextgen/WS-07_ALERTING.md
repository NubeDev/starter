# WS-07 — Alerting: Multi-Condition, Channels, No-Data Policy, Timeline UI

> **Status:** Not started · **Wave:** 1 (mostly independent) · **Owner:** _unassigned_
> **Depends on:** nothing hard — extends the existing engine. Email/Slack channels are standalone.
> **Migration:** block `10xx` (e.g. `1001_alert_channels_v2.sql`, `1002_alert_routing.sql`) · **Read first:** GAP_ANALYSIS §2.7, ROADMAP §0, `docs/session/backend/ALERTING.md`
> **Verified:** `82a6a19a` on 2026-06-09 — re-grep this WS's file:line claims before building (ROADMAP §0).

## Goal
Take alerting from "solid but basic" to power-user grade — **without rewriting** the proven engine.
The state machine, scheduler, and dispatch are good; the design doc explicitly built the enums/traits
to be **additive**. This WS adds multi-condition rules, a no-data/error policy, real notification
channels (email/Slack), notification templating, and an alert timeline UI.

## Current state (evidence) — what's already good (extend, don't fork)
- ✅ 10s scheduler, `FOR UPDATE SKIP LOCKED` (`alerting/schedule.rs`).
- ✅ Pure state machine `Ok→Pending→Firing→Resolved`, dwell/`for_secs`, transition-only dedup
  (`alerting/transition.rs` — unit-tested, **do not rewrite**).
- ✅ Evaluator reuses the guarded query path (`alerting/evaluate.rs`).
- ✅ Silences (`alert/silence.rs`), event log (`alert/record.rs`).
- ⚠️ **Single scalar vs one threshold**; operators `gt/gte/lt/lte/eq/ne` (`alerting/compare.rs`).
- ⚠️ **Webhook only** (`alerting/notify/webhook.rs`); `Notifier` trait + `kind` enum ready for more.
- ⚠️ **No-data = non-breaching**, no override (ALERTING.md "deferred").
- UI is a textarea + operator + threshold (`features/alerts/AlertRuleDialog.tsx`); channels are a
  free-text `kind` + opaque JSON config (`ChannelsTab.tsx`).

## Scope
1. **Multi-condition rules** — a rule becomes a list of conditions combined with AND/OR:
   each condition = `{ query, reducer (last/min/max/avg/count/sum), op, threshold }`. Extend the
   rule DTO + store; the evaluator runs each condition and combines per the boolean. Keep the
   single-condition shape as the 1-element case (back-compat). Reducers operate over the result set
   (not just first row).
2. **Per-series evaluation** (stretch) — when a query returns multiple series (a label column),
   evaluate per series and fire per breaching series; event carries the series labels.
3. **No-data / error policy** — per-rule toggle: `no_data = ok | alerting | keep_last`;
   `exec_error = ok | alerting | keep_last`. Wire into the evaluator + state machine inputs
   (the transition fn stays pure; we feed it the right `breaching` derived from policy).
4. **Notification channels v2** — implement, behind the existing `Notifier` trait:
   - **Email** (`notify/email.rs`) — SMTP config; HTML+text body.
   - **Slack** (`notify/slack.rs`) — incoming-webhook or bot token; blocks formatting.
   - Keep **webhook**. (PagerDuty/OpsGenie = follow-up.)
   - `1001_alert_channels_v2.sql` (WS-07 `10xx` block): typed per-kind config; UI channel forms per kind (replace the
     free-text `kind` + raw JSON with a kind picker + schema-driven form).
5. **Notification templating** — message templates with `{{rule_name}} {{value}} {{threshold}}
   {{state}} {{labels}}`; per-channel default + override. Safe rendering (no injection into webhooks).
6. **Delivery retry/backoff** — a bounded retry on channel failure (the doc defers a durable queue;
   do at least in-memory retry-with-backoff + record attempts on the event). Durable queue = stretch.
7. **Alert UX** — an **alert list + timeline** view: current state per rule, state-history from the
   event log, last value vs threshold, silence-from-here. "**Create alert from panel**" — a button on
   a dashboard panel that pre-fills a rule from the panel's query (couples lightly with WS-04).
8. **Notification policies / routing** (phase 2) — route by tag/label/severity to channel groups,
   grouping + throttling. Can be a follow-up session.

## Design notes
- **The transition fn stays pure and untouched.** Multi-condition + no-data policy resolve to a
  single `breaching: bool` (and per-series set) *before* calling `step()`. Add tests for the
  resolution, not the state machine.
- **Channels are additive** via the `Notifier` trait — email/slack are new files implementing it;
  the dispatch fan-out (`notify/mod.rs`) is unchanged structurally.
- **Reuse query guards** — multi-condition queries run under the same read-only/timeout/cap path.
- **Secrets** for SMTP/Slack tokens go through the **existing envelope-encryption** model, not plaintext.

## Acceptance criteria
- [ ] A rule with two conditions combined by AND fires only when both breach; back-compat single
  condition still works.
- [ ] No-data policy `alerting` fires on an empty result; `ok` doesn't (toggle respected).
- [ ] Email and Slack notifications deliver on `→firing` and `→resolved`; webhook still works.
- [ ] Channel config uses a per-kind form; tokens stored encrypted, never returned.
- [ ] Templated message renders rule/value/threshold/state correctly and safely.
- [ ] Failed delivery retries with backoff and records attempts on the event.
- [ ] Alert timeline UI shows state history; "create alert from panel" pre-fills a rule.
- [ ] Tests: condition resolution, no-data/error policy, each channel (mocked), template rendering.

## Out of scope (hand off)
- The pure state machine (don't touch it) — extend inputs only.
- PagerDuty/OpsGenie + full routing/grouping → phase-2 follow-up (note as a gap).
