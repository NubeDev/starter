# 2026-05-27 — SDUI consolidation: the final plan

This supersedes
[`2026-05-27-sdui-package-consolidation.md`](2026-05-27-sdui-package-consolidation.md),
which proposed a half-port that turned out to be unnecessary once we
traced the real consumers.

## TL;DR

**Delete two packages, no porting required. One SDUI react package
remains. No "AI-dedicated" parallel stack is justified.**

Packages to delete:

1. `packages/starter-sdui-react/` — the "rich" duplicate (4,263 LOC).
2. `packages/starter-ui-ai-builder/` — the only consumer of the above
   (2,004 LOC).

Package to keep as the single source of truth:

- `packages/starter-ui-sdui-react/` — used by `rubix/frontend`,
  `starter-ui-sdui-puck`, and (currently) the deleted-in-Stream-A
  `*-native` packages.

## Why this is safe (and why the earlier plan was wrong)

The previous plan assumed we had to migrate `starter-ui-ai-builder`
from `starter-sdui-react` to `starter-ui-sdui-react`. That migration
is real work — the two `SduiProvider`s take different props, the
`Renderer` component has a different API, and `applyPatch.ts` doesn't
exist in the lean package. I started drafting the port and stopped
because it crossed the "is this still cleanup?" line.

But the migration is **not needed at all** because **nothing in
rubix uses `starter-ui-ai-builder`**.

Concrete evidence (grep run 2026-05-27 on `master`):

```
$ grep -rln "@nube/starter-ui-ai-builder" --include='*.ts' \
    --include='*.tsx' --include=package.json . | grep -v node_modules
packages/starter-ui-ai-builder/src/index.ts       # self barrel
packages/starter-ui-ai-builder/package.json       # self
```

```
$ grep -rln "@nube/starter-sdui-react" --include='*.ts' \
    --include='*.tsx' --include=package.json . | grep -v node_modules
packages/starter-ui-ai-builder/src/...             # only ai-builder
packages/starter-sdui-react/src/index.ts          # self
packages/starter-sdui-react/package.json          # self
```

Both packages form a closed subgraph that nothing else imports.
Deleting them as a pair removes the entire confusion without
porting a single line.

### Why the chat UI doesn't change

The chat surface at [`rubix/frontend/src/routes/chat.tsx`](../../../frontend/src/routes/chat.tsx)
streams from `POST /api/v1/chat/stream`
([`rubix/crates/rubix-agent/src/routes/chat_stream.rs`](../../../crates/rubix-agent/src/routes/chat_stream.rs)),
which dispatches to the MCP server inside the agent. The model calls
`rubix.dashboard.*` tools directly; the result is a row in
`dashboards_definitions`. The user navigates to
`/dashboards/<page_id>` and the **same** `starter-ui-sdui-react`
renderer paints it. **The AI never renders an IR preview live in the
chat UI** — that was `starter-ui-ai-builder`'s job, and that job no
longer exists in the product.

If we ever want a live "watch the AI build the page" preview again,
the right place to add it is **inside** `starter-ui-sdui-react` as a
new optional provider variant (an offline / in-memory tree mode that
skips `/ui/resolve`). One package, two providers, shared renderers.
That is strictly better than a parallel stack — see "Why no
dedicated AI packages" below.

## Why no dedicated AI packages

I considered keeping `starter-sdui-react` + `starter-ui-ai-builder`
around as the "AI-authoring" stack and treating
`starter-ui-sdui-react` as the "runtime" stack. Rejected, because:

1. **The IR is the same.** Both packages render the same JSON shape
   (`UiComponent` / `UiComponentTree`). There is no second IR to
   isolate.
2. **Renderer drift is the real risk.** Any divergence means a
   page the AI builds in the assistant looks different from the
   same page rendered at `/dashboards/<id>`. The whole point of
   SDUI is one tree, one renderer, anywhere.
3. **The "AI authoring mode" is one config flag, not a package.**
   What `ai-builder-canvas` actually does is mount the renderer
   over an inline tree with no transport and no action dispatch.
   That's a 30-line wrapper, not a 4k-line parallel package.
4. **No external consumer.** If there were three other apps using
   ai-builder, a dedicated package might be the right boundary.
   There are zero.
5. **Two packages = two bug surfaces.** The duplicate already
   diverged: it shipped widget types (`stack`, `card`, `badge`,
   `button`, `link`, `field`, `text`, `heading`) that the lean
   one never grew. Nothing in rubix uses any of them either, so
   the divergence is dead-on-arrival.

If a future AI surface needs widgets the lean renderer doesn't have,
**add them to the lean package**. One renderer, one registry, one
allowlist of types.

## Execution plan

Mechanical. No code changes outside of deletions and lockfile
updates.

### Step 1 — Pre-delete audit (verify nothing changed since the plan)

```bash
cd /home/user/code/rust/starter

# Confirm ai-builder still has no external consumers.
grep -rln "@nube/starter-ui-ai-builder" --include='*.ts' \
  --include='*.tsx' --include=package.json . | grep -v node_modules

# Should print only:
#   packages/starter-ui-ai-builder/src/index.ts
#   packages/starter-ui-ai-builder/package.json

# Confirm starter-sdui-react is only consumed by ai-builder + self.
grep -rln "@nube/starter-sdui-react" --include='*.ts' \
  --include='*.tsx' --include=package.json . | grep -v node_modules

# Should print only ai-builder files + the package's own files.
```

If either grep returns anything else, **stop** and re-evaluate.

### Step 2 — Delete the two packages

```bash
git rm -r packages/starter-sdui-react packages/starter-ui-ai-builder
```

### Step 3 — Drop dangling workspace entries

```bash
pnpm install   # updates pnpm-lock.yaml and removes workspace pointers
```

### Step 4 — Verify nothing else depended on them transitively

```bash
pnpm -w build
pnpm -w test
```

If a stray import surfaces (the audit grep should have caught it,
but lockfiles can lie), fix the consumer or back out.

### Step 5 — Clean the docs

The deleted packages are referenced in design/scope docs that were
written before they became dead code. Remove or redirect:

```bash
grep -rln "starter-ui-ai-builder\|starter-sdui-react" \
  rubix/docs/ DOCS/ 2>/dev/null
```

Known offenders to handle:

- `DOCS/frontend/ai-builder/SCOPE.md` — scope doc for the deleted
  package. Move to an `_archive/` subfolder or delete; note the
  deletion in [`rubix/docs/adr/`](../../adr/) if you want an ADR
  trail.
- `DOCS/frontend/sdui/DIVERGENCE.md` — discusses the divergence
  between the two packages. Becomes moot; delete.
- `DOCS/frontend/sdui/SCOPE.md` — names `starter-sdui-react` as the
  package the SDUI scope applies to. Update to name
  `starter-ui-sdui-react`.
- `rubix/docs/design/sdui/dashboard-api-usage.md` — already mentions
  the duplicate in issue #11; update that bullet to "resolved
  2026-05-27 — see this doc."
- `rubix/docs/sessions/sdui/2026-05-27-sdui-package-consolidation.md`
  — supersedes' link target. Add a "**Superseded by**" banner at
  the top pointing here, or delete.

### Step 6 — Update Tailwind sources if needed

`rubix/frontend/src/styles/theme.css` doesn't `@source` either of
the deleted packages, so no change there. Confirmed by inspection.

### Step 7 — Commit

```bash
git add -A
git commit -m "$(cat <<'EOF'
chore(sdui): delete starter-sdui-react + starter-ui-ai-builder

Both packages formed a closed subgraph with no runtime consumers in
rubix/frontend or any other shipped app. The chat-driven dashboard
builder flow runs entirely through MCP → rubix.dashboard.* tools →
the same starter-ui-sdui-react renderer that serves runtime pages,
so the parallel "AI-authoring" stack was redundant.

Leaves packages/starter-ui-sdui-react as the single SDUI react
package. If a future surface needs live in-memory IR preview, add
an offline provider variant inside that package — see
rubix/docs/sessions/sdui/2026-05-27-sdui-consolidation-final.md for
why a parallel package is the wrong shape.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

## Verification after landing

- `find packages/ -maxdepth 2 -name 'package.json' | xargs grep -l '"name"' | sort` shows neither deleted package.
- `pnpm -w build` green.
- `pnpm -w test` green.
- `/dashboards/data-flow-site-a` and `/dashboards/claude-hello` still
  render with row/col layout intact (smoke from earlier in this
  session).
- `/chat` still streams and tool calls still land.

## Open items left after this lands

These are not blockers for the cleanup but are worth tracking
separately:

1. **Tailwind `@source` fragility** — see issue #10 in
   [`dashboard-api-usage.md`](../../design/sdui/dashboard-api-usage.md).
   Long term: ship `scan-source.css` shim from
   `starter-ui-sdui-react`.
2. **Layout validation** — issue #9 there. Reject non-`page` roots
   and `row → row` nesting at create/update time.
3. **`page_set` mislabeling in the dashboard-builder skill** — issue
   #1 there. Either fix the skill doc or rename the tool.
4. **SSE delta is too thin for live re-render of the edit route** —
   issue #6. Either include `revision_id` and have the client
   refetch `/ui/resolve`, or include a body diff.

None of those require the package consolidation to land first; they
can be picked up in any order.
