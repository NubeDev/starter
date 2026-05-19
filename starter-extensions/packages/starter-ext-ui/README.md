# @nube/starter-ext-ui

Host-side Module Federation runtime.

Forked from `rubix-workspace/extension-ui-sdk`'s `./mf` entry, with
rubix-specific concepts (graph nodes, kind ids, slot paths tied to the
graph store) stripped — they belong in rubix-agent
(DOCS/extensions/scope/SCOPE.md §"UI package source").

## v0.1 surface

- `ExtensionHostManager` — host's runtime state: shared singletons
  (React, react-dom, `@tanstack/react-query`, `zustand`), registered
  remotes, contribution registry.
- `ExtensionHostProvider` — one component the host shell mounts to
  wire both the manager context and the `StarterClient` context.
- `<ExtensionSlot id="..."/>` — mounts every contribution whose
  manifest sets `slot: <id>`.
- `useExtensionHost()` — admin-style hook surfacing what the host
  knows about installed extensions.
- `registerExtensionRemote(id, ui, factory)` — singleton-major
  check then `factory.init(handle)`.

## Singleton negotiation (R11)

Every well-known shared package is provided once by the host. Each
extension declares the same packages with the version it was built
against; the runtime enforces a matching-majors check at load time
(SCOPE.md §"Decisions made" / singleton-mismatch). Mismatch is a
hard refusal — the extension's lifecycle state goes to `Failed` with
reason `singleton-mismatch: <pkg>@<expected> vs <actual>`; the rest
of the host keeps loading.

## Why this is separate from `@nube/starter-ui-kit`

A consumer that renders shadcn primitives without extensions does not
pay for the federation runtime (SCOPE R11). This package has zero
design-system deps; it only provides plumbing.
