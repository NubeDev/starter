# `packages/` — Starter TypeScript/React workspace

This directory is the **frontend half** of the starter. Every package is
published as `@nube/starter-*` and is consumed via pnpm workspace links
(`workspace:*`). Each package is intentionally small and single-purpose —
the layering below describes how they compose.

> Looking for the Rust side? See [`crates/`](../crates).
> Looking for runnable demos? See [`examples/`](../examples).

---

## Package map

| Package | Purpose | Layer |
| :--- | :--- | :--- |
| [`@nube/starter-client-ts`](./starter-client-ts) | OpenAPI-generated TypeScript HTTP client for `starter-server`. Zero React, no UI deps. | Wire |
| [`@nube/starter-ui-core`](./starter-ui-core) | React glue: `AuthProvider`/`useAuth`, namespaced TanStack Query keys, `i18n` helpers, `preferences` store, testing utilities, theme-editor primitives. Wraps `client-ts`. | Glue |
| [`@nube/starter-ui-kit`](./starter-ui-kit) | Design system: shadcn/ui primitives + Tailwind v4 tokens + a `theme-editor/config-drawer`. **Zero I/O.** | Design system |
| [`@nube/starter-sdui-react`](./starter-sdui-react) | Renderer for server-driven-UI trees produced by `starter-sdui-routes`. Projects against `ui-kit` primitives. | Renderer |
| [`@nube/starter-ui-chat`](./starter-ui-chat) | Reusable AI chat surface (composer, message list, tool calls) with a headless `ChatAdapter` transport. | Feature kit |
| [`@nube/starter-ui-skills`](./starter-ui-skills) | UI for browsing and inspecting a `starter-skills` registry (SKILL.md bundles). | Feature kit |
| [`@nube/starter-ui-flow`](./starter-ui-flow) | React components for the `starter-flow` node graph. Wraps `@xyflow/react` with typed slots, nodes, and edges. | Feature kit |
| [`@nube/starter-ui-blobs`](./starter-ui-blobs) | Hooks for direct-to-storage blob uploads + a markdown-editor integration. | Feature kit |
| [`@nube/starter-ui-export`](./starter-ui-export) | Browser-side PDF export: `<PrintableContent>`, `<ExportButton>`, `usePrint`, `exportNodeToPdf`. | Feature kit |
| [`@nube/starter-ui-ai-builder`](./starter-ui-ai-builder) | Split-pane composition: chat composer (`ui-chat`) driving a live SDUI canvas (`sdui-react`). | Composition |

---

## Dependency diagram

```
                    ┌──────────────────────────┐
                    │  starter-client-ts       │  wire (OpenAPI codegen)
                    └────────────┬─────────────┘
                                 │
                                 ▼
                    ┌──────────────────────────┐
                    │  starter-ui-core         │  glue (auth, query, i18n)
                    └────────────┬─────────────┘
                                 │ (peer)
                                 ▼
                    ┌──────────────────────────┐
                    │  starter-ui-kit          │  design system (shadcn + tokens)
                    └────┬─────────────┬───────┘
                         │             │
        ┌────────────────┤             ├────────────────┐
        ▼                ▼             ▼                ▼
  starter-ui-chat   starter-ui-skills  starter-sdui-react   …
        │                              │
        └──────────────┬───────────────┘
                       ▼
            starter-ui-ai-builder        composition

  starter-ui-flow   starter-ui-blobs   starter-ui-export
  (standalone — no @nube/* workspace deps)
```

Read this top-to-bottom: a feature kit that imports `ui-kit` also
transitively inherits `ui-core`'s peer contract (React, the auth/query
providers). Standalone packages on the bottom row don't depend on
`ui-kit`/`ui-core` and can be adopted independently.

---

## Conventions

- **Source-only publishing.** Every package's `main`, `types`, and
  `exports` point at `src/*.ts(x)`. Consumers (apps in `examples/` or
  `test-ui-*/`) bundle our source directly via Vite. No `dist/` is shipped.
- **`exports` field is the contract.** Adding a subpath export is a
  feature; removing one is a breaking change. The `scripts/check-public-api.mjs`
  snapshot guards `src/index.ts` so unintended additions/removals
  show up in PR diffs (see [public API surface](#public-api-surface) below).
- **Icons: lucide-react.** The kit ships exactly one icon family — the
  same one shadcn/ui uses by default. Feature kits and consumers should
  prefer `lucide-react` over alternatives (Hugeicons, Tabler, Heroicons)
  to keep bundle size and visual language consistent.
- **CSS imports.** Packages that ship styles expose them as
  `./styles.css` and document required `@source` directives in their
  README so Tailwind v4 can scan their JSX.
- **No deep imports across packages.** Always import from the package
  name or an explicit subpath export — never from `@nube/*/src/...`.

---

## Public API surface

The exported symbol list from each package's `src/index.ts` is
checked into `packages/<pkg>/api.snapshot.txt` and verified in CI by
`scripts/check-public-api.mjs`.

```bash
# Verify all snapshots match current sources (CI)
pnpm api:check

# Update snapshots after an intentional API change
pnpm api:update
```

A failing `api:check` means the public surface of a package changed.
Either: (a) it was intentional — run `pnpm api:update`, review the
diff, and include it in your PR; or (b) it was accidental — fix the
`src/index.ts` re-exports.

---

## Workspace scripts

From the repo root:

```bash
pnpm install                    # link the workspace
pnpm -r typecheck               # typecheck every package
pnpm -r --filter '@nube/*' build
pnpm api:check                  # verify public API snapshots
```

---

## Adding a new package

1. Pick a name: `@nube/starter-<thing>`. Keep it small and single-purpose.
2. Copy `starter-ui-blobs/` as a minimal template (no workspace deps) or
   `starter-ui-chat/` if you need `ui-kit`.
3. Add a one-line entry in the [Package map](#package-map) above and a
   node to the [dependency diagram](#dependency-diagram).
4. Run `pnpm api:update` to seed the API snapshot.
5. If you ship CSS, document the required `@source` directives in the
   package README.
