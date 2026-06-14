# Rubix Provision — Flutter

Scan a device. Place it. Done. A Flutter port of the React + Tauri
`../provision-app`, keeping the same **Grid Pulse** look (obsidian base,
electric-teal accent, frosted glass, animated mood backdrop) and the same
scan → identify → place → confirm provisioning flow.

It talks directly to a **rubix-agent** over REST (`/api/v1`), authenticating
with a `sak_` Bearer token — the Flutter app replaces the Tauri shell, so there
is no Rust core. Runs on Android, iOS, Linux/macOS/Windows desktop, and web.

## Run

```sh
flutter pub get
dart run build_runner build --delete-conflicting-outputs   # freezed/json/riverpod codegen
flutter run --dart-define=AGENT_URL=http://127.0.0.1:8088
```

`AGENT_URL` seeds the Connect screen's host field (default `http://127.0.0.1:8088`).
For a phone hitting your dev machine, pass your LAN IP or use `adb reverse tcp:8088 tcp:8088`.

## Layout

One concept per file, feature-first, per the repo's `FILE-LAYOUT.md` and the
sibling `rubix/flutter` conventions (Riverpod controllers, `package:` imports,
de-Materialized theme).

```
lib/
  core/
    theme/      tokens · app_themes (grid/solar/offpeak/industrial) · statuses ·
                look (resolved accent) · theme_providers · app_theme (ThemeData)
    api/        bc_types (freezed models) · bc_api (typed bc_* wrappers) ·
                refresh (post-write re-fetch signal) · ids
    network/    transport (Dio + Bearer port of webTransport.ts) ·
                credential_store (keychain token + prefs host) · auth_user · ping_result
  shared/widgets/   glass · glass_card · pressable · primary_button · chip ·
                    form_kit (Field/TextField/Picker/Toggle) · bottom_sheet ·
                    toast · mood_backdrop · page_header
  features/
    auth/       auth_controller (session + status) · connect_screen (gate)
    scan/       scan_flow (step machine) · scanner (mobile_scanner) · type_picker ·
                qr_label (qr_flutter) · build_add_url · build_provision_input
    identify/   identity_card
    place/      placement (model) · place_step
    provision/  toggles_step · provisioned_reveal
    devices/    devices_screen · device_detail_screen · place_on_page_sheet · status_dot
    sites/      sites_screen
    templates/  templates_screen · template_edit_sheet · template_qr_sheet
    preview/    page_preview_screen · widgets/ (stat·gauge·battery·counter·led·toggle·line·tile)
  router/       app_router (gate + StatefulShellRoute) · app_shell · nav_bar ·
                top_bar · pages (tab registry)
  app.dart · main.dart
```

## How the React app maps over

| React | Flutter |
|---|---|
| Tailwind `@theme` tokens, `useLook()` | `core/theme/*`, `lookProvider` (Riverpod) |
| `webTransport.ts` (fetch + Bearer) | `core/network/transport.dart` (Dio) |
| `api/bc.ts` + `bc-types.ts` | `core/api/bc_api.dart` + `bc_types.dart` (freezed) |
| React Context (Auth/Theme/Toast) | Riverpod `Notifier`/`Provider`s |
| framer-motion `whileTap` | `Pressable` (AnimatedScale) |
| `@zxing/browser` Scanner | `mobile_scanner` |
| `qrcode.react` | `qr_flutter` |
| in-app tab state in `App.tsx` | go_router `StatefulShellRoute` + floating `NavBar` |

The Tauri-only durable offline scan queue is **not** ported (REST-only build).
Label printing opens a print window in the web app; here the "Print sticker"
button is a stub pending a platform print integration.
