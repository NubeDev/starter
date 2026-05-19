## Done

- New `starter-extensions/crates/starter-ext-smoke` crate added to the workspace with 6 integration test files (13 tests, all passing locally) consolidating the cross-cutting SCOPE smoke scenarios: R7 byte-identical description, R1 single-flavour audit + trait-impl byte-identity, R8 capability violation gate+counter, R9 crash-loop intensity cap, canonical four-name streaming convention, extension-author zero-extra-deps Cargo.toml audit.
- Crate README documents which SCOPE scenarios live here vs. which adapter crate's `tests/` (`starter-ext-server`, `starter-ext-cli`, `starter-ext-mcp`, `starter-ext-supervisor`, `packages/starter-ext-ui` vitest).
- New `.github/workflows/starter-extensions.yml`: per-crate `cargo test -p ...` sweep, `cargo check -p hello-*` matrix job across `example × flavour`, and a dedicated zero-extra-deps audit job.
- Committed as `1bdb4fd` on `codeless/starter-extensions` with message starting `stage 16: smoke tests — full sweep`.

## Next

- (none) — this is the final stage of the job per the brief.

## What you need to know

- `cargo check --workspace` and `cargo test --workspace` do **not** work on `starter-extensions/` (pre-existing): the three `hello-*` examples request mutually-exclusive `starter-ext-sdk` flavour features and cargo's workspace-wide feature unification trips the SDK's duplicate-`#[no_mangle]` `__STARTER_EXT_FLAVOUR_MARKER` linker trap. The new CI workflow drives each crate with `-p` for that reason; the existing parent `.github/workflows/ci.yml` only runs the parent workspace and is unaffected.
- The "Same-source streams over four transports" gRPC leg is documented as `#[ignore]`-able pending Adapter Phase 8 — the smoke crate pins only the convention names. Adapter-level rendering tests are pointed at by README.
- `tests/one_source_three_flavours.rs::hello_trait_impl_body_is_byte_identical_across_flavours` strips comment-only lines before comparing; the per-flavour rationale comments differ intentionally between the three example sources.

## Open questions

- (none)
