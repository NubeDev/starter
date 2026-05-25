# Frontend renderer

## What this design covers

A new TypeScript package — `@nube/starter-ui-sdui-react` — that
turns a resolved `ComponentTree` into rendered React. One
`<SduiPage page_ref target_ref />` becomes the entire "dashboard"
route in the rubix frontend; the same package is reusable by any
starter consumer.

Per **R6** (HOW-TO-CODE.md), this package has **zero I/O** —
it's pure rendering + a hook-based subscription protocol. The
caller (rubix frontend) supplies the transport adapter.

## Why a starter package, not a rubix package

The dashboard renderer is product-neutral: the IR is starter's
([`crates/starter-ui-ir/`](../../../../crates/starter-ui-ir/)),
the routes are starter's
([`crates/starter-sdui-routes/`](../../../../crates/starter-sdui-routes/)),
and any other starter consumer (e.g. the notes example, future
SaaS apps) wants the same renderer. The rubix frontend just
points its `<SduiPage>` at `/api/v1/ui/*` and styles it with its
existing theme tokens.

## Package surface

```
packages/starter-ui-sdui-react/
  package.json
  src/
    index.ts                  ← re-exports
    SduiPage.tsx              ← top-level component
    use-resolve.ts            ← TanStack Query wrapper for /resolve
    use-subscriptions.ts      ← SSE/WS subscription manager
    use-action.ts             ← mutation hook for /action
    use-table.ts              ← paginated table source (/ui/table)
    renderer/                 ← one file per IR variant
      Page.tsx
      Grid.tsx
      Row.tsx
      Col.tsx
      Card.tsx
      Tabs.tsx
      Kpi.tsx
      Chart.tsx
      Table.tsx
      Form.tsx
      Select.tsx
      Toggle.tsx
      Slider.tsx
      DateRange.tsx
      Divider.tsx
      Text.tsx
      Heading.tsx
      Custom.tsx              ← delegates to the registered renderer-id
      Dangling.tsx            ← fallback for unknown / filtered variants
      registry.ts             ← variant → component dispatcher
    capability.ts             ← builds ClientCapabilities from registry
    transport.ts              ← SduiTransport interface + provider + context
    types.ts                  ← re-exports from starter-client-ts
```

`renderer/` is one file per variant — verb-per-file applied to
React components. Each is ≤ 150 LOC.

## `<SduiPage>` — the top-level component

```tsx
type SduiPageProps = {
    pageRef: string;
    targetRef?: string;
    stack?: Record<string, string>;
    initialPageState?: Record<string, unknown>;
};

export function SduiPage({ pageRef, targetRef, stack, initialPageState }: SduiPageProps) {
    const [pageState, setPageState] = useState(initialPageState ?? {});
    const caps = useClientCapabilities();
    const { data, error } = useResolve({ pageRef, targetRef, stack, pageState, caps });

    useSubscriptions(data?.subscriptions ?? [], () => /* refetch slot data */);

    if (error) return <ErrorBanner error={error} />;
    if (!data) return <Skeleton />;
    return (
        <PageStateContext.Provider value={[pageState, setPageState]}>
            <Render node={data.render.root} />
        </PageStateContext.Provider>
    );
}
```

`PageStateContext` holds the in-flight `page_state` object;
widgets that declare `page_state_key` write into it on user input
(toggle, slider, date_range, select), which **re-runs**
`use-resolve` so server-bound `$page.*` reflows live.

## Transport interface

The package never imports `fetch` directly, never calls
`/api/v1/ui/*` URLs, and never reads `process.env`. **All** I/O
crosses a single injected boundary:

```ts
export interface SduiTransport {
    resolve(req: ResolveRequest): Promise<ResolveResponse>;
    action(req: ActionRequest): Promise<ActionResponse>;
    table(req: TableRequest): Promise<TableResponse>;
    subscribe(
        subjects: Subject[],
        onUpdate: (subj: Subject, value: unknown) => void,
    ): () => void;
}

export const SduiTransportContext = createContext<SduiTransport | null>(null);

export function useSduiTransport(): SduiTransport {
    const t = useContext(SduiTransportContext);
    if (!t) throw new Error("Wrap your tree in <SduiTransportProvider transport={...} />");
    return t;
}
```

All four hooks (`use-resolve`, `use-action`, `use-table`,
`use-subscriptions`) consume the transport **only** via
`useSduiTransport()`. They must not import anything from
`@nube/rubix-client-react`, `axios`, `eventsource`, or any
other I/O library. Reviewer's lint check: `grep -r 'fetch\|EventSource\|axios' packages/starter-ui-sdui-react/src` returns nothing.

Rubix's frontend supplies an `SduiTransport` impl in
`rubix/frontend/src/lib/sdui-transport.ts` (≤ 80 LOC) that uses
the existing `@nube/rubix-client-react` fetcher plus an SSE
listener at `GET /api/v1/ui/subscribe?subjects=...`. The app
shell wraps the router subtree in
`<SduiTransportProvider transport={rubixSduiTransport}>`.

## Capability handshake (R7)

`use-client-capabilities.ts` walks the `renderer/registry.ts`
keys to produce a `ClientCapabilities` payload:

```ts
{
    ir_versions: [5],
    custom_renderers: ["rubix.alarm-table", "starter.markdown", ...]
}
```

Sent on every `/resolve`. The server downgrades unknown variants
to `Dangling` (already shipping). For chatty clients, the W7-style
session-id + hash optimisation lands in v2 — out of scope for v1.

## Subscription protocol — wire shape

The SDUI-routes side returns `subscriptions: Subject[]`. The
package emits one SSE connection per page-resolve to
`GET /api/v1/ui/subscribe?subjects=<csv>` (rubix-side implementation
is a deferred follow-up — until SSE ships, `useSubscriptions`
polls every 15 s as a fallback). Both flows call the same
`onUpdate(subject, value)` callback so the renderer stays
transport-agnostic.

For v1 the rubix backend will ship the polling fallback only;
the SSE/WS implementation is documented as a v2 milestone in
[08-open-questions.md](./08-open-questions.md).

## Action dispatch — `<Form>` / `<Button>` / `<Row.Action>` / `<Toolbar.Action>`

When a user clicks an `Action`, the renderer:

1. Optimistically updates `page_state` if `OptimisticHint` is set.
2. Confirms via `ConfirmDialog` if attached.
3. Calls `transport.action({ handler, args, context })`.
4. Renders the typed `ActionResponse`:
   - `Toast` → calls the host's `useToast()` hook.
   - `NavigateTo` → calls the host's router (`Link` style).
   - `Diagnostics` → routes errors back into the form's
     `FieldError`s (and the page's diagnostic panel).
   - `Refresh` → invalidates the resolve query.

`useAction()` returns `{ run, isPending, error }`. One file
(`use-action.ts`, ≤ 100 LOC).

## Theme integration

The package consumes theme tokens via CSS variables only — no
direct import of `@nube/starter-ui-theme`. This keeps the
package theme-agnostic; rubix paints its colours via the existing
theme CSS without the package needing to know.

## What replaces `dashboard.tsx`

Today: 200 LOC of hand-coded React with `SPARK_DEVICES`,
`LOAD`, `ACTIVITY_SEEDS` arrays and one real `useDiskUsage`
hook ([`rubix/frontend/src/routes/dashboard.tsx`](../../../frontend/src/routes/dashboard.tsx)).

After: 6 LOC.

```tsx
import { SduiPage } from '@nube/starter-ui-sdui-react';
export const Route = createFileRoute('/dashboard')({
    component: () => <SduiPage pageRef="dashboard.overview" />,
});
```

The seeded `dashboard.overview` bundled page (covered in
[01-storage.md](./01-storage.md)) re-creates the existing visual
in IR — `kpi_grid`, `chart`s, `card`s — using the typed builder
DSL from [`starter-ui-builder`](../../../../crates/starter-ui-builder/).
That page becomes the *first* fixture that proves the substrate
fixes from [02-bindings-gaps.md](./02-bindings-gaps.md).

## Tests in the same diff

- Vitest fixtures for each renderer/* file using a snapshot of
  the resolved tree from `starter-sdui-routes`' integration tests
  (same JSON, no live server).
- Playwright spec `rubix/frontend/e2e/dashboard-sdui.spec.ts`:
  bundled disk overview renders, KPI tile reflects the live disk
  percent via subscription update (polled fallback acceptable).

## Acceptance

1. `pnpm -F @nube/starter-ui-sdui-react build` succeeds with the
   17 renderer files, the four hooks, and `SduiPage`.
2. `rubix/frontend/src/routes/dashboard.tsx` collapses to the
   6-LOC route definition above; the rendered page matches the
   existing demo visually within tolerance (1px shifts allowed).
3. Toggling a `Toggle` widget bound to a `$page.foo` key
   re-resolves the page server-side and updates dependent widgets
   within one resolve round-trip.
4. The Custom renderer slot accepts a rubix-registered renderer
   id (e.g. `"rubix.alarm-table"`) without the package itself
   importing rubix.
