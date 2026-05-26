# rubix_sdui

Flutter renderer for the Rubix Server-Driven UI (SDUI) IR.

This package is a **scaffold only** — the directory layout and the
file boundaries are in place but most bodies are
`UnimplementedError` stubs. The full implementation plan lives in
[`rubix/docs/design/sdui/renderer/FLUTTER.md`](../../../docs/design/sdui/renderer/FLUTTER.md)
(stages F2–F8).

## Status

| Stage | Status |
|---|---|
| F1 — `starter-sdui-routes` mounted in `rubix-agent` | **Blocking. Not started.** |
| F2 — package scaffold | Done (this commit). |
| F3 — models (pure Dart) | Stubs in place. |
| F4 — client (pure Dart) | Stubs in place. |
| F5 — state (pure Dart) | Stubs in place. |
| F6 — widgets (Flutter), Wave 1 | Stubs in place. |
| F6 — widgets (Flutter), Wave 2 | Not started. |
| F7 — tests | Smoke test only. |
| F8 — wire into app | Not started. |

## Layout

```
lib/
  rubix_sdui.dart                 # barrel — public API
  src/
    models/                       # pure Dart — zero Flutter imports
      ir_version.dart             #   kSupportedIrVersion = 5
      component_tree.dart         #   ComponentTree { irVersion, root }
      component.dart              #   sealed SduiComponent + all 39 variants
      binding.dart                #   sealed BindingSpec { Short | Full }
      action.dart                 #   SduiAction (IR action reference)
      action_response.dart        #   sealed SduiActionResponse (10 variants)
      diagnostic.dart             #   Diagnostic, Severity (mirrors R5)
      patch.dart                  #   SduiPatch — one JSON Patch op
      resolve.dart                #   ResolveRequest + SduiResolveResult + errors
      table_query.dart            #   TableQuery + TableResponse
    client/
      sdui_service.dart           #   facade over RubixApi.getUiApi()
    state/                        # pure Dart
      sdui_status.dart            #   idle | loading | loaded | error
      sdui_state.dart             #   immutable snapshot
      side_effect.dart            #   sealed SduiSideEffect (toast/nav/dialog/...)
      sdui_notifier.dart          #   ChangeNotifier orchestrator
    widgets/                      # Flutter
      sdui_provider.dart          #   InheritedNotifier
      sdui_renderer.dart          #   switch(component) dispatcher
      components/
        layout_widgets.dart       #   page · row · col · grid · tabs · section · divider · spacer
        display_widgets.dart      #   text · heading · badge · markdown · kpi · kpi_grid
        input_widgets.dart        #   toggle · slider · select · *_field · checkbox · segmented
        interactive_widgets.dart  #   button
        composite_widgets.dart    #   form · card
        sentinel_widgets.dart     #   dangling · forbidden · custom · unknown
        custom_registry.dart      #   CustomRendererRegistry
test/
  sdui_smoke_test.dart            # imports + compile check
```

## Layering rule

`models/`, `client/`, `state/` **must not** import
`package:flutter`. Only `widgets/` imports Flutter. This keeps the
binding logic testable in plain `dart test` and mirrors the
React renderer's split (logic in hooks, projection in components).

## Why this is blocked on backend work

`rubix-agent` does not currently mount `starter-sdui-routes`, so
`rubix/openapi.json` has no `/api/v1/ui/*` paths and the generated
[`rubix_api`](../rubix_api/) Dio client has no `UiApi`. Until
stage F1 lands, this package cannot compile against a real
endpoint — the stubs throw `UnimplementedError` so the layout
holds the shape without lying about what works.
