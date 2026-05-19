# Workflow — starter-tools-services

How to drive this job. The shape is "extend `starter-spi` with a
small, surgical set of types, then ship three integrations as
sibling crates on top — mining the codeless workspace for working
implementations, not architecture."

## Sequencing

- Stage 1 is **prose-only**. Pin the four small design points in
  [SCOPE.md](./SCOPE.md), record under "Decisions", commit. No
  code.
- Stage 3 (Phase 1 — `starter-spi` additions) lands first because
  every provider crate depends on the new types. Land it as one
  commit so the SemVer signal is clean; the `starter-spi-deps.
  baseline.txt` file ships in the same commit so the dep-leakage
  smoke test is enforced from day one.
- Phase 2 (Slack) is the heaviest provider phase because it ships
  both an outbound `Tool` crate and an inbound `Service` crate.
  Land the tool first (no lifecycle, easier), then the service
  on top.
- Phases 3 and 4 (Telegram, Gmail send-only) reuse the patterns
  set in Phase 2; land them in either order after Phase 2's
  REVIEW. Telegram has a service half mirroring Slack; Gmail is
  tool-only in v0.1.
- Stage 9 (smoke tests) is the merge gate. No phase ships
  individually without its own subset of the five design tests
  passing; the full sweep gates the final merge.

## Per-stage discipline

- Before any code change in a phase:
  - `git log -20 --oneline` for the surrounding history.
  - Re-read the rule numbers in [SCOPE.md](./SCOPE.md) that the
    stage touches. R1, R2, R4, R5, R8 are the load-bearing ones;
    if a change makes any of them harder to enforce, stop and
    write up the conflict.
  - For provider phases (2/3/4), read the relevant codeless crate
    (`codeless/crates/codeless-slack`, `codeless-telegram`,
    `codeless-tools/src/email/gmail.rs`) before writing anything.
    The architecture there bundles concerns this scope splits —
    lift the **HTTP / socket / message-construction code**, not
    the surrounding `codeless-bot-core` shape.
  - For Phase 1, re-read the source SCOPE's
    §"What lands in `starter-spi`" — the snippet there is the
    spec, byte-for-byte.
- Touch only what the stage names. No drive-by refactors.
- Verify before commit:
  - **Rust**: `cargo check --workspace --all-features --all-targets`,
    then `cargo test -p <touched crate>`, then
    `cargo clippy --workspace --all-targets -- -D warnings`.
  - **Dep baseline**: every stage that touches `starter-spi` runs
    `cargo tree -p starter-spi --edges normal` and diffs against
    `DOCS/tools/scope/starter-spi-deps.baseline.txt`. A diff
    means the change is in scope for an SPI bump (intentional and
    reviewed) or in error.
  - **Dep leakage in providers**: every provider stage runs
    `cargo tree -p starter-spi --edges normal` after building the
    provider crate and confirms none of the provider's deps
    appear. If they do, the provider has bled into `starter-spi`.
  - **Smoke harness**: every provider stage runs the five design
    smoke tests against the new crate (dep-leakage,
    no-special-case-wiring, config-guarded construction,
    secrets-backend-swappable, shutdown-actually-shuts-down).
    A stage is not done until all five pass for the new crate.
- Commit only if green. One logical batch per commit; commit
  message stage-tagged: `stage N: <one-line title>`.

## REVIEW gates

Two:

- **After stage 1** — decisions sign-off before any code lands.
  The four design points (baseline path, broadcast feature gating,
  shutdown deadline constant, EventSink error shape) carve out
  the SPI surface; locking them down first is cheap.
- **After stage 4** — `starter-spi` surface frozen. Phase 1 has
  landed; the three provider phases (2/3/4) must not feed back
  into SPI shape changes. If a real need surfaces during a
  provider phase, stop and propose a kernel change explicitly,
  do not back-door it through a provider.

Write a one-line summary into the handover at each gate. Do not
proceed.

## What "done" looks like per stage

| Stage | Done when |
|---|---|
| 1 | SCOPE.md "Decisions" section filled in for the four design points; no code changed. |
| 3 | `starter-spi` compiles with the new types + `SecretString` re-export + blanket `EventSink` impl + fan-out helper; `DOCS/tools/scope/starter-spi-deps.baseline.txt` committed; round-trip unit tests cover the `ServiceRegistry` shutdown fan-out and the `EmitError::Saturated` bubble from the fan-out helper. |
| 5 | `starter-tool-slack` `chat.postMessage` round-trips against `wiremock`; success / 429 / 5xx / auth-failure paths all covered; latency histogram + error counter visible on the consumer's registry; README mirrors `examples/notes` "how it's extended" shape. |
| 6 | `starter-service-slack` opens the socket-mode WSS against a `tokio-tungstenite` test server, emits a deserialized event via `ctx.sink` as `slack.<event_type>`, and exits within `SHUTDOWN_DEADLINE_DEFAULT` of flipping `ctx.shutdown`. Restart counter + event-emitted counter + is-running gauge registered. Restart policy lives in the service's own retry layer per R9. |
| 7 | `starter-tool-telegram::TelegramSendMessageTool` round-trips against `wiremock`; `starter-service-telegram::TelegramBotService` long-polls `getUpdates` with offset tracking and emits each update as `telegram.<update_type>`; both pass their dep-leakage and shutdown checks. |
| 8 | `starter-tool-gmail::GmailSendTool` round-trips `users.messages.send` against `wiremock`; happy + 401 + 5xx covered; `GmailConfig.oauth_access_token: SecretString` works with both `starter-secrets-file` and `starter-secrets-keyring` consumer-side (secrets-backend-swappable smoke passes); inbound Gmail is explicitly *not* implemented. |
| 9 | Five design smoke tests pass in CI for every shipped provider crate: dep-leakage-vs-baseline, no-special-case-wiring, config-guarded construction, secrets-backend-swappable, shutdown-actually-shuts-down. CI gates the dep-baseline check (a baseline diff fails the build). |

## Anti-patterns

- A `starter-tools` mega-crate with cargo features per provider.
  R1 — cargo feature unification would leak Slack's websocket
  deps into every consumer that depends on the mega-crate, even
  ones that only use Gmail. Per-crate is the contract.
- Collapsing `Tool` and `Service` into one trait. R2 — they have
  different shapes: `Tool` has a caller and returns a value;
  `Service` has no caller and publishes into an `EventSink`.
  Forcing one trait makes every implementer leak one half.
- Pattern-matching on provider event payloads inside a service
  beyond what's needed to deserialize. R4 — services emit
  events; consumers decide what they mean. A Slack message
  detector belongs in the consumer's domain layer, not in
  `starter-service-slack`.
- Reading env vars or secrets files inside a provider crate.
  R5 — credentials arrive via the `Config` struct as
  `SecretString`; the consumer's `starter-secrets-*` resolves
  them. The secrets-backend-swappable smoke test catches this.
- Importing `secrecy::SecretString` in a provider crate. R5 —
  use `starter_spi::SecretString` (re-exported). A CI grep
  rejects direct `secrecy` imports outside `starter-spi`.
- Auto-restarting a service inside `ServiceRegistry`. R9 — the
  registry is a lifetime manager, not a supervisor. Restart
  policy lives in the service's own retry layer (or a
  consumer-built `RestartingService<S>` adapter); the registry
  records the error and increments the restart counter and stops
  there.
- A provider service holding the registry's
  `watch::Sender<bool>`. R2 — the registry owns the sender; the
  service receives a `Receiver` via `ServiceContext.shutdown`.
  Letting a service hold the sender means one service can shut
  down the others.
- Adding a vendor SDK dep to `starter-spi`. R8 — the
  dep-baseline file is enforced in CI. A diff means either the
  baseline updates (a separate, reviewed commit) or the change
  is rolled back.
- A provider crate calling into `ServerBuilder` directly. R6 —
  it hands back a `Router<S>` and the consumer composes.
  Provider crates do not mount their own routes; the consumer's
  `main.rs` does.
- A new metric registration mechanism inside a provider crate.
  R7 — every provider registers against the `prometheus::
  Registry` passed in via `McpHttpOptions` (for tools) or
  `ServiceContext.metrics` (for services).
- An outbox / retry queue / scheduler inside this scope.
  Explicitly out of scope; durable scheduling gets its own
  crate when a real consumer asks.
- Lifting `codeless-bot-core` architecture into the provider
  crates. The codeless workspace is a source of working
  implementations (HTTP calls, websocket handlers, message
  construction); its architecture predates this scope and
  bundles concerns differently. Lift the bytes, not the shape.

## Closing trio — the last three todos of every stage

Every stage's todo checklist ends with the same three items, in
order. The user watches these tick over in the `Stages` overview;
they are how the user confirms a long-running stage actually
landed instead of just looking like it did. Do **not** rename or
reorder them.

1. `checks` — run the stage's verify list. Every step must pass.
   On failure: stop, fix, re-run; do not advance to `docs`.
2. `docs` — update `handover.md` for the next stage and the active
   session doc, in the same worktree, so the fresh agent that opens
   the next stage has the context it needs.
3. `git` — stage the changes, commit with the message
   `stage N: <one-line title from template.yaml>`, and push to the
   job's branch (`codeless/starter-tools-services`).

A stage is not "done" until all three are green and the push
succeeds. Never `--force`, never `--no-verify`; if a hook fails,
fix the cause.
