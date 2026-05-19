## Done

- @nube/starter-ext-sdk-ts package (useHostClient, BlockShell, useSlotContext, registerExtensionContributions) — fork of rubix-workspace/extension-ui-sdk main entry with rubix graph hooks stripped
- @nube/starter-ext-ui package (ExtensionHostManager, ExtensionHostProvider, ExtensionSlot, useExtensionHost, registerExtensionRemote, singleton-major negotiation with load-time refusal) — fork of rubix-workspace/extension-ui-sdk ./mf entry, no rubix graph coupling
- starter-extensions/examples/hello-ui — React panel contributing to `sidebar` slot via `BlockShell` + `useSlotContext`
- Two-extensions-no-React-duplication smoke test passes (ext-ui internal + hello-ui external variant). Singleton-mismatch + missing-singleton refusals covered. `useSyncExternalStore` snapshot stability handled via per-slot memo invalidated on notify
- Workspace-level `pnpm -r typecheck` and `pnpm -r test` both green; 14 new tests across the three new packages
- Root `pnpm-workspace.yaml` extended with `starter-extensions/packages/*` and `starter-extensions/examples/*` globs

## Next

- Stage 11 — Kernel Phase 4: `starter-ext-wasm` (WASI-p2 component host on wasmtime, default-deny capabilities, per-call fuel + memory + deadline, one WIT package `starter:extension@0.1.0`). Plus `examples/hello-wasm` — same source as hello-builtin/hello-process with one cargo feature flipped. `hello-wasm` instantiates under `ext-wasm`; default-deny holds — an extension without `http_out` cannot reach the network even with `wasi:http` linked

## What you need to know

- TS workspace is set up — adding more TS packages just means another package in `starter-extensions/packages/<name>` and a `package.json` with `workspace:*` deps; pnpm picks them up via the root globs
- `useSyncExternalStore` requires referentially-stable snapshots. `ExtensionHostManager.resolveSlot` and `.listRemotes` both memoise their return values; the cache is invalidated inside `notify()`. If you add new read methods that React consumes, follow the same pattern or you'll get "Maximum update depth exceeded"
- Singleton negotiation is intentionally majors-only via `parseMajor` (regex `^[~^=><\s]*(\d+)`) — adequate for React semver, and a permissive policy (`matchingMajor` returns `false` for unparseable input). Consumers wanting different policy wrap the manager
- `RemoteFactory.singletons` is declared by *the extension*; the host's provided singletons are configured at `ExtensionHostManager` construction time. The host hands out *its own* instances — never the extension's — so React duplication is structurally impossible if both extensions go through the manager
- `BlockShell` writes `data-ext-slot={slot.slotId}` on its own outer div for telemetry, so DOM-counting tests that look for `[data-ext-slot='sidebar']` over-count. The slot root itself is reliably identifiable via `.starter-ext-slot`
- Rust side untouched by this stage. The pre-existing `cargo check --workspace` failure (duplicate `__STARTER_EXT_FLAVOUR_MARKER` when hello-builtin + hello-process both pull `starter-ext-sdk` with mutually-exclusive features) predates this stage; per-crate `cargo check -p <name>` still works and was used to sanity-check
- The hello-ui example carries a `block.yaml` with `runtime.kind: builtin` and `crate_name: hello-ui`, but no Rust crate is wired up — the example is TS-only for v0.1 since the smoke test runs in jsdom. A future stage that exercises the full Rust→TS load path would need to either add a no-op `hello-ui` Rust crate or teach the loader to accept manifest-only UI extensions

## Open questions

- (none)
