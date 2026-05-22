# Workflow — flow-nodes

How to drive the stages in `template.yaml`. Read this before every
stage alongside `SCOPE.md` and the authoritative source SCOPE at
[/home/user/code/rust/starter/DOCS/extensions/scope/FLOW-NODES.md](/home/user/code/rust/starter/DOCS/extensions/scope/FLOW-NODES.md).

## Sequencing

Two stages, one REVIEW gate between them. Strictly linear:

- Slice A freezes the manifest shape, the registry composition, and
  the descriptor surface. Without that frozen, slice B's
  `ProcessNodeProxy` has nothing to plug into.
- Slice B replaces slice A's "no behaviour bound" placeholder with
  the real `ProcessNodeProxy` and lands the reload algorithm + MQTT
  demo + acceptance tests.

The REVIEW gate exists because slice A's `NodeDescriptor` Cow
widening is the riskiest mechanical change in the job — if a
built-in `NodeDescriptor::new` site fails to compile after the
widening, every later piece of work stalls. Catch it at the gate.

## Per-stage discipline

Before writing any code in a stage:

1. Re-read the corresponding slice in the source SCOPE. The slicing
   text is the contract; this WORKFLOW is the process.
2. Re-read `SCOPE.md` §"In scope" and §"Out of scope". The biggest
   risk on this job is silent scope creep — the source SCOPE
   explicitly carves out tempting features (WASM, supervisor
   groups, `validate_settings` hook). Stay within the carve-outs.
3. For slice A: `grep -rn 'NodeDescriptor::new' crates/starter-flow-nodes`
   to enumerate every built-in call site that the `Cow` widening
   touches; confirm the count matches what the source SCOPE
   expects (~13 sites). If the count is wrong, surface before
   editing.
4. `cargo check --workspace` from the `starter` repo root before
   any edit so the baseline is known-clean.

Before committing a stage:

1. `cargo fmt --all` clean.
2. `cargo clippy --workspace --all-features -- -D warnings` green.
3. `cargo test --workspace` green.
4. For slice A: the fixture `block.yaml` in
   `starter-ext-flow/tests/fixtures/` round-trips through
   `contributed_node_kinds()`; `GET /api/node-kinds` against
   `flow-agent` returns the fixture kind with resolved i18n labels.
5. For slice B: every bullet in `SCOPE.md` §"Deliverables" §6 is
   green. The MQTT acceptance test runs against a `mosquitto`
   container and passes.

Commit + push via **mani** from the codeless-workspace root:

```
./bin/mani --config mani.yaml run commit --projects starter \
  MSG='stage N: <one-line title from template.yaml>'
./bin/mani --config mani.yaml run push --projects starter
```

No `--force`, no `--no-verify`. If a hook fails, fix the cause.

## Closing trio — the last three todos of every stage

1. `checks` — `cargo fmt --check` + `cargo clippy --workspace
   --all-features -- -D warnings` + `cargo test --workspace`.
   Slice B also runs the MQTT acceptance test against the
   `mosquitto` container.
2. `docs` — update `handover.md` for the next stage, tick the
   relevant `[x]` in `SCOPE.md` §"Deliverables". Module docstrings
   in any new code cite the relevant `R-flow-node-N` rule numbers
   from the source SCOPE.
3. `git` — stage the changes, commit with `stage N: <title>`, push
   to `codeless/flow-nodes`. One slice, one commit.

A stage is not "done" until all three are green and the push
succeeds.

## REVIEW gate (one, between slice A and slice B)

At the gate write a handover comment containing:

- The `cargo check --workspace` transcript proving the
  `NodeDescriptor` Cow widening compiles everywhere built-in
  sites use it.
- The reverse-DNS validator rejection transcript: a fixture
  extension contributing `starter.flow.mqtt` is rejected with the
  reserved-prefix error; a fixture extension contributing
  `com.other.mqtt` under an extension id of `com.nube.foo` is
  rejected with the namespace-mismatch error.
- The `GET /api/node-kinds` response against a `flow-agent` that
  loaded the fixture, showing the resolved i18n labels and the
  absolute settings-schema/description URLs.
- The "no behaviour bound" typed-error transcript from attempting
  to fire a flow that uses the fixture kind before `ProcessNodeProxy`
  lands.

Do not start slice B without explicit approval at the gate.

## Anti-patterns specific to this job

- **Do not** add a second wire method. R-flow-node-1 binds: one
  contribution kind, one adapter, one new wire method
  (`flow.node.invoke`). If something feels like it wants a
  second method, it's almost certainly a misshape of
  `flow.node.invoke`'s params/return — refactor those, do not
  add `flow.node.start_stream` or similar.
- **Do not** invent a new streaming shape. The four `stream.*`
  notifications are sufficient; the proxy uses the same
  `stream_id` (= `invocation_id`) it already manages.
- **Do not** correlate cancellation by JSON-RPC `id`. It's
  short-lived, allocated by `SupervisorHandle::call`, and
  invisible to the proxy. Cancellation is keyed by
  `invocation_id`. Mixing them up is the most likely correctness
  bug in slice B.
- **Do not** make `deadline_ms` authoritative. It is advisory.
  Host-side `SupervisorHandle::call` timeout is the hard bound.
  A child that ignores the deadline gets a `stream.cancel` from
  the host and a `NodeError::Backend("timeout")` returned to the
  engine.
- **Do not** widen `NodeDescriptor` to `String`. R-flow-node-2
  binds: `Cow<'static, str>` is what keeps built-in descriptors
  zero-allocation while letting extension descriptors own their
  strings. `String` everywhere would force `Box::leak` or worse
  on built-ins.
- **Do not** `Box::leak` in the dynamic registry. Owned
  descriptors live as long as the registry does; dropping the
  registry drops the strings. If you find yourself reaching for
  `Box::leak`, the architecture is wrong — surface.
- **Do not** put `schemars`/`jsonschema` into `starter-ext-host`.
  R-flow-node-7 binds: validation belongs to the engine
  (`DefinitionManager::publish`), not the kernel. The kernel
  serves the schema file; the engine gates publication on it.
- **Do not** add `validate_settings` to the trait in either
  slice. The seam is documented in R-flow-node-7 but explicitly
  out of scope. Future work writes it as `flow.node.validate_settings`
  over the wire, gated by a manifest opt-in flag.
- **Do not** spawn a second supervisor per node kind. Nodes are
  behaviours, not processes. One supervisor per extension; many
  node kinds inside.
- **Do not** ship MQTT credentials in `block.yaml`. The bundle
  declares the broker URL; credentials route through
  `secrets.get` per the source SCOPE.
- **Do not** weaken the reload guarantee table. R-flow-node-6's
  bucket semantics — unchanged keeps the handle, replaced/removed
  gracefully wind down to a configurable cap, added gets a fresh
  supervisor — are the contract. Conflating buckets ("just kill
  the old handle") is a regression even if tests pass.
- **Do not** start slice B without a green REVIEW gate. The MQTT
  acceptance test in slice B exercises every property slice A
  set up; if slice A's descriptor shape is wrong, slice B's tests
  produce false greens against a buggy contract.

## When to halt

- The `NodeDescriptor` `Cow` widening fails to compile at a
  built-in call site that the const-fn shim should have covered.
  Halt; the shim signature is wrong, the resolution is to widen
  the shim — but the shape of the widening needs design review
  before code lands.
- The reverse-DNS validator turns out to reject a legitimate
  fixture (e.g. an existing extension already in the tree
  contributes a non-descendant kind). Surface; either the
  existing extension is buggy and needs renaming, or the validator
  is too strict.
- The MQTT acceptance test needs the `mosquitto` container to be
  in the CI image and it isn't. Surface in chat; the resolution
  is to add the container to CI infra (separate concern, may need
  approval), not to weaken the test by mocking the broker.
- `Arc::strong_count == 1` polling for deferred shutdown turns
  out to busy-loop or race against late `Arc::clone`s. Surface;
  the resolution is to add an explicit shutdown channel into
  `SupervisorHandle` rather than rely on refcount alone, which
  is a design change above this job's authority.
- A child that ignores `stream.cancel` cannot be killed by the
  supervisor's restart policy on the next health ping miss (the
  ping itself blocks because the dispatcher is busy). Surface;
  the resolution is in the supervisor's ping loop, not in this
  job.
- The slice B acceptance criteria cannot all pass under the
  remaining budget after slice A. Halt at the REVIEW gate, split
  slice B into `flow-nodes-slice-b`, do not silently land a
  partial reload algorithm.
