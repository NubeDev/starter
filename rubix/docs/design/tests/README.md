# TESTS — R10 in full

> Cites: SCOPE [R10](../../SCOPE.md#r10).

## The two-layer convention

- **Unit tests** live inline as `#[cfg(test)] mod tests { ... }` at
  the bottom of the file they cover. Pure functions, no I/O, fast.
- **Integration tests** live in the crate's `tests/` directory and
  mirror `src/` one-to-one. `tests/disk_usage_test.rs` tests
  `src/system/disk.rs`. Database / network / agent-loop tests
  belong here.

No third convention. No `#[cfg(test)] mod tests` blocks in
`tests/` files; no integration tests inside `src/`.

## Integration test infrastructure

- HTTP: `starter-server::testing` harness — never hand-rolled
  servers.
- Postgres: testcontainers pattern from `starter-store-postgres`.
- ClickHouse: testcontainers pattern from `starter-store-warehouse`.
- Agent loop: **recorded-LLM-response harness** (planned upstream
  — see [STARTER-CHANGES.md](./STARTER-CHANGES.md)). Live LLM
  calls in CI are unaffordable; per-PR tests use recorded
  fixtures; live nightly runs validate the recordings.

## Required tests per tool (R10)

Each rubix tool ships:

1. One unit test inline for any pure logic (input validation,
   shape transforms).
2. One integration test under `tests/<tool>_test.rs` round-tripping
   through the `ai-agent` node loop using recorded-LLM fixtures.

## Required tests per goal

Each goal ships at least one bundled-flow round-trip integration
test that exercises the flow YAML end-to-end through MCP.

## What does NOT count

- A test that mocks `AiRunner` to a no-op and asserts "the agent
  was called" is not an agent-loop test. It's a tool-handler test
  in disguise.
- A test that hits a live LLM in CI fails CI configuration review.
- A test in `tests/` that does no I/O is a misplaced unit test.

## Why this works

Per HOW-TO-CODE.md Rule E, **tests live with the code in the same
PR**. The integration-test path mirrors the source path, so finding
the test for `heartbeat.rs` is trivial. No "test sprawl" wiki page
to consult.
