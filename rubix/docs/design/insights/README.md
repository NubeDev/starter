# INSIGHTS — rules that fire side effects after a tool returns

Insights are the rubix-side hook for "after a tool returns, did
its result cross a threshold?" The v0 surface is a single
hardcoded Rust check in the disk verb; future rules lift into
`starter-insights::RuleRegistry` and load from `rule.rhai` files
(see "promotion trigger" below).

## The v0 hardcoded rule

`rubix-tools::system::disk::run_insights_gate` runs after every
successful probe. It is the literal `if response.percent_used > 90
{ alert_send::dispatch(...).await? }` — no rule engine, no rhai
sandbox, no rule registry. The whole point is to keep the gate's
contract testable today so the dispatch boundary is settled
before the rule engine arrives.

The gate dispatches `rubix.alert.send` with severity `Error` and a
`rubix.system.disk.full` diagnostic. The alert sink in v0 is the
local tracing pipeline; downstream channels (Slack, Telegram,
email) defer to the alert-sink wiring tracked in
`docs/design/audit/`.

The integration test in `rubix-tools/tests/
system_disk_insights_test.rs` asserts the gate fires exactly once
on a 95%-used response and zero times on a 50%-used one. The
counter is the process-wide `alert_send::dispatched_count()` and
the assertion is a delta so other tests in the same binary can
run concurrently without interfering.

## Why a hardcoded `if` for v0

Two reasons:

1. **One rule does not justify a rule engine.** The cost of
   wiring `starter-insights::RuleRegistry` (DSL parsing, sandbox,
   per-rule audit) is meaningful; the value lands the moment a
   second rule appears. v0 only has one rule.
2. **It pins the dispatch boundary now.** The shape
   `Diagnostic + AlertSeverity → alert_send::dispatch` is what
   the rule engine eventually invokes. Settling that shape against
   the v0 caller means the migration is "register rules + replace
   the `if`," not "rebuild the alert plumbing."

## Promotion trigger to `rule.rhai`

The second rule. The moment any verb beyond `system.disk` needs a
threshold check, the hardcoded `if` lifts into the registry:

- `starter-insights::RuleRegistry::register` accepts the existing
  rule definitions, including the v0 `disk_used > 90` ported into
  a one-line rhai expression.
- The post-dispatch hook in each verb becomes a single call to
  `RuleRegistry::evaluate(verb_id, response_json)`, returning the
  list of rules that fired.
- Each fired rule calls `alert_send::dispatch(...)` with the same
  `(AlertSeverity, Diagnostic)` it does today — the dispatch
  contract does not change.

The TODO comment in `rubix-tools::system::disk::run_insights_gate`
carries the exact `(upstream: rule.rhai migration)` qualifier the
lint pattern searches for so the migration cannot be forgotten.
