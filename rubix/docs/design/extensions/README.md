# EXTENSIONS — block-author guide + 10-minute scaffold

> **Authoritative extension framework:** the `starter-extensions/`
> sibling workspace (`starter-ext-spi`, `starter-ext-sdk`,
> `starter-ext-supervisor`, etc.).
>
> Cites: SCOPE [R8](../../SCOPE.md#r8), [Phase 5](../../SCOPE.md).

## What rubix extensions contribute

Via `block.yaml` `contributes`:

- **tools** — Rust `impl Tool` packaged into the extension binary.
- **skills** — `SKILL.md` files (quarantined by default).
- **flows** — flow YAML rooted at `ai-agent` (or any registered
  node kind).
- **nodes** — new `NodeBehavior` impls (rare; usually upstream to
  `starter-flow-nodes`).

All four flow into the **same** host registries as rubix-bundled
contributions. The `ai-agent` node doesn't know or care which
bucket a thing came from (SCOPE R7).

## The 10-minute scaffold (Phase 5 exit goal)

A fresh extension author starting from zero should reach a working
extension in under 10 minutes:

1. **Copy** [extensions/com.rubix.example/](../../extensions/com.rubix.example/)
   to `extensions/com.<org>.<name>/`.
2. **Edit** `block.yaml` — set `id`, `version`, and which of
   `tools` / `skills` / `flows` you contribute.
3. **Write** the contributed pieces:
   - Rust tools → implement `Tool` against `rubix-extensions-sdk`
     (planned upstream — see [STARTER-CHANGES.md](./STARTER-CHANGES.md)).
   - Skills → `SKILL.md` files under `skills/<id>/`.
   - Flows → YAML under `flows/`.
4. **Build** the extension binary: `cargo build -p <crate>`.
5. **Boot** rubix-agent; the host loader picks the extension up
   from the configured extensions dir.

If any of those steps takes more than two minutes, the gap is an
upstream starter ergonomic issue — file it per SCOPE R2 *before*
Phase 5 exit.

## What an extension never depends on

Per SCOPE R8:

- **Never** `rubix-domain` / `rubix-tools` / `rubix-flows` /
  `rubix-skills` / `rubix-agent` — none of those.
- **Never** any `starter/crates/*` directly (only via the SDK).
- **Never** `rubix-client` (the agent's HTTP client is for callers,
  not for in-host extensions).

Extensions depend on:
- `rubix-extensions-sdk` (planned upstream).
- `rubix-spi` for shared DTOs (only if the extension talks to
  rubix-specific endpoints; usually not needed).

## Trust + content-hash quarantine

Per starter agent SCOPE R4: extension-shipped skills default to
`trust: quarantined`. An operator approves the bundle by its
content hash; one byte changes → re-quarantined. This is the
single most load-bearing safety property.
