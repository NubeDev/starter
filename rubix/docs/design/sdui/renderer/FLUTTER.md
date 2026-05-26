# Flutter renderer — review of `block-os-sdui-flutter` and plan to land Dart SDUI

> Cites: [SCOPE.md](../../../SCOPE.md), [renderer/README.md](./README.md)
> (React renderer), [components/README.md](../components/README.md),
> [host-glue/README.md](../host-glue/README.md),
> [crates/starter-ui-ir/src/lib.rs](../../../../../crates/starter-ui-ir/src/lib.rs),
> [crates/starter-sdui-routes/src/lib.rs](../../../../../crates/starter-sdui-routes/src/lib.rs),
> [DOCS/frontend/sdui/SCOPE.md](../../../../../DOCS/frontend/sdui/SCOPE.md),
> [DOCS/frontend/sdui/DIVERGENCE.md](../../../../../DOCS/frontend/sdui/DIVERGENCE.md).

Companion to [renderer/README.md](./README.md), which covers
`@nube/starter-ui-sdui-react`. This file covers the Dart/Flutter
side: what the reference repo
[NubeDev/block-os-sdui-flutter](https://github.com/NubeDev/block-os-sdui-flutter)
ships today, where it has drifted from the current `starter-ui-ir`
contract, and the concrete plan to land a working Flutter
renderer in `rubix/flutter/`.

---

## 1. The reference repo, in one screen

Cloned at `/tmp/sdui-review/block-os-sdui-flutter` for this review.
~3.8 k lines of Dart in `lib/`, plus two tests.

```
lib/
  sdui_flutter.dart              # barrel export
  src/
    models/                      # pure Dart — no Flutter imports
      component_tree.dart        #   ComponentTree { ir_version, root } + kSupportedIrVersion = 5
      components.dart            #   sealed SduiComponent + ~32 variants
      action.dart                #   SduiAction + sealed SduiActionResponse
      binding.dart               #   sealed BindingSpec { Short | Full }
      resolve_response.dart      #   SduiResolveResult + SduiVersionMismatchError
    client/
      sdui_service.dart          # pure Dart — wraps AgentClient.ui.resolve
    state/                       # pure Dart
      sdui_state.dart            #   immutable value class
      sdui_notifier.dart         #   ChangeNotifier ViewModel
    widgets/                     # Flutter
      sdui_provider.dart         #   InheritedNotifier
      sdui_renderer.dart         #   switch(component) dispatcher
      components/
        layout_widgets.dart      #   row · col · grid · tabs · drawer · ...
        display_widgets.dart     #   text · heading · badge · kpi · chart · diff ...
        data_widgets.dart        #   table · tree · timeline ...
        input_widgets.dart       #   toggle · slider · select · ref_picker ...
        interactive_widgets.dart #   button · ...
        composite_widgets.dart   #   form · card · wizard ...
        custom_widget.dart       #   registry + fallback stubs
```

`pubspec.yaml` depends on:

- `rubix_agent_client` (path: `../rubix-client-dart`) — provides the HTTP
  transport plus the `UiResolveRequest` / `UiResolveOk` / `UiWritePlanEntry`
  / `UiConcurrency` types.
- `flutter_markdown ^0.7.3` — for `SduiMarkdownWidget`.
- `fl_chart ^0.69.0` — for `SduiChartWidget`.

The package is structured as a publishable Dart package
(`publish_to: none` for now, but `name: rubix_sdui_flutter`).

### Layering rule

`models/`, `client/`, `state/` must not import `package:flutter`.
The reference repo enforces this informally — the dart files in
those folders only import `dart:core` plus `package:flutter/foundation.dart`
for `ChangeNotifier`. Worth keeping when we port: it makes the
binding logic testable in plain `dart test` and mirrors the React
split where `useResolve` / `useBoundWrite` are pure hooks.

### What the reference repo did well

- **Sealed `SduiComponent`** with an exhaustive `switch` in
  `sdui_renderer.dart`. Adding a variant is a compile error
  everywhere a switch is missing — same compile-time guarantee
  the Rust IR enum gives us in `starter-ui-ir`.
- **Unknown `type` degrades to `DanglingComponent`** instead of
  throwing. Matches the resolver's R7 capability filter behaviour
  (server emits `Dangling` for binding misses; client emits it for
  unknown variants).
- **Optimistic writes via JSON round-trip** in
  `SduiNotifier._applyOptimisticValue` — pragmatic, not the most
  efficient, but correct and trivially correct on rollback.
- **Live state separate from tree**: `liveValues` and `liveSeries`
  on `SduiState` are merged in by the SSE bridge so a fresh
  `/ui/resolve` doesn't wipe chart history. The note in
  `pushSlotEvent` (*"We do NOT re-resolve"*) is the right call.
- **IR-version guard at the service layer**: `SduiService.resolve`
  raises `SduiVersionMismatchError` before any widget sees the
  tree, and `SduiRenderer` paints a dedicated banner for that
  variant.

### What the reference repo got wrong / has drifted on

These are the gaps to fix when we port the package into
`rubix/flutter/`:

1. **`writes: List<UiWritePlanEntry>` doesn't exist on the current
   wire.** The repo's `SduiResolveResult` carries a write plan
   (`writes[i].componentId / path / slot / concurrency / generation`)
   and `SduiNotifier.writeBound` walks it to call
   `client.slots.writeSlot(path, slot, value, opts)`. The current
   [resolve.rs](../../../../../crates/starter-sdui-routes/src/routes/resolve.rs)
   `ResolveResponse` only contains `{ render, subscriptions }`. There
   is no separate `/slots` write path mounted by
   `starter-sdui-routes` — two-way controls go through
   `POST /api/v1/ui/action` like everything else. The Dart
   renderer must be re-wired around that.
2. **`SduiActionResponse` variants are stale.** Reference has
   `none | toast | navigate | full_render | form_errors | download | stream | patch`.
   The current `starter_ui_ir::ActionResponse` (R5) is
   `patch | full_render | navigate | toast | diagnostics | download | stream | none`
   plus starter shorthands `dialog | toast_and_refresh`. So:
     - `form_errors` → `diagnostics` (carries a `Vec<Diagnostic>`
       with `severity` + `code` + `message`, not a `Map<field, string>`).
     - Add `dialog` and `toast_and_refresh`.
3. **Component catalogue lags.** Reference covers ~32 variants;
   `starter-ui-ir` v5 has **39 portable** variants. Missing in
   the reference repo: `repeat`, `divider`, `field_group`,
   `section`, `array_table`, `json_table`, `markdown_editor`,
   `text_field`, `number_field`, `textarea`, `select_field`,
   `radio_group`, `segmented`, `date_field`, `checkbox`,
   `action_widget`. The reference also has `LinkComponent`,
   `MenuComponent`, `DialogComponent`, `ToastComponent`,
   `StackComponent`, `SplitComponent`, `ScrollComponent`,
   `SpacerComponent`, `IconComponent`, `ImageComponent`,
   `CodeComponent`, `ListComponent`, `DetailComponent`,
   `FieldComponent`, `DateComponent`, `SearchComponent` —
   some of these may have been renamed in the IR (`date_range`,
   the `*_field` family). The variant table needs a side-by-side
   diff against [`components.rs`](../../../../../crates/starter-ui-ir/src/component.rs)
   before we copy any model code over.
4. **`/ui/table` shape is different.** Reference has it as a
   `POST` style invocation through `client.ui.table(...)`. The
   current handler is a **`GET`** with query string
   `?source_id=&page=&size=&sort=&filter=`, returning
   `QueryResponse` per
   [table.rs](../../../../../crates/starter-sdui-routes/src/routes/table.rs).
5. **`rubix_agent_client` doesn't exist in this workspace.** The
   reference repo expects `package:rubix_agent_client/agent_client.dart`
   with `AgentClient.ui` and `AgentClient.slots`. We have
   [`rubix_api`](../../../../../rubix/flutter/packages/rubix_api/)
   (auto-generated Dart-Dio from `rubix/openapi.json`) and **it
   doesn't include any `/ui/*` paths** because `rubix-agent`
   doesn't mount `starter-sdui-routes` yet. That gap is the
   single biggest blocker — see §3 below.
6. **`PENDING.md` shows ~40 % of widgets unimplemented.** Notably
   `field`, `date`, `link`, `menu`, `dialog`, `toast`, `image`,
   `icon`, `code`, `list`, `detail`, `card`, `header`, `kpi_grid`,
   `stack`, `split`, `scroll`, `spacer` are all `[ ]`. The
   renderer compiles but a real page will hit `SduiUnknownWidget`
   for any of these.
7. **`SduiNotifier.dispatchAction` is a stub.** It only sets
   `SduiStatus.error` with an `UnimplementedError` — the whole
   action round-trip is unwritten.
8. **Tests are thin** — one model round-trip fixture, one widget
   test for `SduiToggle` + `SduiSlider`. No version-mismatch
   test, no diagnostics test, no dispatch-action test.

---

## 2. Mapping the reference repo to our current contract

The summary diff (read top→bottom, left to right):

| Layer | Reference repo (`block-os-sdui-flutter`) | Current contract (this workspace) | Action |
|---|---|---|---|
| IR version constant | `kSupportedIrVersion = 5` | `IR_VERSION = 5` ([lib.rs L85](../../../../../crates/starter-ui-ir/src/lib.rs#L85)) | **Match** — carry over. |
| Component variants | ~32, sealed `SduiComponent` | 39 portable, `Component` enum ([component.rs](../../../../../crates/starter-ui-ir/src/component.rs)) | **Re-derive** from the IR JSON Schema rather than copy from the repo. |
| `BindingSpec` | sealed `Short` / `Full` | `BindingSpec` ([starter-ui-ir](../../../../../crates/starter-ui-ir/src/component.rs)) | **Carry over** — wire form unchanged. |
| Resolve request | `UiResolveRequest { page_ref, target_ref?, stack, page_state, user, capabilities }` from `rubix_agent_client` | `ResolveRequest` in [resolve.rs](../../../../../crates/starter-sdui-routes/src/routes/resolve.rs) — **same shape** | **Match** once `rubix_api` exposes it. |
| Resolve response | `{ render, subscriptions, writes }` | `{ render, subscriptions }` — **no `writes`** | **Drop `writes`** from `SduiResolveResult`; delete `SduiNotifier.writeBound` + `_findEntry` + write-plan rollback. |
| Two-way bound controls | `client.slots.writeSlot(path, slot, value)` with OCC | No `/slots` endpoint in `starter-sdui-routes` | **Re-design** toggle/slider to fire an action — see §4. |
| Action request | `client.ui.action(SduiAction)` (stub) | `POST /api/v1/ui/action` with `ActionRequest { handler, args, context }` ([action.rs](../../../../../crates/starter-sdui-routes/src/routes/action.rs)) | **Implement** in `SduiNotifier`. |
| Action response variants | `none / toast / navigate / full_render / form_errors / download / stream / patch` | `patch / full_render / navigate / toast / diagnostics / download / stream / none / dialog / toast_and_refresh` | **Rename** `form_errors` → `diagnostics`; **add** `dialog`, `toast_and_refresh`. |
| Table | reference calls `client.ui.table(...)` | `GET /api/v1/ui/table?source_id=&page=&size=&sort=&filter=` returning `QueryResponse` | **Re-wire** `SduiTableWidget` to issue paginated GETs. |
| HTTP transport | `package:rubix_agent_client` | `package:rubix_api` (OpenAPI-generated Dart-Dio) | **Re-target** every `client.*` call onto `RubixApi.getUiApi()` (does not exist yet — see §3). |
| Live updates | SSE bridge feeds `SduiNotifier.pushSlotEvent` | Bridge not implemented in `rubix/flutter/` | **Port** the SSE bridge — but only after the agent ships an SSE endpoint for `slot_changed`. Park behind a feature flag. |

---

## 3. Blocker: `rubix-agent` doesn't expose `/api/v1/ui/*` yet

`starter-sdui-routes::sdui_router` is a `Router<S>` builder; no one
in `rubix-agent` calls it today. Confirmed by:

- `rubix/openapi.json` has zero `/api/v1/ui/*` paths (only `theme`).
- The root `openapi.json` also only has the theme endpoints.
- `rubix/flutter/packages/rubix_api/` therefore has no `UiApi` class.

A Flutter renderer with no resolve endpoint to call is a dead
package. The order of work is:

1. **Wire `starter-sdui-routes` into `rubix-agent`.** Implement
   the four host traits from `starter-sdui-routes`
   (`EntityGraph`, `PageProvider`, `QueryEngine`,
   `HandlerRegistry`) over rubix's existing storage + flow
   surfaces — there's a stub-friendly initial pass that returns
   a fixed demo page and an empty handler registry, just to get
   the routes mounted and serving JSON.
2. **Add the routes to the agent's `OpenApiDoc` merge.** Mirror
   the auth-merge pattern already in
   [openapi.rs](../../../../../rubix/crates/rubix-agent/src/openapi.rs)
   so `make api-client` regenerates a `UiApi` class in
   `rubix_api`.
3. **Refresh `rubix/openapi.json`.** Verify
   `tests/openapi_test.rs` covers the new paths.
4. **Only then** start the Flutter port.

Skipping (1)–(3) means writing the Flutter package against a
hand-rolled Dio call that will get replaced by the generated
client a week later. Don't do that.

---

## 4. Plan for the Flutter package in `rubix/flutter/`

Target location: `rubix/flutter/packages/rubix_sdui/` (sibling
to `rubix/flutter/packages/rubix_api/`). Use `path:` deps inside
the Flutter app — no pub.dev publish until we have a consumer
outside `rubix/`.

### Stage F1 — backend gate (prerequisite)

- [ ] Mount `starter-sdui-routes::sdui_router` in `rubix-agent`.
      Start with stub `EntityGraph` / `PageProvider` / `QueryEngine` /
      `HandlerRegistry` impls so the routes serve a tiny seeded
      demo page.
- [ ] Merge the SDUI paths into the agent's OpenAPI doc and snap
      `rubix/openapi.json`.
- [ ] Run `make api-client` and confirm `RubixApi.getUiApi()` +
      `getUiTableApi()` (or whatever the generator names them)
      appear in `packages/rubix_api/`.

### Stage F2 — package scaffold

- [ ] Create `rubix/flutter/packages/rubix_sdui/` with
      `pubspec.yaml` (`publish_to: none`, path dep on
      `../rubix_api`).
- [ ] Lift `lib/sdui_flutter.dart` + the layered `src/` tree
      structure from the reference repo. **Do not copy the
      code yet** — copy the shape only, with files containing
      `library …;` stubs.

### Stage F3 — models (pure Dart)

- [ ] `src/models/component_tree.dart` —
      `kSupportedIrVersion = 5`, `ComponentTree` wrapper.
- [ ] `src/models/components.dart` — regenerate the sealed
      `SduiComponent` variant list from the IR JSON Schema at
      [crates/starter-ui-ir/schema/starter-ui-ir.schema.json](../../../../../crates/starter-ui-ir/schema/starter-ui-ir.schema.json)
      so the 39 variants stay in sync without hand transliteration.
      A tiny script under `rubix/flutter/tool/gen_sdui.dart` is
      the right place — runs alongside `make api-client`.
- [ ] `src/models/action.dart` — `SduiAction` (the IR-level
      action reference) plus sealed `SduiActionResponse` with
      the **current** 10 variants (note `diagnostics`,
      `dialog`, `toast_and_refresh`).
- [ ] `src/models/binding.dart` — sealed `BindingSpec`
      (`Short` / `Full`) — straight port from reference.
- [ ] `src/models/resolve_response.dart` — `SduiResolveResult`
      with `{ tree, subscriptions }` only (no `writes`).
      `SduiVersionMismatchError` carries over unchanged.

### Stage F4 — client (pure Dart)

- [ ] `src/client/sdui_service.dart` — wraps `RubixApi.getUiApi()`.
      Three methods: `resolve(ResolveRequest)`, `dispatchAction(ActionRequest)`,
      `queryTable(TableQuery)`. Each one parses the
      generator-produced DTOs back into the typed model
      classes, runs the IR-version guard, and surfaces
      `SduiServerError` / `SduiVersionMismatchError`.

### Stage F5 — state (pure Dart, `ChangeNotifier` first; Riverpod adapter optional later)

- [ ] `src/state/sdui_state.dart` — drop `writes` field; keep
      `liveValues` / `liveSeries`.
- [ ] `src/state/sdui_notifier.dart`:
  - [ ] `load(...)` — re-implement against `SduiService.resolve`.
  - [ ] `dispatchAction(SduiAction, {context})` — real
        implementation, switches on the response variants,
        applies `patch` via a JSON-Patch lib (`json_patch ^3.x`),
        re-issues `load()` on `full_render`, surfaces
        `diagnostics`, emits `toast` / `navigate` / `download` /
        `dialog` / `toast_and_refresh` via a side-channel
        `Stream<SduiSideEffect>` for the widget layer.
  - [ ] **`writeControl(componentId, value)`** — new helper that
        composes a synthetic `SduiAction` against the component's
        declared write handler (the `bind` field carries the
        target; the action handler name is conventionally
        `slot.write` until we formalise it). Replaces the old
        `writeBound` write-plan path entirely.
  - [ ] `pushSlotEvent(...)` — port the SSE bridge logic
        unchanged. Wire to a real SSE source only when the
        agent ships one.

### Stage F6 — widgets (Flutter)

Land in two waves so the package becomes useful early:

**Wave 1 — minimum useful set** (matches what the seeded demo
page emits):

- [ ] `sdui_provider.dart`, `sdui_renderer.dart`.
- [ ] Layout: `page`, `row`, `col`, `grid`, `tabs`, `section`,
      `divider`, `spacer` (use `SizedBox` for spacer).
- [ ] Display: `text`, `heading`, `badge`, `markdown` (via
      `flutter_markdown`).
- [ ] Input: `toggle`, `slider`, `select`, `text_field`,
      `number_field`, `checkbox`, `segmented`.
- [ ] Composite: `form`, `card`, `kpi`, `kpi_grid`.
- [ ] Interactive: `button`.
- [ ] Sentinels: `dangling`, `forbidden`, `custom` (registry +
      unknown stub).

**Wave 2 — full catalogue**:

- [ ] Data: `table` (paginated, calls `/ui/table`), `tree`,
      `timeline`, `list`, `detail`, `array_table`, `json_table`.
- [ ] Display: `chart` (fl_chart), `sparkline`, `diff`, `code`,
      `icon`, `image`.
- [ ] Input: `ref_picker`, `date_field`, `date_range`,
      `radio_group`, `textarea`, `select_field`, `markdown_editor`,
      `rich_text`.
- [ ] Interactive: `link`, `menu`, `dialog` (`showDialog`),
      `drawer`, `toast` (SnackBar — driven by side-effect stream).
- [ ] Composite: `wizard` (`Stepper`), `header`, `field_group`,
      `repeat` (renderer-side fan-out using `EvalContext`).
- [ ] `action_widget` — escape hatch for handler-driven widgets.

### Stage F7 — tests

- [ ] `test/models/components_test.dart` — round-trip every
      variant against fixtures from
      [`crates/starter-ui-ir/schema/`](../../../../../crates/starter-ui-ir/schema/)
      and the React renderer's fixtures.
- [ ] `test/state/sdui_notifier_test.dart` — load, dispatch,
      patch, full_render, diagnostics, version mismatch.
- [ ] `test/widgets/sdui_renderer_test.dart` — golden-tree
      widget tests for the Wave 1 set.

### Stage F8 — wire into the Rubix app

- [ ] New route `/sdui/:pageRef` in
      [app_router.dart](../../../../../rubix/flutter/lib/core/router/app_router/app_router.dart)
      hosting a `SduiPage` that constructs an `SduiNotifier`,
      calls `load(pageRef: ...)`, wraps `SduiRenderer` in
      `SduiProvider`.
- [ ] Listen on the side-effect stream from `SduiPage` to fire
      `ScaffoldMessenger.showSnackBar` (toast), `context.go(url)`
      (navigate), `showDialog` (dialog), `url_launcher` (download).

---

## 5. Open questions

1. **Bound-control writes** — is the long-term plan to bring
   back a `/api/v1/slots` endpoint (matching the reference repo)
   or to keep two-way controls in the unified `/ui/action`
   channel? The Dart port pivots on this; pick before Stage F5.
2. **Subscription transport** — does rubix expose `slot_changed`
   over SSE, WebSocket, or NATS? The reference repo assumes SSE
   via a `pushSlotEvent` callback driven by an external bridge.
   No bridge code is in this workspace yet.
3. **Generator naming** — confirm what `openapi-generator-cli`
   names the SDUI API class. Probably `UiApi` from the `tags`
   field in `starter-sdui-routes`; we may want a tag override
   in `openapi.rs` to keep it predictable.
4. **Vendoring vs path-dep** — keep the package inside
   `rubix/flutter/packages/rubix_sdui/` (current proposal) or
   publish to a private repo and pull in over `git:` so the
   demos under `examples/` can consume it too? Mirrors the same
   question we already answered for `rubix_api` (kept local).

---

## 6. TL;DR

The reference repo is a useful skeleton but has rotted against
the current IR contract. Real port work is small —
~40 % of the renderer files are reusable as-is — but it is
**blocked on `rubix-agent` actually mounting
`starter-sdui-routes`**. Land that first, regenerate
`rubix_api`, then walk Stages F2–F8 above.

Net new code expected: ~1.5 k lines for Wave 1 (usable),
~3.5 k lines for Wave 2 (full catalogue). No new Rust crates
needed — `starter-sdui-routes` already does the server side.
