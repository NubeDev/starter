## Done

- reviewed Phase A diff (3 commits: ef9dd9f starter-cron, c94076a scheduled_flows PG, 3f8c109 trigger_schedule body)
- ran cargo test -p starter-cron (4 unit + 1 doctest pass)
- ran cargo test -p starter-flow-nodes --lib trigger_schedule (4/4 pass)
- ran cargo test -p starter-store-postgres --features testing --test scheduled_flows (2 testcontainer cases compile + register; ignored under no-docker sandbox — sqlx::migrate! validates SQL at compile time, crate builds clean)
- confirmed Layer-1 invariants hold: R1 (starter-cron is leaf; no new upward edges from store-postgres or flow-nodes), R2 untouched, R4/R5 untouched, wire formats untouched
- SCOPE OQ-1: starter-cron landed as its own crate (crates/starter-cron/), not folded into starter-flow-spi — keeps cron/chrono out of the SPI

## Next

- Phase B: durable scheduler tick loop in starter-flow-surfaces (FlowAsService::tick + LISTEN starter_scheduled_flows + register_schedule API)

## What you need to know

- trigger_schedule node is intentionally passive — it only forwards cron_expr to the schedule slot; firing is host-driven and lands in Phase B
- the docker-gated PG tests (scheduled_flows_notify_fires_on_insert_and_update, scheduled_flows_unique_tenant_flow_enforced) should be re-run in CI / on a docker-enabled host before Phase B closes — sqlx::migrate! compile-time check is not a substitute for a live apply
- starter-cron expects 6/7-field cron expressions (sec min hour dom mon dow [year]); the SCOPE example "0 0 * * 0" is a 5-field POSIX form — Phase B's register_schedule must either normalise or document this; raised as a Phase B carry-over

## Open questions

- 5-field vs 6/7-field cron grammar mismatch between SCOPE example strings and starter-cron parser — decide in Phase B (normalise on input, or update SCOPE examples)

PASS: Phase A landed three commits with green tests, kept starter-cron as a leaf crate per OQ-1, and touched no Layer-1 invariants (deps, transport, trust boundary, wire formats).
