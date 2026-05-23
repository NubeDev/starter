# TESTS — R11 in full

> Source: `rubix/SCOPE.md` R11 ("Tests live with the code"),
> §"Testing strategy", "Decisions made" (R1 enforcement bullet).
> Cross-refs: `MIGRATIONS.md` (testcontainers run migrations
> before each test), `rubix/docs/testing/SETUP.md` (docker
> prerequisites for testcontainers).

## The rule

> **Tests live with the code, in the same PR, in the same diff.
> `tests/` mirrors `src/` one-to-one. A new behaviour without a
> sibling test is a CI failure, not a "follow-up PR."**

R11 is a **presence** gate, not a **coverage** gate. There is no
percentage threshold to block PRs at this time (see §"Coverage" at
the bottom). The presence rule alone has paid for itself by
catching API drift and uncaught regressions early enough that the
expensive percentage gate hasn't been needed.

## File naming — 1:1 mirror

Rust:

```
src/heartbeat.rs               tests/heartbeat_test.rs
src/manifest.rs                tests/manifest_test.rs
src/point_writable.rs          tests/point_writable_test.rs
```

TypeScript:

```
src/useNode.ts                 src/useNode.test.ts
src/SduiRenderer.tsx           src/SduiRenderer.test.tsx
```

Dart:

```
lib/auth.dart                  test/auth_test.dart
lib/devices.dart               test/devices_test.dart
```

The pairing is mechanical so a contributor or an AI assistant
opening a file knows exactly where its tests are. **No
test-namespace files** like `tests/lib.rs` or `tests/common.rs` —
each test file is named for the source file it exercises.

Helpers shared across multiple test files live in a
`tests/support/` (Rust) or `src/__testing__/` (TS) directory and
follow the same naming rule (`tests/support/devices_fixture.rs`,
not `tests/support/utils.rs`).

## The three test tiers

### 1. Unit tests — pure, fast, no I/O

Live `#[cfg(test)] mod tests` next to the function in `src/` (Rust)
or alongside the source as `.test.ts` (TS). No database, no
network, no filesystem. Run in milliseconds.

Use unit tests for:

- Pure helpers (parse, format, compute).
- `KindManifest` shape (serialisation round-trips,
  `utoipa::ToSchema` snapshot).
- Slot-value type coercion.
- Authorisation rule evaluation (given principal + rule → permit/deny).

If a function needs a database to test, it should not be a pure
helper — push the I/O up to the caller.

### 2. Integration tests — testcontainers, `#[ignore]`, real store

Live in `tests/` (Rust integration test convention) or `tests/`
(TS Vitest convention). Use **testcontainers** to bring up Postgres
(and ClickHouse when the test touches the warehouse). Marked
`#[ignore = "needs-docker"]` (Rust) or `.skip(...)`-gated by env
var (TS) so they only run in the CI `RUBIX_E2E=1` lane or when an
operator deliberately runs them locally.

The testcontainer seam:

```rust
use starter_store_postgres::testing::with_database;

#[tokio::test]
#[ignore = "needs-docker"]
async fn devices_table_apply_smoke() {
    with_database(|pool| async move {
        // Migrations have already been applied by the harness.
        let row = sqlx::query!("SELECT 1 as one")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(row.one, Some(1));
    }).await;
}
```

`with_database` (in `starter-store-postgres::testing`) starts a
Postgres testcontainer, applies every registered migration source
in the boot order described in `MIGRATIONS.md`, and hands the
closure a connected pool. Tear-down is automatic.

For ClickHouse there's a parallel fixture in
`starter-store-clickhouse::testing`; `rubix/agent/crates/
data-clickhouse` exposes a smoke test that walks the same shape
(already wired in stage 2 of Phase 0).

**Always** go through the `testing::with_*` seam. Hand-rolling a
test that calls `sqlx::PgPool::connect(env::var("DATABASE_URL"))`
bypasses the per-source migration runner and you'll catch
ordering bugs only in production.

### 3. Transport tests — `TestApp::spawn`, real handlers

Live in `tests/` of the transport crate. Use
`starter-server::testing::TestApp::spawn()` — random local port,
real handlers, real domain functions, real database
(testcontainer-backed). Drive through `rubix-agent-client` so the
test exercises the client surface at the same time.

```rust
#[tokio::test]
#[ignore = "needs-docker"]
async fn rest_creates_device_returns_201() {
    let app = TestApp::spawn().await;
    let client = rubix_agent_client::Client::new(app.url(), app.token());

    let dev = client.create_device("My Device", tenant_id).await.unwrap();
    assert_eq!(dev.name, "My Device");

    let listed = client.list_devices().await.unwrap();
    assert!(listed.iter().any(|d| d.id == dev.id));
}
```

Transport tests are the canonical Phase 1+ acceptance gate. The
"Swap REST for gRPC" smoke test (OVERVIEW.md) is checked at PR
time by running the same scenario against the gRPC transport;
disparity in behaviour fails the test.

## Block / extension tests

For a third-party block, tests live in `extensions/<id>/tests/`.
Spawn the block process via the supervisor in-test and drive it
through `rubix-agent-client`:

```rust
#[tokio::test]
#[ignore = "needs-docker"]
async fn mqtt_block_publishes_messages_as_points() {
    let app = TestApp::spawn().await;
    let supervisor = ExtensionSupervisor::new(&app);
    supervisor.install("com.rubix.mqtt-client").await.unwrap();

    let client = Client::new(app.url(), app.token());
    let topic_node = client.find_node("/mqtt/test/topic").await.unwrap();
    // Publish via an external broker fixture; assert the topic node's
    // value slot updates.
}
```

The supervisor + block-process pair is exercised end-to-end. Mocks
at the supervisor seam are not allowed — the same ADR-001 reasoning
applies (mock/prod divergence costs more than the testcontainer
boot).

## SPI conformance tests — the contract-test pattern

`rubix-spi` exposes traits (e.g. `Authenticator`, `ArtifactStore`,
`SecretStore`, future `FleetTransport`). For each trait, ship a
**contract test** in the SPI crate that any implementor can drive
against their impl:

```rust
// rubix-spi/src/secret_store/contract.rs
pub async fn run_contract_suite<S: SecretStore>(store: &S) {
    contract_set_and_get(store).await;
    contract_rotate(store).await;
    contract_missing_returns_error(store).await;
    contract_concurrent_reads(store).await;
}
```

Each implementing crate (e.g. `starter-secrets-file`,
`starter-secrets-keyring`) imports the suite and runs it from its
own integration test. New impls are conformant by default; missing
behaviours surface as test failures at the impl site, not as silent
divergence in production.

The contract suites live in `rubix-spi` so the contract is the
single source of truth (R5). Adding a new requirement to a trait
means adding a test to the suite, not editing every implementor.

## In-memory paths for unit tests

For unit tests that need a "store" but shouldn't pay testcontainer
boot costs, `rubix-spi` ships **in-memory impls** of the major
traits:

- `InMemoryGraphStore` — a `HashMap`-backed slot store; sufficient
  for unit tests of `NodeBehavior`s.
- `InMemorySecretStore` — for tests of code that reads secrets.
- `InMemoryArtifactStore` — for tests of the artifacts domain crate
  that don't need a real S3.

These impls are **for tests only**; they live behind a `testing`
cargo feature in `rubix-spi`. The `cfg(test)` machinery and the
`testing` feature combine to keep the in-memory paths out of
production binaries.

**Never** use an in-memory store as the production default for a
trait that has a Postgres impl. The mock/prod divergence trap is
the same as the data layer's; ADR-001's reasoning extends.

## Mocks — what's allowed, what isn't

**Allowed:** in-memory impls of SPI traits (above). They are
*alternative implementations*, not mocks of behaviour-under-test.

**Allowed:** mocks of *external systems* the rubix process talks to
— an MQTT broker fixture, a fake S3, a stubbed Zitadel IdP. The
boundary being mocked is outside the rubix codebase.

**Forbidden:** mocks at the data layer (`PgPool`, `ClickHouseClient`).
ADR-001's "burned by mock/prod divergence" applies — tests run
against real Postgres / ClickHouse via testcontainers, every time.

**Forbidden:** mocks of `starter-flow`'s propagator or
`graph::GraphStore` for tests of a `NodeBehavior`. Use the
`InMemoryGraphStore` instead — it implements the real trait, exercises
the real slot-write chokepoint (R3), and surfaces propagator
contract bugs early.

## CI lanes

Two lanes:

| Lane | Trigger | What runs | Wall-clock target |
|---|---|---|---|
| **Fast** | every push | `cargo test --workspace` (unit + non-ignored), `pnpm test`, `flutter test`, `mani run lint` | < 5 min |
| **E2E** | `RUBIX_E2E=1` or scheduled | the fast lane PLUS `cargo test -- --ignored`, transport tests, extension tests | < 30 min |

The Phase 0 testcontainer smoke tests (`rubix-data-postgres` and
`rubix-data-clickhouse`) sit in the E2E lane. PRs that add a new
domain crate also add new E2E tests; the E2E lane is the merge gate
for those PRs.

A failing fast lane blocks the PR immediately. A failing E2E lane
blocks the merge. We do not merge with a red E2E.

## Coverage

**No percentage gate.** R11 is the presence gate; the project's
philosophy is "if a function ships without a test, the PR doesn't
merge — but the *shape* of the test matters more than the
percentage."

A coverage drop in a PR is reviewed (does the diff remove tests
without removing the code they exercised?) but not auto-blocked. If
practice shows we need a numeric gate later, adding one is easy;
removing one once it's institutional is hard. Open question in
SCOPE.md "Open questions".

## R1 enforcement at lint time

`mani run lint` runs:

1. `cargo fmt --check` — formatting.
2. `cargo clippy --all-targets --all-features -- -D warnings` —
   lints.
3. `pnpm exec eslint .` — TS lint.
4. `dart analyze` — Dart lint.
5. **Line-count check** — every file under `rubix/` (excluding
   `target/`, `node_modules/`, `**/generated/`, `**/migrations/`)
   has ≤ 400 lines. Fail on any file over 400.
6. **Cross-tree-FK parser** — see `MIGRATIONS.md`.
7. **Comment-rot grep** — `// STAGE`, `// Phase`, `// FIXED`,
   `// Previously`, emoji banners, bare `// TODO` without an owner.
   Per R12.

Items 5–7 are rubix-specific; the script lives at
`rubix/scripts/check-file-size.sh` (and siblings) and is invokable
locally before push.

## Test-driven where possible

For a new behaviour: write the failing test first, in the same
file pair (R11), commit it, then make it pass. The TDD discipline
isn't enforced by CI but is the recommended default — bugs caught
by a red test before the implementation lands are bugs that
*never lived in main*.

For a bug fix: the regression test is mandatory. Land the
failing test and the fix in the same PR; the commit history shows
the cycle.

## Naming inside the test

Test function names describe the behaviour, not the function under
test:

```rust
// ✗ tests function name
#[test]
fn test_create_device() { … }

// ✓ describes the behaviour
#[test]
fn creating_a_device_returns_the_persisted_row() { … }

#[test]
fn creating_a_device_in_a_foreign_tenant_returns_403() { … }
```

A failing test message reads as a sentence: "creating a device in a
foreign tenant returns 403 — FAILED". An operator chasing a CI
failure understands the regression without opening the test file.

## What tests do NOT need to cover

- **Pure data structures with derived traits.** A
  `#[derive(Serialize, Deserialize, Debug, Clone)]` struct doesn't
  need a round-trip test per field; one round-trip per struct is
  enough.
- **Re-exports.** A `pub use` doesn't need a test that proves the
  re-export resolves; the compiler handles it.
- **Generated code.** `agent-client-ts/src/generated/` is verified
  by the codegen step; per-file tests would just shadow
  `mani run codegen`.
- **Trivial getters.** A `pub fn id(&self) -> Id { self.id }`
  doesn't need a sibling test; covered transitively by the test of
  the caller.

The R11 presence rule applies to **behaviour**, not to every line.
A 50-line file with one public function gets one test file with
one test; a 350-line file with five public functions probably has
ten test cases. The line-count check is per-file (R1), not per-test.

## Phase 0 exit expectation

Phase 0's testcontainer smoke tests (`rubix-data-postgres` and
`rubix-data-clickhouse`) pass against a running docker daemon. The
CI E2E lane runs them via `RUBIX_E2E=1`. Phase 0 ships no domain
tests because Phase 0 ships no domain code.

Phase 1 opens with a green E2E lane and the expectation that every
PR landing a `domain-*` crate adds matching transport and unit
tests in the same PR.
