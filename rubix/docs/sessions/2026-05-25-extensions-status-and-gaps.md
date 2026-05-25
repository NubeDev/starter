# 2026-05-25 — Extensions: status check + remaining gaps (upstream-first)

Walking session note. Reviewed the state of the
`codeless/rubix-extensions-wire` job
([`.codeless/jobs/rubix-extensions-wire/SCOPE.md`](../../../.codeless/jobs/rubix-extensions-wire/SCOPE.md))
against the four asks: (1) is the extension server done; (2) can an
extension extend the flow / nodes pallet; (3) can an extension load a
frontend via Module Federation; (4) does the host start / stop /
monitor / restart-on-fail.

**Bottom line:** the backend is done; two consumer-side wires are
missing. **Both remaining wires belong upstream in
`starter-extensions/` (Rust adapter + `@nube/starter-ext-ui` host),
not in bespoke rubix code.** Rubix's role stays "compose upstream
primitives" per SCOPE R2 + R8.

## What is done

### Server lifecycle — done

- `starter-ext-supervisor` provides restart-with-backoff for
  process-flavour extensions.
- `starter-ext-server::router_with_auth` is merged under
  `/api/v1/extensions/*` in
  [`rubix/crates/rubix-agent/src/main.rs`](../../crates/rubix-agent/src/main.rs)
  (`L260–L275`), behind the upstream `with_principal` +
  `with_role(Role::Admin)` gate.
- Lifecycle verbs: `GET /extensions`, `GET /:id`, `POST
  /:id/{start,stop,restart,enable,disable}`, `GET /:id/events` (SSE),
  `POST /extensions/install` (multipart tarball — registry-URL stub
  returns `not_implemented`), `DELETE /:id`, `GET /:id/ui/*path`, `GET
  /:id/i18n/:lang`.
- PG persistence via the upstream
  [`starter-ext-store-pg`](../../../starter-extensions/crates/starter-ext-store-pg/)
  crate; migration applied at boot from
  [`rubix/crates/rubix-agent/src/boot/extensions.rs`](../../crates/rubix-agent/src/boot/extensions.rs).
- Autostart on boot under the synthetic principal
  `system:extensions-autostart`.
- Integration test:
  [`tests/extensions_lifecycle_test.rs`](../../crates/rubix-agent/tests/extensions_lifecycle_test.rs)
  (testcontainers, `#[ignore]`-gated like the other PG tests).

### MCP tool surface — done

- `starter-ext-mcp::register_process_tools` is called from
  [`boot/mcp/mod.rs`](../../crates/rubix-agent/src/boot/mcp/mod.rs)
  (`L152`) so each extension's `contributes.tools[]` lands in the
  same `ToolRegistry` as the bundled `FlowAsTool` entries.

### Example bundle builds — done

- [`rubix/extensions/Cargo.toml`](../../../extensions/Cargo.toml)
  sibling workspace produces `rubix-example-extension`. SCOPE R8
  boundary holds — no `rubix-*` path-deps.

## What is **not** done — and where the fix belongs

### Gap 1 — flow / skills / nodes contributions are not surfaced

The upstream adapter
[`starter-extensions/crates/starter-ext-flow/`](../../../starter-extensions/crates/starter-ext-flow/)
already implements:

- `contributed_skills()`   → `Vec<ContributedSkill>` for
  `starter_skills::SkillRegistry::extend`,
- `contributed_node_kinds()` → `Vec<DynamicNodeKindEntry>` for the
  flow `NodeRegistry` (slice A with `UnboundNodeBehavior`; slice B's
  `ProcessNodeProxy` already exists in the same crate),
- the trust-matrix contract (extension skills are always
  `Trust::Quarantined`).

`grep -r starter-ext-flow rubix/crates/` returns **no matches**.
Rubix-agent loads bundles, gets a sealed `ExtensionRegistry`, but
never walks it through the adapter. Net effect: the example's
`flows/example-assistant.yaml` and `kinds/` are on disk but the
running agent doesn't know about them.

**Where the fix lives:** rubix gets *one* tiny composer (a verb file
`boot::extensions_flow::wire` ~80 lines) that takes the
`ExtensionAdminBundle` plus the existing `FlowRegistry` / `NodeRegistry`
/ `SkillRegistry` builders and calls the three upstream functions. **No
new logic. No bespoke walker.** If anything is awkward to call from a
host (e.g. the adapter wants a builder shape rubix doesn't have),
that's an upstream issue — fix it in `starter-ext-flow`, not in
rubix.

The `contributes.flows` branch is still a *later phase of the flow
track* per [`starter-ext-flow/src/lib.rs`](../../../starter-extensions/crates/starter-ext-flow/src/lib.rs)
module docs — that piece is **owned upstream** and gated there; rubix
just consumes the function when it's available.

### Gap 2 — no frontend consumer for the MF host

- Server route exists:
  [`starter-extensions/crates/starter-ext-server/src/ui.rs`](../../../starter-extensions/crates/starter-ext-server/src/ui.rs)
  serves `/api/v1/extensions/:id/ui/*path`.
- Host runtime is published as `@nube/starter-ext-ui` at
  [`starter-extensions/packages/starter-ext-ui/`](../../../starter-extensions/packages/starter-ext-ui/)
  with `ExtensionHostProvider` + `ExtensionSlot`.
- The current
  [`packages/test-ui-5/`](../../../packages/test-ui-5/) has **no**
  `app/extensions/` route and **no** `@nube/starter-ext-ui`
  dependency. The design doc
  ([`rubix/docs/design/extensions/README.md`](../design/extensions/README.md))
  claims `packages/test-ui-5/src/app/extensions/page.tsx` exists — it
  does not. Phase D.2 was missed.

**Where the fix lives:**

1. The page in `packages/test-ui-5/` is a thin consumer (~30 lines):
   `<ExtensionHostProvider baseUrl={...}><ExtensionSlot id="main"
   /></ExtensionHostProvider>` plus an `@nube/starter-ext-ui`
   dependency in `package.json`. **No bespoke MF runtime in
   test-ui-5.**
2. The example's `ui/remoteEntry.js` (or vite-mf build script) so
   the FE round-trip is observable. The example bundle's `ui/` dir
   sits under [`rubix/extensions/com.rubix.example/ui/`](../../../extensions/com.rubix.example/ui/);
   if shaping the build needs a shared helper (it likely does — every
   extension's UI build looks identical), the helper lives in
   `@nube/starter-ext-sdk-ts` upstream, not in rubix.

If `@nube/starter-ext-ui`'s API is awkward for the consumer page, the
fix is *to that package*, not to a parallel rubix host.

### Cosmetic — stale docs

[`rubix/extensions/com.rubix.example/README.md`](../../../extensions/com.rubix.example/README.md),
the example's [`block.yaml`](../../../extensions/com.rubix.example/block.yaml)
header comment, and a handful of [`THIN-SLICE.md`](../scope/THIN-SLICE.md)
rows still say *"planned starter-ext-flow"* — that crate exists. Drop
the "planned" language once Gap 1 lands.

## Constraints reminder — upstream-first

SCOPE R2: extensions framework changes land in
`starter-extensions/` first. Rubix is the **first real consumer**, not
the owner. Concretely for the two gaps above:

| Need | Owner | Rubix's part |
|---|---|---|
| Walk a sealed `ExtensionRegistry` and contribute flows / nodes / skills | `starter-ext-flow` (already there) | One ~80-line composer verb that calls the three functions |
| MF host runtime + slot API | `@nube/starter-ext-ui` (already there) | One ~30-line `app/extensions/page.tsx` consuming it |
| MF build helper / scaffold for an extension's `ui/` | `@nube/starter-ext-sdk-ts` (if missing, add upstream) | Use the helper from the example |
| Registry-URL install | `starter-ext-server::lifecycle` (currently stubs `not_implemented`) | Nothing until upstream lights it up |
| Per-tenant scoping, fine-grained AuthZ, WASM flavour | Future upstream work | Nothing in this job |

Anti-patterns to avoid when finishing Gaps 1 + 2:

- ❌ A rubix-side `walk_extension_contributions()` that re-implements
  what `starter-ext-flow` already does.
- ❌ A rubix-side React host that re-implements
  `ExtensionHostProvider` / `ExtensionSlot`.
- ❌ A rubix-side MF build script in `rubix/extensions/`. Build tooling
  for `ui/` belongs in the SDK package.

## Suggested next-job shape

`codeless/jobs/rubix-extensions-flow-and-fe-consumer/` — one branch,
three commits:

1. **`feat(rubix-agent): wire starter-ext-flow contributions into boot`**
   — verb file + integration assertion that the example's flow and
   skill appear in `FlowRegistry` / `SkillRegistry` after boot.
   Upstream changes only if `starter-ext-flow`'s public API needs
   adjustment.
2. **`feat(test-ui-5+example): MF round-trip via @nube/starter-ext-ui`**
   — adds the consumer page, the package dep, and the example's
   `ui/remoteEntry.js` build. Upstream the build helper if there
   isn't one in `@nube/starter-ext-sdk-ts`.
3. **`docs: drop "planned starter-ext-flow" language`**
   — README / block.yaml header / THIN-SLICE rows.

Test counts at end:
`cargo test -p rubix-agent --lib` + the existing
`#[ignore] extensions_lifecycle_test` still green, plus one new
ignored test asserting flow/skill contribution, plus a vitest in
`packages/test-ui-5` that loads the example panel against a recorded
fixture.

## References

- [SCOPE](../../../.codeless/jobs/rubix-extensions-wire/SCOPE.md) —
  the wire job's authoritative scope.
- [Design doc](../design/extensions/README.md) — current (rewritten)
  rubix-side overview.
- [Closing note for the wire job](./2026-05-24-extensions-wired.md) —
  what landed last pass.
- [`starter-ext-flow`](../../../starter-extensions/crates/starter-ext-flow/)
  — upstream adapter for `contributes.{flows,skills,nodes}`.
- [`@nube/starter-ext-ui`](../../../starter-extensions/packages/starter-ext-ui/)
  — upstream MF host runtime.
- [`@nube/starter-ext-sdk-ts`](../../../starter-extensions/packages/starter-ext-sdk-ts/)
  — author-side TS SDK (home for any MF build helper).
- SCOPE.md R2 (upstream-first), R8 (extensions never depend on
  rubix-*) — the rules this work lives under.
