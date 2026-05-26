# PENDING — rubix_sdui

Implementation backlog. Stages mirror
[`rubix/docs/design/sdui/renderer/FLUTTER.md`](../../../docs/design/sdui/renderer/FLUTTER.md).

## F1 — backend gate (blocking, not in this package)

- [ ] Mount `starter-sdui-routes::sdui_router` in `rubix-agent`.
- [ ] Implement stub `EntityGraph` / `PageProvider` / `QueryEngine` / `HandlerRegistry` over rubix's storage.
- [ ] Merge SDUI paths into the agent's OpenAPI doc; refresh `rubix/openapi.json`.
- [ ] Regen `rubix_api`; verify `UiApi` / `UiTableApi` (or whatever generator names them) appear.

## F3 — models

- [ ] Flesh out `SduiComponent.fromJson` for all 39 IR variants (currently only the sealed shape + sentinel parsing).
- [ ] Add `toJson` round-trips for every variant.
- [ ] Generate variants from `crates/starter-ui-ir/schema/starter-ui-ir.schema.json` via a `tool/gen_sdui.dart` script.
- [ ] Add `SduiPatch.apply(JsonValue)` and walk over `ComponentTree`.

## F4 — client

- [ ] Wire `SduiService.resolve` to `RubixApi.getUiApi().resolve(...)` once available.
- [ ] Wire `SduiService.dispatchAction` to `getUiApi().action(...)`.
- [ ] Wire `SduiService.queryTable` to the GET `/ui/table` endpoint.

## F5 — state

- [ ] `SduiNotifier.load` — replace stub with real `SduiService.resolve` call.
- [ ] `SduiNotifier.dispatchAction` — handle all 10 `SduiActionResponse` variants.
- [ ] `SduiNotifier.writeControl` — open question: action-only or restore `/api/v1/slots`. See FLUTTER.md §5 Q1.
- [ ] `SduiNotifier.pushSlotEvent` — SSE bridge port (blocked on agent SSE endpoint).
- [ ] Side-effect stream wiring (toast / navigate / dialog / download).

## F6 — widgets, Wave 1

- [ ] `SduiRenderer._buildComponent` — fill in the dispatch arms.
- [ ] Layout: `page`, `row`, `col`, `grid`, `tabs`, `section`, `divider`, `spacer`.
- [ ] Display: `text`, `heading`, `badge`, `markdown`, `kpi`, `kpi_grid`.
- [ ] Input: `toggle`, `slider`, `select`, `text_field`, `number_field`, `checkbox`, `segmented`.
- [ ] Composite: `form`, `card`.
- [ ] Interactive: `button`.
- [ ] Sentinels: `dangling`, `forbidden`, `custom`.

## F6 — widgets, Wave 2

- [ ] Data: `table` (paginated), `tree`, `timeline`, `list`, `detail`, `array_table`, `json_table`.
- [ ] Display: `chart` (fl_chart), `sparkline`, `diff`, `code`, `icon`, `image`.
- [ ] Input: `ref_picker`, `date_field`, `date_range`, `radio_group`, `textarea`, `select_field`, `markdown_editor`, `rich_text`.
- [ ] Interactive: `link`, `menu`, `dialog`, `drawer`, `toast`.
- [ ] Composite: `wizard`, `header`, `field_group`, `repeat`.
- [ ] `action_widget` escape hatch.

## F7 — tests

- [ ] Round-trip fixture per variant (drive from `crates/starter-ui-ir/schema/`).
- [ ] `SduiNotifier` unit tests: load, dispatch, patch, full_render, diagnostics, version mismatch.
- [ ] Golden widget tests for Wave 1 set.

## F8 — wire into app

- [ ] `rubix/flutter/lib/features/sdui/` — `SduiPage` shell.
- [ ] Route `/sdui/:pageRef` in `app_router.dart`.
- [ ] Side-effect listener: `ScaffoldMessenger.showSnackBar`, `context.go`, `showDialog`, `url_launcher`.
- [ ] Add `rubix_sdui: { path: packages/rubix_sdui }` to the app's `pubspec.yaml` (skipped in this scaffold to avoid breaking `flutter pub get`).
