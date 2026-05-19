## Done

- Pinned the starter-spi transitive-dep baseline at `DOCS/tools/scope/starter-spi-deps.baseline.txt` (generated via `cargo tree -p starter-spi --edges normal --prefix none | sed 's/ (\*)//' | sort -u`, header documents the command).
- Appended a `## Decisions` section to `DOCS/tools/scope/SCOPE.md` recording D1–D4 with revisit triggers, plus a dated decisions log entry.
- Committed as `Stage 1: resolve implicit design points …` on `codeless/starter-tools-services`.

## Next

- Stage 2 picks up: implement `Service`, `ServiceRegistry`, `ServiceContext`, `ServiceHandle`, `EventSink`, `SinkError`, `SHUTDOWN_DEADLINE_DEFAULT`, the broadcast blanket impl (default-on feature), and the `secrecy::SecretString` re-export in `crates/starter-spi`. After landing, re-run `cargo tree -p starter-spi …` and update the baseline in the same commit (D1).

## What you need to know

- Decision summary: (D1) baseline only changes when starter-spi changes; (D2) blanket EventSink-for-broadcast::Sender behind default-on `broadcast` feature in starter-spi; (D3) `SHUTDOWN_DEADLINE_DEFAULT = 5s` const + `shutdown_with_deadline`; (D4) `SpiResult<(), SinkError>` with `Saturated`/`Closed`/`Other`, fan-out logs Other/Closed and bubbles Saturated.
- The baseline file currently includes `secrecy`-free deps. When Stage 2 adds `secrecy` re-export and `tokio` `sync` already covers broadcast, expect new transitive deps; regenerate the baseline in that same commit.
- `SpiResult` arity in current starter-spi is single-parameter — Stage 3/wherever `EventSink` lands needs to either extend the alias or introduce a sibling alias (called out in D4).
- Existing crates list at `crates/` confirms naming pattern; no provider crates exist yet.

## Open questions

- Whether to strip workspace-absolute paths in the baseline comparison or normalize them in CI — punted to whoever wires the smoke test into CI.
- Final name of the result alias for `EventSink::emit` (extend `SpiResult` to two params vs. add `SpiSinkResult`) — deferred to Stage 2/3 when the trait actually lands.
