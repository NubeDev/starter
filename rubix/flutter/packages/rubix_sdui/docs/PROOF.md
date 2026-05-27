# Flutter SDUI — page/kpi/chart proof

Minimum end-to-end render: fetch a real `ComponentTree` from
`/api/v1/ui/resolve` and project `page` (title) + `row` + `col` +
`kpi` + `chart` to Flutter widgets. First slice through the package
to validate the model parsers, dispatch shape, service transport,
state plumbing, and the bridge into the host app.

This doc is the **scope + progress tracker** for that slice. It does
**not** replace [`PENDING.md`](../PENDING.md) (the full F1–F8 backlog)
or [`FLUTTER.md`](../../../../docs/design/sdui/renderer/FLUTTER.md)
(the renderer design). When this proof lands, the items below come
off `PENDING.md` and this doc is archived.

## Acceptance

Open `rubix/flutter` on the running dev stack
([`Makefile`](../../../../Makefile), `make start` →
`http://127.0.0.1:8088`), authenticate against the bootstrap user
(`op@example.com` / `rubix-dev-passwd`), navigate to
`/sdui/dashboard.data-flow-site-a`, see:

- The page title `Site A — energy + water` painted as a header.
- A horizontal row of two KPIs (`Site A — last 24h kWh` and
  `Site A — last 24h L`), each in its own 6-span `col`.
- Below it, a second row containing the `Electricity — main
  (30d, 15m)` line chart sourced from the embedded `series[].points`
  the resolver inlines.

Backend gate is **green**: `starter-sdui-routes` is already mounted
in `rubix-agent` (verified via `boot/sdui.rs` + a live curl against
`/api/v1/ui/resolve`). The proof is entirely a Flutter-side
implementation.

## Wire shape we render against

`POST /api/v1/ui/resolve` body:

```json
{ "page_ref": "dashboard.data-flow-site-a" }
```

Response (abridged, real payload from the live agent on
2026-05-27):

```json
{
  "render": {
    "ir_version": 5,
    "root": {
      "type": "page", "id": "root", "title": "Site A — energy + water",
      "children": [
        { "type": "row", "id": "kpis", "children": [
            { "type": "col", "span": 6, "children": [
                { "type": "kpi", "id": "kpi-kwh-24h",
                  "label": "Site A — last 24h kWh",
                  "value": 10002.42, "format": "number",
                  "unit_symbol": "kWh" } ] },
            { "type": "col", "span": 6, "children": [ /* kpi */ ] }
          ] },
        { "type": "row", "id": "charts", "children": [
            { "type": "col", "children": [
                { "type": "chart", "id": "chart-elec-main",
                  "title": "Electricity — main (30d, 15m)",
                  "kind": "line",
                  "series": [
                    { "label": "meter_value_30d_15m",
                      "points": [[1779801300000, 10005.22], … ] } ] } ] } ] }
      ]
    }
  },
  "subscriptions": []
}
```

Two server-side facts worth pinning:

- `kpi.value` arrives **already resolved** — the analytics template
  evaluated server-side, the client doesn't refetch.
- `chart.series[].points` is inlined too — the resolver baked the
  historical query into the response. No `/ui/table` round-trip
  needed for the chart proof.

Both decisions are server-side concerns; the client just renders
literals.

## Tasks

In order. Tick as we land each piece — one PR-sized commit per
task is fine, the whole slice in one is also fine.

### Package `rubix_sdui` — transport + state

- [ ] **T1. `client/sdui_service.dart`** — implement `resolve` over
      raw Dio. Constructor `SduiService({required Dio dio, String
      baseUrl = '/api/v1'})`. Posts to `/ui/resolve` with the
      `ResolveRequest.toJson()` body. Parses `render` →
      `ComponentTree`, `subscriptions[]` → `List<SduiSubject>`.
      Throws `SduiVersionMismatchError` when
      `tree.irVersion > kSupportedIrVersion`; wraps everything else
      in `SduiServerError`. `dispatchAction` / `queryTable` stay
      `UnimplementedError` — out of proof scope.

      *Decision:* skip the `rubix_api` codegen path entirely for
      this slice. Per [`PENDING.md`](../PENDING.md) F1 the generator
      hasn't been refreshed; building this proof against a hand-rolled
      Dio call unblocks F6 without waiting on F1, and the migration to
      the generated client is a search-replace inside one file when it
      lands.

- [ ] **T2. `state/sdui_notifier.dart`** — flesh out `load`:
      set `status: loading`, await `_service.resolve(...)`, swap
      state to `loaded` with the parsed tree + subscriptions. On
      `SduiVersionMismatchError` keep the error verbatim so the
      banner in [`sdui_renderer.dart`](../lib/src/widgets/sdui_renderer.dart)
      handles it. On everything else stash the exception as
      `state.error` with `status: error`. `dispatchAction` /
      `writeControl` / `pushSlotEvent` stay stubs.

### Package `rubix_sdui` — widgets

- [ ] **T3. `widgets/components/layout_widgets.dart`** —
      `SduiPageWidget`, `SduiRowWidget`, `SduiColWidget`.

      - `page`: read `raw['title']` → optional `String`, paint as
        a `Text(theme.textTheme.headlineSmall)` above a `Column` of
        children. Wrap the whole thing in a scrolling `ListView` so
        long pages don't overflow on phones.
      - `row`: `Row(crossAxisAlignment: start)` containing each
        child wrapped in `Expanded(flex: child.span ?? 12)`. A child
        whose `type` isn't `col` is still rendered — flex defaults
        to 12 so it spans the row. (Strict-mode rejection is the
        server's job per `rubix-tools/.../layout.rs`.)
      - `col`: `Column(crossAxisAlignment: stretch)` of children
        with a vertical gap (`SizedBox(height: 12)`).

      All three recurse via the exported `buildComponent` from
      `sdui_renderer.dart`. Children parsed once at widget-build
      time from `raw['children'] as List`, each map → `SduiComponent.fromJson`.

- [ ] **T4. `widgets/components/display_widgets.dart`** —
      `SduiKpiWidget`, `SduiChartWidget`.

      - `kpi`: card-shaped container, label on top
        (`labelLarge`), value below in display size with the
        `unit_symbol` appended. Formatting: `format == "percent"`
        appends `%`, `format == "number"` rounds to 2 decimals;
        anything else falls back to `value.toString()`. Source of
        the value is `raw['value']` — already resolved by the
        server.
      - `chart`: `fl_chart` `LineChart` from the first entry of
        `raw['series']`. X axis = `points[i][0]` (ms epoch), Y =
        `points[i][1]`. Min/max derived from the series itself. Title
        rendered above the chart. Empty `points` → "no data" stub.
        Multi-series is out of proof scope (one line in v0; the
        loop is a one-liner to extend later).

- [ ] **T5. `widgets/sdui_renderer.dart`** — fill the dispatch
      arms in `buildComponent`. Order: `PageComponent`,
      `RowComponent`, `ColComponent`, `KpiComponent`,
      `ChartComponent`. Everything else stays at
      `SduiUnknownWidget` — the surrounding sentinel arms stay
      intact.

### App `rubix_flutter` — host

- [ ] **T6. `pubspec.yaml`** — add `rubix_sdui: { path:
      packages/rubix_sdui }` under `dependencies`. PENDING F8
      flagged this as deferred to avoid breaking `flutter pub get`;
      with the widgets compiling this is safe.

- [ ] **T7. `lib/features/sdui/presentation/sdui_page_screen.dart`** —
      a `ConsumerStatefulWidget` that:

      1. Reads the active connection's authenticated Dio out of the
         existing Riverpod graph (whatever
         [`features/auth`](../../../../lib/features/auth) /
         [`features/connections`](../../../../lib/features/connections)
         already provide — reuse, don't re-do auth).
      2. Builds an `SduiService(dio: …)` and an `SduiNotifier`,
         calls `notifier.load(pageRef: pageRef)` in `initState`,
         disposes the notifier in `dispose`.
      3. Wraps `SduiProvider` around `SduiRenderer` inside a
         `Scaffold` body. AppBar shows the route param; the page
         title rendered by `SduiPageWidget` is the in-content title
         (intentional duplication — the app chrome stays whether
         resolve succeeds or not).

- [ ] **T8. `core/router/app_router/app_router.dart`** — add
      `GoRoute(path: '/sdui/:pageRef', builder: …)`. Keep it
      outside the auth-shell stack for now so it's reachable for
      manual testing without re-rigging the connection PIN flow;
      tighten later.

- [ ] **T8b. Navigation entry — `Dashboards` tab.** A standalone
      `/sdui/:pageRef` route is great for deep-linking but
      undiscoverable in the app. Add a 4th `StatefulShellBranch` at
      `/dashboards` with a screen
      ([`features/sdui/presentation/dashboard_list_screen.dart`](../../../../lib/features/sdui/presentation/dashboard_list_screen.dart))
      that posts to `/api/v1/tools/rubix.dashboard.list` with
      `{tenant_id: 'system'}` and renders each `items[]` entry as a
      `ListTile` whose tap pushes `/sdui/<page_id>`. Source goes
      through Dio directly (same rationale as T1 — skip
      `rubix_api` codegen until F1 refreshes).

      Tab placement in [`app_shell.dart`](../../../../lib/core/router/app_shell/app_shell.dart):
      Home · **Dashboards** · Connections · Settings, icon
      `LucideIcons.layoutDashboard`. Label hardcoded `'Dashboards'`
      until the ARB regen lands (`flutter gen-l10n`); add
      `dashboards` key to `app_en.arb` / `app_es.arb` in the same
      commit if regen runs. Tenant scoping is hardcoded to
      `system` — picker lands when the rest of the app grows
      multi-tenant support.

### Verification

- [ ] **T9.** `flutter test` in `packages/rubix_sdui` — extend
      [`sdui_smoke_test.dart`](../test/sdui_smoke_test.dart) with a
      fixture-driven render test: feed a hand-built tree
      (`page → row → col[span:6] → kpi`) through `SduiRenderer`
      under `pumpWidget`, assert the title and the KPI label
      appear. No HTTP — the service stays mocked at the notifier
      seam.

- [ ] **T10.** Manual smoke: `make start`, launch the Flutter app,
      log in, hit `/sdui/dashboard.data-flow-site-a`, verify the
      three acceptance bullets above.

## Out of scope (deliberately)

- `action` dispatch round-trip (`POST /ui/action`) — no buttons in
  the fixture. Land with the `button` widget in a follow-up.
- `table` paging (`GET /ui/table`) — the data-flow page has no
  table. Lands with `TableComponent` in F6 Wave 2.
- Live updates / SSE bridge — the resolver returned an empty
  `subscriptions` array on this page. Bridge lands when the first
  page with a real subscription plan does.
- Binding parsing on the client. Per
  [`SCOPE.md`](../../../../../DOCS/frontend/sdui/SCOPE.md) R4,
  bindings are server-resolved; the client only ever sees literals.
- Multi-series charts, sparklines, bar/area variants. One line on
  one chart is enough to prove the dispatch and the fl_chart wiring.
- Theme tokens. Default Material colours for v0; the theme bridge
  to `starter-ui-kit` tokens is a separate piece.

## Risks / open

- **`rubix_api` codegen drift.** This proof side-steps the
  generated client. When F1 refreshes `rubix/openapi.json` and the
  `UiApi` class appears, swap T1's Dio call for the generated one
  and delete the hand-rolled body. The seam is `SduiService` so
  the rest of the package doesn't move.
- **fl_chart version.** Pinned to `^0.69.0` in
  [`pubspec.yaml`](../pubspec.yaml). API has moved between minors
  before; if `flutter pub get` resolves a newer breaking minor,
  pin tighter.
- **Auth coupling.** The screen reuses the host app's authenticated
  Dio. If the active-connection plumbing changes shape, T7 needs to
  follow it — not a package concern.

## Definition of done

T1–T10 ticked, `flutter test` green in
`packages/rubix_sdui`, and the manual smoke renders the three
acceptance bullets against the live agent. At that point the
package has cleared the F6 Wave 1 floor for the five variants
named here and the F4/F5 transport+state work for `resolve` /
`load`. Wave 1's other variants (`text`, `heading`, `badge`,
`markdown`, `kpi_grid`, inputs, `form`, `card`, `button`) follow
the same pattern and unblock as their pages need them.
