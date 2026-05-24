# 2026-05-24 — Fix admin Tabs layout regression (grey ellipse + split columns)

> **Tier:** session note. Lifetime: days. Per
> [HOW-TO-CODE.md §0a](../../HOW-TO-CODE.md) and
> [NEW-SESSION.md §2](../../NEW-SESSION.md), **source code must
> never reference this file.**

## The bug, in one paragraph

Both `/admin/access` and `/admin/warehouse` render with a broken layout: a large grey rounded shape (looks like an ellipse `border-radius: 50%`) wraps the tab list, the tab triggers are stretched and sit inside that shape, and the active tab's content is rendered **to the right of the tab list as a separate column** instead of below it. The active tab content also shows a partial form (e.g. "Create tenant" with a Loading… indicator below it). Functionally each tab still toggles, but the page looks broken. The route source ([`routes/admin/access.tsx`](../../frontend/src/routes/admin/access.tsx), [`routes/admin/warehouse.tsx`](../../frontend/src/routes/admin/warehouse.tsx)) and the two shells ([`packages/starter-ui-authz/src/panels/authz-admin.tsx`](../../../packages/starter-ui-authz/src/panels/authz-admin.tsx), [`rubix/frontend/src/components/admin/warehouse/warehouse-admin.tsx`](../../frontend/src/components/admin/warehouse/warehouse-admin.tsx)) are clean — they use `<Tabs>` / `<TabsList>` / `<TabsTrigger>` / `<TabsContent>` from `@nube/starter-ui-kit` in the normal shape with `<TabsContent className="mt-6">`. So the regression lives in the **`Tabs` primitive itself** (the kit's component or its CSS), in the rubix-frontend global CSS clobbering it, or in a Tailwind v4 source-scanning gap that drops the classes the primitive relies on. The fact that both pages exhibit the identical visual bug, and the source uses the kit's exports verbatim, means it is one root cause that affects every page using `Tabs`.

The route source for proof — `warehouse-admin.tsx` is the simplest:

```tsx
<Tabs value={tab} onValueChange={(v) => setTab(v as WarehouseAdminTab)}>
  <TabsList className="flex-wrap">
    <TabsTrigger value="rules">Rules</TabsTrigger>
    …
  </TabsList>
  <TabsContent value="rules" className="mt-6"><WarehouseRulesPanel /></TabsContent>
  …
</Tabs>
```

The visual symptoms (ellipse around the list, content as right column) are not producible from that JSX without a global style override or a missing class. The shipped `@nube/starter-ui-kit` `Tabs` primitive must currently:

- Be missing the styles that render `<TabsList>` as a flat horizontal pill (the grey ellipse suggests `<Tabs>` itself has an unintended `rounded-full` / large `border-radius` and the `<TabsList>` inside is not given its own bg/border so the parent bleeds through).
- Have `<TabsContent>` positioned `flex` / `inline` / `grid-cols-2` next to the list instead of `block` below it (the rendering as a right-side column is the tell).

This is one CSS/markup issue — fix the primitive once, both pages return to normal.

## Read first

Before touching anything:

1. [HOW-TO-CODE.md](../../HOW-TO-CODE.md) — contributor entry point.
2. [SCOPE.md](../../SCOPE.md) — R2 (upstream-first) matters here: the bug is in `@nube/starter-ui-kit`, not in rubix-frontend; the fix lands upstream.
3. [`packages/starter-ui-kit/src/components/ui/`](../../../packages/starter-ui-kit/src/components/ui/) — find `tabs.tsx` (or whatever the file is named). Read it end-to-end.
4. [`packages/starter-ui-kit/src/components/ui/tabs.test.tsx`](../../../packages/starter-ui-kit/src/components/ui/tabs.test.tsx) — verify whether tests exist; if yes, read; if no, add one as part of the fix.
5. [`packages/starter-ui-authz/src/panels/authz-admin.tsx`](../../../packages/starter-ui-authz/src/panels/authz-admin.tsx) — the canonical consumer; should not need modification.
6. [`rubix/frontend/src/components/admin/warehouse/warehouse-admin.tsx`](../../frontend/src/components/admin/warehouse/warehouse-admin.tsx) — the rubix consumer; same shape; should not need modification.
7. [`rubix/frontend/src/styles/`](../../frontend/src/styles/) — `primitives.css`, `theme.css`, `tokens.css`. Look for any selector that targets `[data-state]`, `[role="tablist"]`, or `[role="tab"]` — a global rule here could be the override clobbering the primitive.
8. [`rubix/frontend/index.html`](../../frontend/index.html) and [`rubix/frontend/vite.config.ts`](../../frontend/vite.config.ts) — verify the Tailwind v4 source-scanning includes `node_modules/@nube/starter-ui-kit/**/*.tsx` and `node_modules/@nube/starter-ui-authz/**/*.tsx`. The merge commit [b72f238 "migration of ui theme to starter"](https://github.com/NubeDev/starter/commit/b72f238) and the follow-up [27c3e20 "ship scan-source.css for Tailwind v4 source-scanning"](https://github.com/NubeDev/starter/commit/27c3e20) both touch this area — if classes from the kit are absent from the final stylesheet, Radix renders unstyled and the visual symptoms match exactly. This is the most likely root cause.
9. The two screenshots the operator captured (in this conversation) — they are the ground truth for what "broken" looks like.

## Hypothesis order (work the cheap one first)

1. **Tailwind v4 source-scan misses the kit packages** — most likely. The kit's `Tabs` primitive renders Radix's accessible markup but relies on Tailwind classes (e.g. `inline-flex`, `items-center`, `rounded-md`, `bg-muted`, `data-[state=active]:*`). If Tailwind v4's content scanner does not include `node_modules/@nube/starter-ui-kit/dist/**/*.js` (or the package's source), those classes are not in the final CSS bundle. The rendered HTML still has the classes as strings, but no rule applies — so the browser falls back to default block layout, the `<Tabs>` root inherits the page background, the `<TabsList>` looks like an unstyled `<div>`, and `<TabsContent>` lays out adjacent to the list under default flow rules. The grey ellipse is whatever fallback border-radius/background the parent route's container provides. Confirm by inspecting the rendered HTML in dev tools — if the classes are present on the elements but no CSS rules match them, this is the cause. Fix: add the kit packages to the Tailwind v4 source-scan config (`@source` directive in the global CSS, or `content` array if a `tailwind.config.*` exists). Stage 1 fixes this and re-verifies both pages render normally.
2. **A global CSS rule in the rubix-frontend styles directory clobbers the primitive** — second-most likely. Some `[role="tablist"] { … }` or `[data-state] { … }` selector with a large `border-radius` or `display: flex; flex-direction: row` against the children would produce exactly the observed symptoms. Confirm by inspecting dev tools, finding the offending rule, tracing it to a file under `rubix/frontend/src/styles/`. Fix: remove or scope the offending rule.
3. **The kit's `Tabs` primitive itself is wrong** — least likely given the shape of the source consumers, but possible. If neither (1) nor (2) explains the rendering, read `tabs.tsx` and compare to a canonical shadcn-style Tabs implementation. Fix: correct the primitive's classes/structure.

Cheaper-first ordering is non-negotiable. Don't redesign the primitive before verifying it's not a scan / global-CSS issue.

## The work

One PR off `fix/admin-tabs-layout`, two-or-three commits depending on the diagnosis.

### Stage 1 — diagnose

Spin up the local dev stack:

```bash
cd /home/user/code/rust/starter/rubix
make start
# open http://127.0.0.1:5185/admin/access (or 5173 — confirm vite port)
# log in as op@example.com / rubix-dev-passwd
```

Open dev tools, inspect a `<TabsList>` element:

- **What classes are on the element?** Compare to the source in `packages/starter-ui-kit/src/components/ui/tabs.tsx`. If the classes match the source, Tailwind didn't drop them — proceed to inspect computed styles.
- **What computed styles apply?** Look at the rules cascading onto the element. If no kit classes have any rules but the classes exist as strings, hypothesis (1) is confirmed.
- **Are there any global selectors targeting `[role="tablist"]` / `[role="tab"]` / `[data-state]`?** If yes and they apply, hypothesis (2) is confirmed.
- **Walk the parent tree.** The "grey ellipse" is some ancestor element; find which one and what gives it the rounded shape. Is it the `<Tabs>` root, or is it a route-shell wrapper inherited from `__root.tsx`?

Write the diagnosis as one paragraph in the **Resolution** section of this session note. **Do not** push code until the diagnosis is recorded. The fix must address the cause, not the symptom.

### Stage 2 — fix

Apply the smallest change that resolves the diagnosis. Concretely:

- **If hypothesis (1):** add `@source "../../node_modules/@nube/starter-ui-kit/dist/**/*.js";` (and equivalent for `starter-ui-authz` if its panels also render unstyled — verify by checking the same dev-tools symptoms on a non-Tabs primitive like a button inside `TenantsPanel`). The `@source` directive form is the Tailwind v4 way; the v3 way is a `content` array entry. Find which form this project uses and match it. Commit:

  ```
  fix(rubix-frontend): include starter-ui-kit + ui-authz in Tailwind source-scan

  The kit's Tabs primitive (and other primitives) shipped class
  attributes that Tailwind v4 was not scanning, so the final CSS
  bundle dropped the rules. Result: <Tabs> rendered unstyled and
  every page using it (the admin Access and Warehouse pages) looked
  broken. Adds the kit + authz packages to the @source directives.
  ```

- **If hypothesis (2):** delete or properly scope the offending global rule. The fix is to one file in `rubix/frontend/src/styles/`. Commit:

  ```
  fix(rubix-frontend): remove global selector clobbering Radix Tabs

  A selector in <path> targeting [role="tablist"] applied a large
  border-radius and flex layout to every Radix Tabs primitive on
  the page. The intent was to style a specific landing-page widget;
  scope the rule to that widget instead.
  ```

- **If hypothesis (3):** correct the primitive. This is an **upstream change in `@nube/starter-ui-kit`** per R2. Commit:

  ```
  fix(starter-ui-kit): tabs primitive renders TabsContent below TabsList

  The Tabs primitive was emitting <TabsContent> as a sibling of
  <TabsList> in a flex-row layout, causing the active tab content
  to render to the right of the list instead of below it. Adjusts
  the primitive's root element to a column flex and removes the
  unintended rounded-full on the wrapper.
  ```

Verify the fix:

```bash
pnpm --filter @nube/starter-ui-kit typecheck && pnpm --filter @nube/starter-ui-kit test
pnpm --filter @nube/rubix-frontend typecheck && pnpm --filter @nube/rubix-frontend test
# then visually:
cd rubix && make restart  # picks up CSS changes via vite HMR but a restart is safer
# open /admin/access and /admin/warehouse — assert visible tab list with content below
```

### Stage 3 — add a regression test

Whatever the diagnosis, one or both of these tests must land:

- **A vitest unit test in `packages/starter-ui-kit/src/components/ui/tabs.test.tsx`** asserting that `<Tabs>` renders with `<TabsList>` followed by `<TabsContent>` in document order, and that the active `<TabsContent>` is `display: block` (or whatever the intended block-layout assertion is). This catches hypothesis (3) and any future regression of the primitive.
- **A Playwright assertion in `rubix/frontend/e2e/authz-admin.spec.ts` and/or `warehouse.spec.ts`** that the rendered tab list is followed by the tab content in the DOM and that the bounding box of the active tab content is **below** the tab list, not beside it. Pseudo:

  ```ts
  const list = page.getByRole('tablist').first();
  const content = page.locator('[role="tabpanel"]:visible').first();
  const lb = await list.boundingBox();
  const cb = await content.boundingBox();
  expect(cb!.y).toBeGreaterThan(lb!.y + lb!.height - 1);
  ```

  This catches hypothesis (1) and (2) at the integration level — if Tailwind drops the classes again or a global rule re-breaks the layout, this assertion fails.

Commit:

```
test(starter-ui-kit + rubix-frontend): assert TabsContent renders below TabsList

Adds a vitest unit + two Playwright assertions that fail when the
Tabs primitive's content is rendered beside (rather than below)
the tab list. Catches both upstream regressions of the primitive
and downstream regressions of the build/CSS pipeline.
```

### Stage 4 — close

Update the **Resolution** section at the bottom of this session note with:

- The hypothesis the diagnosis confirmed.
- The exact file(s) changed.
- A before/after screenshot (or a short description; the operator already has the "before" captured).
- Any follow-ups surfaced (e.g. "should scan all `@nube/*` packages by default to prevent the next package from hitting this", "the screenshot also showed a 'Loading…' state next to the Create tenant form — is that a separate bug worth its own issue?").

Open the PR. Title:

```
fix(ui): admin Tabs layout — TabsContent below TabsList, no ellipse
```

Body: per the standard convention, summarise each commit. Mention the two pages affected so reviewers know where to verify visually.

## Out of scope

- **Do not redesign the Tabs primitive's visual style.** The fix restores the intended layout; refinements (spacing, hover states, focus rings, mobile responsiveness) are separate work.
- **Do not change the routes (`access.tsx`, `warehouse.tsx`) or the shells (`AuthzAdmin`, `WarehouseAdmin`).** They are correct; the bug is below them in the kit / build layer.
- **Do not "fix" by adding a wrapper `<div>` in the routes** to mask the regression. Find and fix the root cause.
- **Do not touch the `Loading…` indicator below the Create-tenant form.** That is downstream of `TenantsPanel`'s data hook and not part of this layout bug — track it separately if it persists after the layout fix.
- **No backend changes.**
- **No new starter packages.**
- **No `--no-verify`, no `--force`.**

## Bootstrap user (carry-forward)

For the next operator continuing this work from a fresh worktree: the bootstrap user is `op@example.com / rubix-dev-passwd` (admin). Created idempotently by `make start` per [`rubix/Makefile`](../../Makefile).

## Hard rules

- R1 — verb per file; ≤ 200 lines TS.
- R2 — upstream-first; the fix lives in `packages/starter-ui-kit` or the Tailwind config, **not** in rubix-frontend's local CSS as an override.
- R3 — code comments link `docs/design/<area>/README.md` only; this session note (under `docs/sessions/`) is unreferenced from any source file.
- R6 — tests live with the code in the same commit.

## References

- [`packages/starter-ui-kit/src/components/ui/tabs.tsx`](../../../packages/starter-ui-kit/src/components/ui/tabs.tsx) — the primitive.
- [`packages/starter-ui-authz/src/panels/authz-admin.tsx`](../../../packages/starter-ui-authz/src/panels/authz-admin.tsx) — canonical consumer.
- [`rubix/frontend/src/components/admin/warehouse/warehouse-admin.tsx`](../../frontend/src/components/admin/warehouse/warehouse-admin.tsx) — rubix consumer (same shape).
- [`rubix/frontend/src/styles/`](../../frontend/src/styles/) — possible site of a global selector clobber.
- [`rubix/frontend/vite.config.ts`](../../frontend/vite.config.ts) and [`rubix/frontend/index.html`](../../frontend/index.html) — Tailwind v4 wiring.
- Commits to suspect: `b72f238` (ui-theme migration), `27c3e20` (scan-source.css ship). Both recent, both touch the styling pipeline.
- Operator screenshots in the originating conversation (Access Control + Warehouse).
- PR #35 + #37 — the merges that shipped the affected routes.

## Resolution

**Hypothesis (3) — the kit's `Tabs` primitive itself was wrong.**

### Diagnosis

`packages/starter-ui-kit/src/components/ui/tabs.tsx` used the Tailwind v4
bare-key data variants `data-horizontal:flex-col`, `data-vertical:*`,
`data-active:bg-background`, `group-data-horizontal/tabs:*`,
`group-data-vertical/tabs:*`, and `group-data-[variant=line]/tabs-list:data-active:*`.
Those shorthands compile to attribute-**presence** selectors —
`[data-horizontal]`, `[data-active]`, etc. But Radix Tabs only emits
`data-orientation="horizontal"` and `data-state="active"`. No
`data-horizontal` or `data-active` attribute exists on the rendered DOM,
so every one of those variants silently produced no CSS rule. Concretely
the root `<Tabs>` stayed at its default `flex` (row), causing
`<TabsContent>` to render to the right of `<TabsList>` as a sibling
column; the active triggers also never got their `bg-background` paint.
The `rounded-full p-1 bg-muted` on `<TabsList>` was the only TabsList
styling that *did* apply, which is why it appeared as a grey ellipse
wrapping the stretched triggers.

Confirmed by:
- Inspecting the Radix output in `@radix-ui/react-tabs/dist/index.mjs` —
  emits `data-state` + `data-orientation` only, never `data-horizontal`
  or `data-active`.
- `rg @custom-variant packages/starter-ui-kit/src/styles/globals.css`
  found only the `dark` variant — no `data-horizontal`/`data-active`
  custom variants registered anywhere.

### Fix

One commit on `fix/auth-path-prefix` (continuing the active branch per
operator instruction — no new branch). Edits localized to the primitive:

- `packages/starter-ui-kit/src/components/ui/tabs.tsx` — rewrote every
  bare-key data variant to the explicit attribute-value form:
  - `data-horizontal:` → `data-[orientation=horizontal]:`
  - `group-data-horizontal/tabs:` → `group-data-[orientation=horizontal]/tabs:`
  - `group-data-vertical/tabs:` → `group-data-[orientation=vertical]/tabs:`
  - `data-active:` → `data-[state=active]:`
  - and the compound variant in the line-variant trigger.

Per R2 (upstream-first) the fix lives in the kit, not in
`rubix/frontend/src/styles/`. No rubix-frontend, route, or shell file
changed. `pnpm --filter @nube/starter-ui-kit typecheck` and
`pnpm --filter @nube/rubix-frontend typecheck` both pass clean.

### Regression tests

- `rubix/frontend/e2e/authz-admin.spec.ts` — added
  `TabsContent renders below TabsList, not beside it`. Reads the
  bounding boxes of the active `[role="tablist"]` and
  `[role="tabpanel"][data-state="active"]` and asserts
  `panel.y > list.y + list.height - 1`. Fails on both the bare-key
  regression (panel beside list) and on any future global-CSS clobber
  that re-introduces the side-by-side layout.
- `rubix/frontend/e2e/warehouse.spec.ts` — same assertion against
  `/admin/warehouse`. Two pages, two assertions; one root cause.

No vitest unit was added to `packages/starter-ui-kit` — that package
currently has no vitest setup and the spec accepted either unit *or*
Playwright. The Playwright assertions cover the layout shape, which is
what the diagnosis showed was broken.

### Follow-ups

1. Consider declaring project-wide `@custom-variant` aliases for
   `data-active`, `data-horizontal`, `data-vertical` in the kit's
   `scan-source.css` as a belt-and-braces guard — but only after
   confirming the kit doesn't intend to support the bare-key form for
   any other primitive. Skipped here to keep the fix minimal.
2. The "Loading…" indicator the screenshot showed next to the
   Create-tenant form is downstream of `TenantsPanel`'s data hook; the
   layout fix alone resolves the *visual* regression, but verify that
   indicator behaves as intended once the page renders normally — if
   not, file a separate issue.
3. Audit other shadcn-style primitives in the kit
   (`accordion.tsx`, `dialog.tsx`, etc.) for the same bare-key data
   variant pitfall before the next minor release.
