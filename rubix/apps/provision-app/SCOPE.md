# SCOPE.md — Provision App (cross-platform scan-to-dashboard client)

**Status:** **planned** — nothing scaffolded yet. This document is the
source of truth the UI and the Tauri Rust core follow.
**Owner dir:** `rubix/apps/provision-app/` (a new, standalone Tauri v2
workspace — **not** inside the `com.nubeio.rubixos` extension, **not** a
fork of `rubix-agent`).
**One-line:** *One React codebase, shipped as desktop (macOS/Win/Linux),
iOS, Android, and browser — the field/admin client for the existing
"scan-to-dashboard" provisioning feature: scan a sticker (or pick a
type) → place it on a site's dashboard → it trends and alarms — and it
works offline, queuing the provision until the gateway is reachable.*

This app is a **pure client**. It owns no warehouse data and adds no
server-side tables. It speaks to the already-built `bc_*` tool surface
documented in
[`../../extensions/com.nubeio.rubixos/BARCODE.md`](../../extensions/com.nubeio.rubixos/BARCODE.md)
(B0–B7 landed). Read that first — its four-nouns/one-verb model, the
`bc_*` tools, and the phone-PWA flow in §6.1 are exactly what this app
re-homes into a real cross-platform binary.

> **Removability:** deleting `rubix/apps/provision-app/` leaves the
> extension and the agent untouched. This directory path-deps into
> nothing in the host; it only talks HTTP to `rubix-agent` (see §10).

---

## 1. Why a Tauri app (and what it supersedes)

The extension already ships **two** clients (BARCODE.md §6): a federated
admin UI module (`./Provision`, 5 tabs) and a thin phone **PWA**
(`ui-src/pwa/`). Both are served *by the agent's UI bundle* and live
*inside the extension*. That is the right place for an admin surface
bolted onto the dashboard. It is the wrong shape for a **field tool**:

- a PWA can't reliably hold a session, a local queue, or camera/BLE
  permissions across cold starts on iOS;
- it isn't installable as a first-class app on a tech's phone or a
  kiosk laptop;
- it can't grow into native camera / BLE / NFC commissioning later;
- it couples the field client's release cadence to the extension's
  manifest-hash pinning and binary rebuilds (BARCODE.md §12 runtime
  note).

A Tauri v2 app fixes all of that from **one React codebase**:

| Want | How Tauri gives it |
|---|---|
| desktop + mobile + web from one UI | `tauri build` / `tauri ios` / `tauri android` + a plain `vite build` SPA |
| installable, persistent session | OS app + keychain-backed session store |
| native camera now, BLE/NFC later | Tauri plugins behind one Rust seam |
| **offline-first** field use | a **local SQLite queue** in the Rust core |
| no server changes | Rust core is a *thin proxy* to existing `bc_*` tools |

**Honest overlap:** this app **supersedes and absorbs the PWA flow in
BARCODE.md §6.1.** The five PWA steps (Scan → Identify → Place → Toggles
→ Confirm) are reproduced here screen-for-screen (§4). The extension's
admin `./Provision` module **stays** as the in-dashboard admin surface;
this app is the standalone field/admin client that does the same job
with offline + native reach. Once this app ships its mobile targets, the
`ui-src/pwa/` flow can be retired (tracked as a follow-up in §12).

---

## 2. The mental model (reused, not reinvented)

This app does not invent a model. It renders the one in
[BARCODE.md §1](../../extensions/com.nubeio.rubixos/BARCODE.md#1-the-mental-model--four-nouns--one-verb):

```
Network → Device → Point          ← the transport hierarchy (from the barcode/template)
Site → Location                   ← PHYSICAL: where the device sits
Site → Page → Widget              ← DISPLAY: the screen a viewer opens
```

Both **Location** and **Page** hang off the **same Site**. A device
carries a `location_id` (physical) and lands its widgets on a **page
that belongs to the same site** (`bc_pages.site_id`, BARCODE.md §4.4).
The one verb is **provision**; everything after "identity" is
deterministic and driven by the device's YAML template. The only human
decisions are **which site/location/page** and **which toggles (trend /
alarm)**. This app is a thin, beautiful surface over exactly those
decisions.

---

## 3. Architecture

```
┌──────────────────────────────────────────────────────────────────────┐
│  React 19 + TS + Vite + Tailwind v4 + Framer Motion  (the SAME UI)     │
│                                                                        │
│   screens (§4) ── call ──► transport interface  (one TS seam)          │
│                              │                                         │
│                 ┌────────────┴─────────────┐                           │
│        native target                  web target                       │
│   (desktop/iOS/Android)              (browser SPA)                      │
│           │                                │                           │
│   invoke('tool_dispatch', …)        fetch(agentUrl + '/api/v1/…')      │
└───────────┼────────────────────────────────┼──────────────────────────┘
            │                                 │
            ▼                                 │  (no Rust core in browser;
┌───────────────────────────┐                │   talks to agent directly)
│  Tauri Rust core (src-tauri)│               │
│  #[command] verbs (§6)      │               │
│    auth_login / me / logout │               │
│    tool_dispatch  (proxy)   │               │
│    queue_enqueue/list/      │               │
│       flush/drop            │               │
│    barcode_decode (helper)  │               │
│                             │               │
│   ┌──────────────┐  ┌────────────────┐      │
│   │ HTTP client  │  │ local SQLite    │      │
│   │ to rubix-    │  │ offline queue   │      │
│   │ agent REST   │  │ (pending_       │      │
│   │  + CSRF      │  │  provisions)    │      │
│   └──────┬───────┘  └────────────────┘      │
└──────────┼──────────────────────────────────┘
           │                                  │
           └──────────────┬───────────────────┘
                          ▼
        ┌─────────────────────────────────────────┐
        │  rubix-agent REST  (dev: 127.0.0.1:8088) │
        │   POST /api/v1/auth/login → csrf_token    │
        │   GET  /api/v1/auth/me                    │
        │   POST /api/v1/auth/logout                │
        │   POST /api/v1/tools/{tool_id}  ← bc_*    │
        └─────────────────────────────────────────┘
                          │
                          ▼
        com.nubeio.rubixos extension (bc_* tools, bc_* tables)
        — UNCHANGED, owns all the data (§10).
```

**The offline path:** on a native target, `tool_dispatch` for a
`bc_provision` first tries the agent. If unreachable, the Rust core
writes the request into `pending_provisions` and returns a
`queued` result. The UI shows "queued — will sync". When connectivity
returns, `queue_flush` replays them in order (§7).

**The abstraction boundary (web vs native):** the UI never calls
`invoke` or `fetch` directly. It calls a **`Transport`** interface (§6,
§9). Two impls satisfy it:

- **`tauri-invoke` impl** — calls the Rust `#[command]`s; gets the
  SQLite offline queue and keychain session for free.
- **`fetch` impl** — used on the web/SPA target where there is **no Rust
  core in the browser**; talks to `rubix-agent` directly with
  `credentials: 'include'` + the `X-CSRF-Token` header. The offline
  queue is unavailable on web (it degrades to online-only; see §12 open
  question on cross-origin auth).

At startup the app picks the impl by detecting the Tauri runtime
(`window.__TAURI_INTERNALS__`), so screens are identical on every
target.

---

## 4. User journeys (screens → tools)

Screen parity with BARCODE.md §6 is a hard requirement. Each screen maps
to the `bc_*` tools it calls (full request shapes in §5). Look-and-feel
follows the sexo design system (§ below): dark obsidian canvas,
glassmorphic cards, accent glows, Framer-Motion springs, a registry-driven
nav, bottom sheets for pick/create.

| # | Screen | What it does | Tools called |
|---|---|---|---|
| 0 | **Connect / Login** | Enter agent URL (default `http://127.0.0.1:8088`) + email/password. Stores session; shows who you are. Dev creds `op@example.com` / `rubix-dev-passwd`. | `auth_login`, `auth_me` |
| 1 | **Scan** | Camera (QR + Code128) **or** scanner-wedge keyboard input **or** manual *"pick a device type"* (template dropdown → synthesises the canonical `rubix://add?…` string, no barcode needed — BARCODE.md §6.2). | `bc_decode` (+ `bc_templates_list` to populate the type picker) |
| 2 | **Identify** | Decoded **device card**: "Droplet · LoRa · DRP-9F2C18", template icon + point-list preview. Confirm or rescan. | (uses `bc_decode` result) |
| 3 | **Place** | Pick **Site** (or *＋ create inline*) → pick **Location** (or *＋ new*) → pick/create **Page**. The page picker is **scoped to the chosen site** and **only appears after a site is selected** (`bc_pages.site_id`, BARCODE.md §4.4). | `bc_sites_list`, `bc_site_create`, `bc_locations_list`, `bc_location_create`, `bc_pages_list?site_id=…`, `bc_page_create` |
| 4 | **Toggles** | Two switches — *Trending* / *Alarming* — pre-filled from template defaults. | (carried into provision) |
| 5 | **Confirm** | One `bc_provision` call. Success reveal ("Device connected") shows the live tile. **Offline → queued** (§7). | `bc_provision` |
| 6 | **Devices** | List/search/filter provisioned devices; inline rename, re-place, print label, decommission. Drill-in → points + per-point toggles + alarms. | `bc_devices_list`, `bc_points_by_device`, `bc_alarms_by_device`, `bc_device_update`, `bc_device_decommission`, `bc_label_render` |
| 7 | **Sites** | Site + location tree with inline create. | `bc_sites_list`, `bc_site_create`, `bc_locations_list`, `bc_location_create` |
| 8 | **Templates** | List YAML templates, view raw YAML, validate + upsert (add a device type at runtime). | `bc_templates_list`, `bc_template_yaml`, `bc_template_upsert` |
| 9 | **Page preview (client view)** | Pick **Site → one of its pages** → render the page's widget tiles, exactly what an end viewer sees (*Site → dashboards → sensors*). | `bc_pages_list?site_id=…`, `bc_widgets_by_page` |
| 10 | **Activity** (optional) | Recent provision/decommission events feed (top-sheet, sexo `ActivityCenter` pattern). | `bc_provision_log_recent` |

Steps 1→5 are the **provision wizard** (the §6.1 PWA flow). Screens 6–10
are the standing surfaces (the §6.2 admin tabs). The **Add device**
button on Confirm stays disabled until **a site AND a page** are chosen,
with an inline hint — a device with no page is invisible to viewers
(BARCODE.md §6.2).

---

## 5. The `bc_*` tool surface this client consumes

All dispatched via `POST /api/v1/tools/{tool_id}` (the `tool_id` is the
fully-qualified `com.nubeio.rubixos.<tool>`). Reads are list/by-id tools;
writes go through the named write tools. Confirmed against BARCODE.md §5
and the extension's `kinds/bc_*` schemas.

| tool id | screen(s) | request shape (summary) |
|---|---|---|
| `bc_decode` | Scan/Identify | `{ barcode }` → `ScannedIdentity` + resolved template (pure, no DB) |
| `bc_provision` | Confirm | `{ barcode, site_id, location_id\|new_location, page_id\|new_page, name?, trend, alarm }` → `{ device_id, points, widgets, page_id, warnings[] }` (BARCODE.md §5.1) |
| `bc_device_update` | Devices | `{ device_id, name?, site_id?, location_id?, page_id?, trend?, alarm? }` |
| `bc_device_decommission` | Devices | `{ device_id, hard? }` (soft status flip by default) |
| `bc_site_create` | Place / Sites | `{ name }` → `{ site_id }` |
| `bc_sites_list` | Place / Sites / Preview | `{}` → `[{ site_id, name }]` |
| `bc_location_create` | Place / Sites | `{ site_id, name }` → `{ location_id }` |
| `bc_locations_list` | Place / Sites | `{ site_id }` → `[{ location_id, name }]` |
| `bc_page_create` | Place | `{ site_id, name }` → `{ page_id }` |
| `bc_pages_list` | Place / Preview | `{ site_id? }` → `[{ page_id, site_id, name }]` (empty `site_id` = all pages; BARCODE.md §4.4) |
| `bc_template_upsert` | Templates | `{ template, yaml }` → validate + store |
| `bc_templates_list` | Scan (type picker) / Templates | `{}` → `[{ template, display_name, network, category }]` |
| `bc_template_yaml` | Templates | `{ template }` → `{ yaml }` |
| `bc_devices_list` | Devices | `{ site_id? }` → `[{ device_id, template, name, status, site_id, location_id, page_id }]` |
| `bc_points_by_device` | Device detail | `{ device_id }` → `[{ point_id, point_key, name, unit, kind, widget, trend_on, alarm_on }]` |
| `bc_widgets_by_page` | Page preview | `{ page_id }` → `[{ widget_id, point_id, widget, slot }]` |
| `bc_alarms_by_device` | Device detail | `{ device_id }` → `[{ alarm_id, point_key, severity, message, … }]` |
| `bc_label_render` | Devices | `{ device_id }` → printable label payload (QR + Code128 + serial) |
| `bc_provision_log_recent` | Activity | `{ limit? }` → recent provision/decommission events |

> The page picker query in Place and Page-preview is precisely
> `bc_pages_list` **with** `site_id` set — the "client opens a site, sees
> its dashboards" query from BARCODE.md §4.4.

---

## 6. Tauri Rust core responsibilities (the backend contract)

The Rust core is **thin**: a typed HTTP client to `rubix-agent`, a
SQLite offline queue, and a session/CSRF holder. No business logic that
duplicates the extension — `bc_provision` orchestration stays server-side.

Commands are **one verb per file** under `src-tauri/src/commands/`
(FILE-LAYOUT.md §2). The contract:

| `#[command]` | file | does |
|---|---|---|
| `auth_login` | `commands/auth_login.rs` | POST `/auth/login`; store session cookie + `csrf_token` in the keychain-backed store; return identity-less ok |
| `auth_me` | `commands/auth_me.rs` | GET `/auth/me` → `{ subject, email, role }` |
| `auth_logout` | `commands/auth_logout.rs` | POST `/auth/logout`; clear stored session + CSRF |
| `tool_dispatch` | `commands/tool_dispatch.rs` | **generic proxy** — `{ tool_id, body }` → POST `/api/v1/tools/{tool_id}` with cookie + `X-CSRF-Token`; on a `bc_provision` body while offline, enqueue instead and return `{ queued: true, queue_id }` |
| `barcode_decode` | `commands/barcode_decode.rs` | local helper that normalises a scanned QR/Code128 string into the canonical `rubix://add?…` form before calling `bc_decode` (keeps the wedge/manual/camera inputs uniform; mirrors the PWA's client-side normalisation) |
| `queue_enqueue` | `commands/queue_enqueue.rs` | insert a `pending_provisions` row (used directly by the wizard when the user explicitly defers) |
| `queue_list` | `commands/queue_list.rs` | read pending/errored items for the "pending sync" UI |
| `queue_flush` | `commands/queue_flush.rs` | replay pending items in `created_at` order; on success drop, on failure mark `error` (§7) |
| `queue_drop` | `commands/queue_drop.rs` | remove one queued item by id (user discards it) |

Supporting (non-command) modules, also verb/concept-named — never
`utils`/`helpers` (FILE-LAYOUT.md §5):

```
src-tauri/src/
  agent/            ← the rubix-agent HTTP client
    login.rs        ← POST /auth/login wire call
    dispatch.rs     ← POST /api/v1/tools/{id} wire call
    me.rs · logout.rs
    reachable.rs    ← connectivity probe used by tool_dispatch / queue_flush
  session/
    store.rs        ← keychain-backed session-cookie + csrf_token storage
    csrf.rs         ← attaches X-CSRF-Token to mutating requests
  queue/
    schema.rs       ← creates the SQLite table on first run
    enqueue.rs · list.rs · flush.rs · drop.rs   ← queue verbs (called by commands)
  barcode/
    normalize.rs    ← QR/Code128 → canonical rubix://add string
  error.rs          ← the app's error domain
```

### Offline-queue SQLite schema

A single table; SQLite file lives in the OS app-data dir
(`tauri::path::app_data_dir`).

```
pending_provisions(
  id           INTEGER PRIMARY KEY AUTOINCREMENT,
  payload_json TEXT    NOT NULL,   -- the exact bc_provision request body
  created_at   TEXT    NOT NULL,   -- ISO-8601; flush order
  status       TEXT    NOT NULL,   -- 'pending' | 'syncing' | 'error' | 'done'
  last_error   TEXT                -- last failure message when status='error'
)
```

`payload_json` is the verbatim `bc_provision` body so flush is a literal
replay — no re-derivation, and the device_id inside it makes replay
idempotent (§7).

### Secrets / session / CSRF

- The **session cookie** and **`csrf_token`** are held in the
  `tauri-plugin-store` backed by the OS keychain
  (`tauri-plugin-keychain` / Stronghold on mobile). They never touch
  `localStorage`.
- The HTTP client is a cookie-jar client; every **mutating** dispatch
  (any `bc_*` write) sends `X-CSRF-Token: <stored csrf_token>`. Reads
  (`*_list`, `*_by_*`) send only the cookie.
- `auth_login` captures `csrf_token` from the `LoginResponse` body and
  the session from the `Set-Cookie`; `auth_logout` clears both.

---

## 7. Offline-queue lifecycle

```
   user confirms a provision
            │
   agent reachable? ──yes──► tool_dispatch → bc_provision → success tile
            │
            no
            ▼
   queue_enqueue (status='pending', payload_json = the bc_provision body)
            │
   (connectivity regained — detected by agent/reachable.rs, or user taps "Sync now")
            ▼
   queue_flush: for each pending in created_at order
       status='syncing' → POST bc_provision
         success → status='done' → drop row
         failure → status='error', last_error=<msg>, keep row (retry later)
```

- **Order:** strict FIFO by `created_at`, so a site/location/page
  created earlier in a batch exists before later devices reference it.
- **Idempotency:** replay is safe because `bc_provision` is **idempotent
  per `device_id`** — re-running repairs rather than duplicating
  (BARCODE.md §5.1 / §7 "re-scan = idempotent repair"). A flush that
  partially ran before a crash simply re-runs; already-provisioned
  devices no-op.
- **Errors don't block the queue head forever:** an `error` item is
  skipped on the next pass (kept for the user to inspect/retry/drop via
  `queue_list` / `queue_drop`), so one bad payload can't wedge sync.
- **Web target:** no Rust core → no queue. The wizard's Confirm is
  online-only there and surfaces a plain "agent unreachable" error
  rather than queuing (§12).

---

## 8. Cross-platform targets & how each is built

One `package.json` + one `src-tauri/`. The frontend is identical; the
transport impl is chosen at runtime (§3).

| Target | Build / run | Transport impl | Offline queue |
|---|---|---|---|
| **Desktop** macOS/Win/Linux | `npm run tauri dev` / `npm run tauri build` | `tauri-invoke` | ✅ SQLite |
| **iOS** | `npm run tauri ios init` then `tauri ios dev` / `tauri ios build` | `tauri-invoke` | ✅ SQLite |
| **Android** | `npm run tauri android init` then `tauri android dev` / `tauri android build` | `tauri-invoke` | ✅ SQLite |
| **Web / SPA** | `npm run build` (`tsc -b && vite build`) → static host | `fetch` (direct to agent) | ❌ online-only |

- The web build **cannot** use the Rust core (no Tauri runtime in a
  plain browser), so it talks to `rubix-agent` directly via `fetch` with
  `credentials: 'include'`. This is the explicit
  **`Transport`** abstraction named in §3/§9: a `tauri-invoke`
  implementation and a `fetch` implementation behind one interface.
- Mobile camera permission strings (`NSCameraUsageDescription`,
  Android `CAMERA`) are declared in the Tauri mobile config; the camera
  itself is a web `getUserMedia` viewport inside the React UI
  (§12 picks the decode lib).

---

## 9. File layout (whole app, folder-of-verbs)

Obeys FILE-LAYOUT.md: one verb/concept per file, ≤400 lines (≤150
typical), barrels (`index.ts` / `mod.rs`) re-export only, no
`utils`/`helpers`/`common`.

```
rubix/apps/provision-app/
├── SCOPE.md                      ← this document
├── package.json · vite.config.ts · tsconfig.json · index.html
├── src/                          ← React UI (the same on every target)
│   ├── main.tsx · App.tsx        ← shell: providers + registry-driven nav (sexo pattern)
│   ├── index.css                 ← Tailwind v4 @theme tokens + glass utilities (re-skinned from sexo)
│   ├── transport/                ← THE seam (§3)
│   │   ├── transport.ts          ← the Transport interface + runtime pick
│   │   ├── tauri-invoke.ts       ← impl over Tauri #[command]s
│   │   └── fetch.ts              ← impl over rubix-agent REST (web target)
│   ├── tools/                    ← one file per bc_* call (thin, typed)
│   │   ├── decode.ts · provision.ts · device-update.ts · device-decommission.ts
│   │   ├── site-create.ts · sites-list.ts · location-create.ts · locations-list.ts
│   │   ├── page-create.ts · pages-list.ts
│   │   ├── template-upsert.ts · templates-list.ts · template-yaml.ts
│   │   ├── devices-list.ts · points-by-device.ts · widgets-by-page.ts
│   │   ├── alarms-by-device.ts · label-render.ts · provision-log-recent.ts
│   │   └── bc-types.ts           ← shared wire types (the allowed shared file)
│   ├── auth/                     ← login.tsx · session.ts (calls auth_* via transport)
│   ├── scan/                     ← scan.tsx · normalize.ts · pick-type.tsx (manual type path)
│   ├── identify/                 ← identify.tsx (decoded device card)
│   ├── place/                    ← place.tsx · site-picker.tsx · location-picker.tsx · page-picker.tsx
│   ├── toggles/                  ← toggles.tsx
│   ├── confirm/                  ← confirm.tsx · success-reveal.tsx · queued-notice.tsx
│   ├── devices/                  ← devices.tsx · device-detail.tsx · label-sheet.tsx
│   ├── sites/                    ← sites.tsx (site+location tree, inline create)
│   ├── templates/                ← templates.tsx · template-editor.tsx
│   ├── preview/                  ← page-preview.tsx + widgets/ (gauge·stat·battery·counter·led·toggle·line)
│   ├── activity/                 ← activity-center.tsx (provision log feed)
│   ├── queue/                    ← pending-sync.tsx (queue_list/flush/drop UI)
│   ├── pages/                    ← registry.tsx (nav source of truth, sexo pattern)
│   ├── theme/                    ← themes.ts · ThemeProvider.tsx · useLook.ts
│   └── components/               ← PhoneFrame · NavBar · Toast · ui.tsx (sexo primitives)
└── src-tauri/                    ← Rust core (folder-of-verbs, §6)
    ├── Cargo.toml · tauri.conf.json · build.rs
    └── src/
        ├── main.rs · lib.rs      ← builder: register commands + plugins (barrel)
        ├── commands/             ← auth_login · auth_me · auth_logout · tool_dispatch
        │                            barcode_decode · queue_enqueue · queue_list
        │                            queue_flush · queue_drop  (one #[command] per file)
        ├── agent/                ← login · dispatch · me · logout · reachable
        ├── session/              ← store · csrf
        ├── queue/                ← schema · enqueue · list · flush · drop
        ├── barcode/              ← normalize
        └── error.rs
```

---

## 10. Removability / relationship to the existing feature

This app is **purely additive**:

- It adds **no** server-side tables and **no** tools — it consumes the
  `bc_*` surface the extension already exposes (§5).
- It path-deps into **nothing** in the host or the extension; the only
  coupling is HTTP to `rubix-agent`'s public REST surface.
- Deleting `rubix/apps/provision-app/` leaves the extension, its `bc_*`
  tables, and the agent **byte-for-byte unchanged**. The in-dashboard
  `./Provision` admin module keeps working exactly as before.

The relationship to the extension's own clients (BARCODE.md §6): this
app **absorbs the phone-PWA flow** (§1) and offers the same admin
surfaces as standalone screens; the federated admin module remains the
in-dashboard option. The two are independent and either can ship or be
removed without the other.

---

## 11. Phasing (T0–T6)

Mirrors BARCODE.md §9's per-phase "done when" rigor. T0–T3 is the spine
(scaffold → auth → scan → provision); T4–T6 is breadth + offline +
mobile.

| phase | deliverable | done when |
|---|---|---|
| **T0** | Tauri v2 workspace scaffolded; React+TS+Vite+Tailwind v4 + sexo shell (PhoneFrame, NavBar, registry, glass tokens) renders an empty Dashboard | `tauri dev` opens a window with the themed shell; `vite build` produces an SPA |
| **T1** | `Transport` seam + both impls; `auth_login`/`auth_me`/`auth_logout` commands; CSRF + keychain session; **Connect/Login** screen | log in to a dev agent (`op@example.com`), `auth_me` shows identity on desktop **and** in the web build |
| **T2** | **Scan → Identify**: camera + wedge + manual "pick a type"; `barcode_decode` normalise + `bc_decode`; decoded device card | scanning a `rubix://add?…` (or picking a template) renders the Droplet identify card with its point preview |
| **T3** | **Place → Toggles → Confirm**: site/location/page pickers (page scoped to site, inline create), toggles, one online `bc_provision`; success reveal | a device provisions end-to-end online; appears on its page in **Page preview** |
| **T4** | Standing lists: **Devices** (rename/re-place/decommission/label), **Sites**, **Templates**, **Page preview**, **Activity** | every screen in §4 calls its §5 tools; freshness — a just-added device/page shows without reload |
| **T5** | **Offline queue**: SQLite `pending_provisions`, `queue_*` commands, reachability detection, flush-on-reconnect, **Pending sync** UI | airplane-mode a desktop build, provision → queued; reconnect → auto-flush; re-flush is idempotent (no dupes) |
| **T6** | **Mobile targets**: `tauri ios` / `tauri android` init + camera permissions; smoke a provision from a phone | a real iOS **and** Android build provisions a device end-to-end, offline-queue included |

---

## 12. Open questions

1. **Camera/decode lib per platform.** `BarcodeDetector` (native, best
   on Android/Chromium, absent on iOS Safari/WKWebView) vs `@zxing/browser`
   vs `html5-qrcode` as the portable fallback. Likely: feature-detect
   `BarcodeDetector`, fall back to `@zxing/browser`. Decide before T2.
2. **BLE / NFC commissioning (later).** Out of scope for v1 (catalog
   provisioning only, like BARCODE.md). The Rust seam (§6) is where a
   `tauri-plugin-blec` / NFC reader would land when real on-air
   commissioning is needed — note it, don't build it.
3. **Web build cross-origin auth.** The SPA's `fetch` impl needs the
   agent to send CORS + `Set-Cookie` that the browser will keep for a
   different origin (SameSite/secure, credentialed CORS). Either host the
   SPA same-origin behind the agent, or add a CORS allowlist + a token
   fallback (`POST /api/v1/auth/token` exists) for the browser target.
   Decide before shipping web.
4. **i18n + unit prefs.** The host has EN/ES + per-user unit conversion
   (convert-on-read). This app **defers** both for v1 — it renders
   canonical units and EN strings, and reads identity/role from
   `auth_me`. Wiring per-user unit prefs and ES strings is a follow-up
   once the provision flow is solid.
5. **PWA retirement.** Once T6 mobile targets ship, decide whether to
   delete the extension's `ui-src/pwa/` flow (this app supersedes it,
   §1) or keep it as a zero-install fallback.
6. **Provision atomicity (inherited).** `bc_provision` is best-effort +
   idempotent server-side (BARCODE.md §5.1/§10). This client relies on
   that idempotency for queue replay (§7); if the host later gains a
   `warehouse_tx` capability, the queue logic is unaffected.

---

*Companion docs:*
[`../../extensions/com.nubeio.rubixos/BARCODE.md`](../../extensions/com.nubeio.rubixos/BARCODE.md)
(the backend feature this client drives),
[`../../FILE-LAYOUT.md`](../../FILE-LAYOUT.md) (the layout discipline this
app obeys), [`../../openapi.json`](../../openapi.json) (the agent REST
surface), and the sexo `DESIGN_SYSTEM.md` (the UI language this app
re-skins from).
