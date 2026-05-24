## Done

- reviewed Phase B commits 68b60c6 (B.1 scaffold) and 0783882 (B.2 tick + start)
- verified SELECT FOR UPDATE SKIP LOCKED LIMIT 32 with in-tx next_run_at rewrite, dispatch outside tx
- verified tests/scheduled_flows_tick_test.rs uses TestClock for determinism (2-minute advance → 2 fires, 3rd tick at same clock claims 0)
- verified Layer-1 invariants (R1/R2/R4/R5) untouched

## Next

- FAIL: the 60s tick interval is hardcoded at service.rs:401 (Duration::from_secs(60)); no constructor/builder option (only with_clock exists). Add a with_tick_interval(Duration) builder + a field on FlowAsService, then re-run the gate.

## What you need to know

- FAIL: tick interval is not configurable — Phase B gate requires the 60s default be settable via a constructor option, and only `with_clock` exists on FlowAsService.
- Other gate criteria pass: both Phase B commits named, SKIP LOCKED is correct, TestClock determinism shown by the tick test.
- No code changes made this stage (gate stage; patches deferred per instructions). Nothing to commit.

## Open questions

- (none)
