## Done

- Created the sibling `starter-extensions/` cargo workspace (its own `[workspace]` root inside this repo so it does not pollute the parent `starter` workspace members list).
- Built `starter-ext-spi` (commit `dd9190c`) per SCOPE.md R2 — depends only on `starter-spi`, zero runtime logic, zero I/O. Modules: `behavior`, `manifest`, `id`, `capability`, `lifecycle`, `jsonrpc`, `error`.
- `Manifest` parses with `deny_unknown_fields` at every level, carries `v: 1` (`MANIFEST_VERSION`), and the `contributes.{tools,cli,rest,grpc,workers,ui}` block per R13. Per-entry `AuthGate { require_role, require_scope }` is on every contribute entry (post-R13 follow-up).
- `JsonRpcEnvelope` (untagged Request/Response/Notification) plus `stream_methods::{EVENT,END,ERROR,CANCEL}` constants and a typed `StreamNotification` view.
- `ExtensionId` newtype validates reverse-DNS form, rejects `sys.*` / `starter.*`, and implements `owns()` for R4 namespace checks.
- 27 unit tests, all passing. `cargo clippy --all-targets -- -D warnings` clean.

## Next

- Stage 4: per the SCOPE phasing this is Kernel Phase 1 continued — `starter-ext-host` (manifest loader, two-phase validator, namespace + capability checks, `ExtensionRegistry`) and/or `starter-ext-sdk` (`#[derive(Extension)]` proc-macro, `requires!{}`, builtin entry-point glue) plus `examples/hello-builtin`. Confirm scope at the start of that stage.

## What you need to know

- The new workspace lives at `starter-extensions/` inside the starter repo as a self-contained cargo workspace. SCOPE.md calls for a *sibling* workspace; nesting it here is the pragmatic shape that lets a single git worktree commit carry both. The parent `Cargo.toml` does not list any of these crates and the dep arrow is `starter-spi → starter-ext-spi` only — no parent-workspace member depends on anything under `starter-extensions/`. If the project later wants a true sibling, `git mv starter-extensions ../starter-extensions` is mechanical.
- Per-entry `AuthGate` fields are typed as `Option<String>` rather than `starter_spi::auth::Role` / `Scope` — this is deliberate so consumer-defined role names flow through the manifest without a `starter-ext-spi` version bump. Adapters resolve the strings against their own role registry at load time.
- `Capability` is serialised with `#[serde(tag = "kind", rename_all = "snake_case")]`. The SCOPE.md example YAML uses `secrets: ["..."]` map syntax; the manifest test uses the `kind`-tagged shape. If the loader prefers the map shape, the adapter can add a `From<HashMap<String, Value>>` translation later — kept out of `ext-spi` to preserve the "zero runtime logic" rule.
- Dependencies pinned to workspace versions: `serde_yaml 0.9` (used only in dev-tests + manifest deserialisation by downstream crates), `semver 1` (for `version` and `requires[].version`), `thiserror 1`.

## Open questions

- (none — fresh session picks up Stage 4.)
