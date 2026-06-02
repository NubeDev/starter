# BARCODE.md — Scan-to-Dashboard provisioning system

**Status:** **built — B0–B7 landed** (catalog provisioning end-to-end).
Live ingest of real readings remains the sibling concern noted in
[§10](#10-open-questions). See [§9 Phasing](#9-phasing) for the per-phase
state and [§6 Frontend](#6-frontend--app) for what the admin UI actually
does today.
**Owner:** `com.nubeio.rubixos` extension (this directory).
**One-line:** *Add a sensor (scan a sticker **or** pick its type) → it
appears on a site's dashboard page, trending and alarming, placed at a
location — without anyone hand-editing config.*

This document is the source of truth for the feature. Keep it updated
when the shape changes. It is deliberately **self-contained and
removable** (see [§11 Removability](#11-removability)).

> **What changed since the original scope:** (1) a page now **belongs to
> a site** (`bc_pages.site_id`) so the client browses *Site → its
> dashboards* — see [§4.4](#44-bc_pages--a-page-belongs-to-a-site); (2)
> the admin wizard added a **"pick a device type"** path and **inline
> site creation** so a human with no barcode and an empty system can
> still provision — see [§6.2](#62-admin-panel-federated-ui-in-this-extension);
> (3) all list views re-read fresh after a write so newly-added
> devices/pages appear without a reload.

---

## 0. Why this exists

Today the extension is **read-only**: [`scripts/load-dump.sh`](scripts/load-dump.sh)
bulk-loads a Rubix-OS Postgres dump into `com_nubeio_rubixos__*`
warehouse tables and the dashboard reads them back through
`warehouse_query` templates. There is no way to *add* a device, and
provisioning a new sensor means a laptop, the Rubix-OS UI, and manual
point mapping.

The field reality we are designing for:

> A technician unboxes a **Droplet** wall sensor, peels the sticker,
> scans the barcode with a phone, taps *"Add to Level 3 — North"*, and
> walks away. The sensor is now trending temperature + humidity, the
> low-battery alarm is armed, and it shows on the floor dashboard.

That is the whole goal. The barcode carries enough identity to look up
a **device template** (a YAML file), the template knows what points the
device has and what widgets to render, and the provisioning system does
the CRUD to make it real.

This is **separate code** from the existing warehouse/dashboard surface.
It can be deleted wholesale and the energy/water dashboards keep working.

---

## 1. The mental model — four nouns + one verb

The Rubix-OS hierarchy the technician thinks in:

```
Network        LoRa | BACnet | Rubix(REST)      ← the transport
  └─ Device    a Droplet, a Micro Edge, an IO-22 ← the physical thing
       └─ Point   Temp, Humidity, Battery, DI-1   ← one readable value
```

Placement is orthogonal to the network hierarchy. A **site** has both a
physical breakdown (locations) and one or more display surfaces (pages):

```
Site           "Building A"          ← a customer site (may pre-exist; created on demand)
  ├─ Location  "Level 3 — North"     ← a room/zone/floor; where the device physically sits
  │    └─ (devices are placed here)
  └─ Page      "Floor 3 dashboard"   ← a screen the client opens; widgets render here
       └─ (widgets for placed devices render here)
```

So a device carries **both** a `location_id` (physical) and lands its
widgets on a **page that belongs to the same site** (display). The client
journey is *Site → its pages → the sensor tiles*. See
[§4.4](#44-bc_pages--a-page-belongs-to-a-site).

The verb is **provision**:

```
barcode ──decode──► identity ──lookup──► template(YAML)
                                            │
                              ┌─────────────┴──────────────┐
                              ▼                             ▼
                    CRUD: device + points        choose/create site+location
                              │                             │
                              └──────────────┬──────────────┘
                                             ▼
                              generate widgets + (optional) page
                                             ▼
                              enable trending + alarming (toggles)
```

Everything downstream of "identity" is deterministic and driven by the
YAML template. The only human decisions are: **which site/location**,
and **which toggles** (trend / alarm).

---

## 2. What's on the barcode

Sensors and gateways ship with a printed sticker. The barcode is the
**minimum identity** needed to (a) know what the device is and (b) reach
it on the network. We do **not** put secrets or full config on the
sticker — the template supplies the rest.

Payload (encoded as a compact URL or a JSON blob in a QR / Code128):

| field         | example                    | purpose                                            |
|---------------|----------------------------|----------------------------------------------------|
| `id`          | `DRP-9F2C18`               | globally unique device id (also the human serial)  |
| `model`       | `droplet`                  | **template key** → `templates/droplet.yaml`        |
| `network`     | `lora`                     | `lora` \| `bacnet` \| `rubix`                       |
| `default_ip`  | `192.168.15.42`            | for `rubix`/`bacnet`; omitted for `lora`           |
| `eui` / `addr`| `70B3D5...`                | LoRa DevEUI or BACnet MAC; the on-air address      |
| `hw`          | `1.2`                      | hardware rev (template may branch on it)           |
| `v`           | `1`                        | barcode schema version                             |

Canonical QR form (human-clickable, survives copy-paste, versionable):

```
rubix://add?v=1&id=DRP-9F2C18&model=droplet&network=lora&eui=70B3D5...
```

Code128 fallback (cheap thermal-printer label, scanner-keyboard input):

```
1|DRP-9F2C18|droplet|lora|70B3D5...
```

A **decoder** normalises both into the same `ScannedIdentity` struct
(see [§5 Tools](#5-backend-tools)). Unknown `model` → friendly error
listing the known template keys, never a 500.

> **Label generation is in-scope too.** When a device is provisioned
> without a sticker (manual add), the system can render a printable
> label (QR + Code128 + human serial) so re-scanning works later. This
> closes the loop for devices that arrive without a Nube-iO sticker.

---

## 3. The YAML device template (the heart of it)

One file per model, stored **in Postgres** (see [§4](#4-data-model))
and seeded from `templates/*.yaml` in this directory. The template
declares the device's points, default widgets, and which features can
be toggled. The provisioning engine reads it and does the rest.

### 3.1 `templates/droplet.yaml`

```yaml
template: droplet
version: 1
display_name: Droplet Wall Sensor
network: lora            # default; barcode can override
category: sensor
icon: droplet

# Points this device exposes. Each becomes a row in
# com_nubeio_rubixos__bc_points at provision time.
points:
  - key: temp
    name: Temperature
    unit: "°C"            # canonical storage unit (convert-on-read per user prefs)
    kind: analog
    widget: gauge
    trend: { default: true,  interval: "5m" }
    alarm:
      default: false
      rules:
        - { when: "> 35", severity: warning, message: "High temperature" }
        - { when: "< 5",  severity: warning, message: "Low temperature" }

  - key: humidity
    name: Humidity
    unit: "%RH"
    kind: analog
    widget: gauge
    trend: { default: true, interval: "5m" }

  - key: battery
    name: Battery
    unit: "%"
    kind: analog
    widget: battery
    trend: { default: false }
    alarm:
      default: true       # low-battery alarm armed by default
      rules:
        - { when: "< 20", severity: warning,  message: "Low battery" }
        - { when: "< 5",  severity: critical, message: "Battery critical" }

  - key: rssi
    name: Signal
    unit: dBm
    kind: analog
    widget: stat
    trend: { default: false }

# How the device renders as a unit on a page.
widget_group:
  layout: card           # card | row | bento-tile
  title: "{{device.name}}"
  primary: temp          # the value shown big
  secondary: [humidity, battery]
```

### 3.2 `templates/micro_edge.yaml` (sketch)

```yaml
template: micro_edge
display_name: Micro Edge
network: lora
points:
  - { key: ui1,     name: "Universal Input 1", kind: analog,  widget: stat,    trend: { default: true } }
  - { key: pulse1,  name: "Pulse 1",           kind: counter, widget: counter, trend: { default: true } }
  - { key: battery, name: Battery, unit: "%",  kind: analog,  widget: battery, alarm: { default: true, rules: [{ when: "< 20", severity: warning, message: "Low battery" }] } }
```

### 3.3 `templates/io_22.yaml` (sketch)

```yaml
template: io_22
display_name: IO-22 — 22-channel IO Controller
network: rubix          # REST
points:
  # 12 digital inputs
  - { key: di1,  name: "DI 1",  kind: digital, widget: led,    repeat: 12 }   # repeat expands di1..di12
  # 10 relay / digital outputs (writable)
  - { key: do1,  name: "DO 1",  kind: digital, widget: toggle, writable: true, repeat: 10 }
widget_group:
  layout: row
```

**Template rules**

- `template` is the lookup key matched against the barcode `model`.
- Unknown YAML keys are a **load-time error** (strict parse) — a typo
  must never silently drop a point or an alarm.
- `repeat: N` expands one point spec into `key1..keyN` so the IO-22's
  22 channels aren't 22 copy-pasted blocks.
- `widget` is an enum the UI knows how to render: `gauge | stat |
  battery | counter | led | toggle | line`.
- A template is **versioned**; re-provisioning an already-known device
  diffs against the stored template version (see [§7](#7-lifecycle)).

---

## 4. Data model (Postgres, host-owned, prefixed)

All new tables follow the host convention: declared **unprefixed** in
`block.yaml` under `contributes.warehouse_tables[]`, created by
`boot::extension_tables` as `com_nubeio_rubixos__<name>` with
`tenant_id TEXT` prepended at column 0. The extension never writes
`tenant_id` — the host stamps it from the caller session.

> These are **new `bc_` tables**, disjoint from the existing
> `histories` / `points` / `*_tags` warehouse tables. The `bc_` prefix
> (barcode/provisioning) is what makes the feature droppable: removing
> the feature = removing the `bc_*` table declarations + their tools.

| table (unprefixed)     | what it holds                                            | key column     |
|------------------------|----------------------------------------------------------|----------------|
| `bc_templates`         | the YAML files (raw text + parsed metadata)              | `template`     |
| `bc_sites`             | customer sites ("Building A")                            | `site_id`      |
| `bc_locations`         | zones/floors/rooms under a site                          | `location_id`  |
| `bc_devices`           | provisioned devices (one row per add)                    | `device_id`    |
| `bc_points`            | points expanded from the template per device             | `point_id`     |
| `bc_widgets`           | widget instances placed on a page (generated)            | `widget_id`    |
| `bc_pages`             | dashboard pages **scoped to a site** (`site_id`)         | `page_id`      |
| `bc_alarms`            | armed alarm rules (materialised from template + toggles) | `alarm_id`     |
| `bc_provision_log`     | audit: every scan/provision/decommission event           | `event_id`     |

### 4.1 `bc_templates`

```yaml
- name: bc_templates
  order_by: [template]
  columns:
    - { name: template,     type: "TEXT" }          # PK / lookup key (== barcode model)
    - { name: version,      type: "INTEGER" }
    - { name: display_name, type: "TEXT", default: "NULL" }
    - { name: network,      type: "TEXT", default: "NULL" }
    - { name: category,     type: "TEXT", default: "NULL" }
    - { name: yaml,         type: "TEXT" }           # raw YAML, source of truth
    - { name: points_json,  type: "JSONB", default: "NULL" }  # parsed cache for fast reads
    - { name: updated_at,   type: "TIMESTAMPTZ", default: "now()" }
```

YAML lives in PG so an operator can edit/add templates **without a
rebuild** — a `templates_upsert` tool validates + stores; the `templates/*.yaml`
files in this repo are just the **seed** loaded at first boot.

### 4.2 `bc_devices`

```yaml
- name: bc_devices
  order_by: [site_id, device_id]
  columns:
    - { name: device_id,   type: "TEXT" }            # from barcode `id`
    - { name: template,    type: "TEXT" }            # FK → bc_templates.template
    - { name: name,        type: "TEXT", default: "NULL" }   # human label, editable
    - { name: network,     type: "TEXT", default: "NULL" }   # lora|bacnet|rubix
    - { name: address,     type: "TEXT", default: "NULL" }   # eui / mac / ip
    - { name: default_ip,  type: "TEXT", default: "NULL" }
    - { name: hw_rev,      type: "TEXT", default: "NULL" }
    - { name: site_id,     type: "TEXT", default: "NULL" }
    - { name: location_id, type: "TEXT", default: "NULL" }
    - { name: page_id,     type: "TEXT", default: "NULL" }
    - { name: status,      type: "TEXT", default: "'provisioned'" } # provisioned|online|offline|decommissioned
    - { name: provisioned_at, type: "TIMESTAMPTZ", default: "now()" }
```

### 4.3 `bc_points`

```yaml
- name: bc_points
  order_by: [device_id, point_id]
  columns:
    - { name: point_id,    type: "TEXT" }            # device_id + ':' + key
    - { name: device_id,   type: "TEXT" }
    - { name: point_key,   type: "TEXT" }            # 'temp', 'humidity', ...
    - { name: name,        type: "TEXT", default: "NULL" }
    - { name: unit,        type: "TEXT", default: "NULL" }
    - { name: kind,        type: "TEXT", default: "NULL" }   # analog|digital|counter
    - { name: widget,      type: "TEXT", default: "NULL" }
    - { name: writable,    type: "BOOLEAN", default: "false" }
    - { name: trend_on,    type: "BOOLEAN", default: "false" }
    - { name: alarm_on,    type: "BOOLEAN", default: "false" }
    # The live value lands in the existing `histories` hypertable,
    # keyed by point_uuid == point_id, so the existing trend/chart
    # templates work unchanged.
```

### 4.4 `bc_pages` — a page belongs to a site

```yaml
- name: bc_pages
  order_by: [site_id, page_id]
  columns:
    - { name: page_id,    type: "TEXT" }
    - { name: site_id,    type: "TEXT", default: "NULL" }  # FK → bc_sites.site_id
    - { name: name,       type: "TEXT", default: "NULL" }
    - { name: created_at, type: "TIMESTAMPTZ", default: "now()" }
```

A **page is a site's dashboard**: it belongs to exactly one site, so the
mental model is *Site → Location* (where the device physically is) **and**
*Site → Page → Widget* (what a viewer looks at), both hanging off the same
site:

```
Site  "Building A"
  ├─< Location  "Level 3 — North"   ← PHYSICAL: where the sensor sits
  └─< Page      "Floor 3 dashboard" ← DISPLAY: a screen the client opens
                   └─< Widget ── Device.Point   ← one reading, rendered
```

`site_id` is **nullable** so pages created before this column existed
read as *unassigned* (they still load; they just aren't tied to a site).
`bc_provision` stamps the page's `site_id` from the device's chosen site,
and `bc_pages_list` takes an optional `site_id` filter — empty returns
every page, a site id returns just that site's dashboards. That filter is
the *"client opens a site, sees its dashboards"* query.

`bc_sites`, `bc_locations`, `bc_widgets`, `bc_alarms`,
`bc_provision_log` follow the same shape; the columns above plus
[§4.1–4.3](#41-bc_templates) are load-bearing for the walkthrough in
[§8](#8-end-to-end-walkthrough).

**Why histories is reused, not re-invented:** the live readings table
(`com_nubeio_rubixos__histories`) already exists as a Timescale
hypertable with the right indexes and the `history_recent` /
`history_bucketed_1m` templates. Provisioned points write their samples
there with `point_uuid = bc_points.point_id`, so **trending is free** —
no new time-series plumbing.

---

## 5. Backend tools

All CRUD goes through the host's `WarehouseWriteBackend`, whose trait
is exactly `insert(table, rows)` / `update(table, key_column, rows)` /
`delete(table, key_column, keys)` (see
[`rubix/crates/rubix-agent/src/extensions/warehouse_write.rs`](../../crates/rubix-agent/src/extensions/warehouse_write.rs)).
The host validates columns against the manifest, stamps `tenant_id`,
and resolves the unprefixed table name. We add **one write grant** and
a handful of thin tool handlers — same pattern as
[`com.rubix.example`](../com.rubix.example) `products_create/update/delete`.

`block.yaml` capability addition:

```yaml
capabilities:
  - kind: warehouse_write
    tables: [bc_templates, bc_sites, bc_locations, bc_devices,
             bc_points, bc_widgets, bc_pages, bc_alarms, bc_provision_log]
  - kind: warehouse_read           # extend the existing read grant
    tables:
      - com_nubeio_rubixos__bc_devices
      - com_nubeio_rubixos__bc_points
      - com_nubeio_rubixos__bc_sites
      - com_nubeio_rubixos__bc_locations
      # …(existing histories/points read grant stays as-is)
```

Tool surface (each is a thin handler in `process/src/main.rs`, named by
the macro as `handle_com_nubeio_rubixos_<tool>`):

| tool id (`com.nubeio.rubixos.`…)  | does                                                                 |
|-----------------------------------|----------------------------------------------------------------------|
| `bc_decode`                       | barcode string → `ScannedIdentity` (pure; no DB). Resolves template. |
| `bc_provision`                    | **the big one** — identity + site/location + toggles → CRUD + widgets |
| `bc_device_update`                | rename, re-place, change toggles                                     |
| `bc_device_decommission`          | soft-delete (status) or hard `delete` + cascade points/widgets      |
| `bc_site_create` / `bc_site_list` | sites CRUD                                                           |
| `bc_location_create`              | location under a site (created on demand from the scan flow)         |
| `bc_template_upsert` / `_list`    | add/edit a YAML template at runtime (validated before store)        |
| `bc_label_render`                 | device → printable label payload (QR + Code128 + serial)            |
| `warehouse_query` (existing)      | reads for the provisioning UI via the existing read-template proxy   |

### 5.1 `bc_provision` — the orchestrator

Input:

```json
{
  "barcode":  "rubix://add?v=1&id=DRP-9F2C18&model=droplet&network=lora&eui=70B3D5...",
  "site_id":  "site-bldg-a",
  "location_id": "loc-l3-north",       // or { "new_location": { "name": "Level 3 — North" } }
  "page_id":  "page-floor-3",          // or { "new_page": { "name": "Floor 3" } }
  "name":     "L3 North Droplet",      // optional human label override
  "trend":    true,                    // master toggle; falls back to per-point template defaults
  "alarm":    true
}
```

Steps (all inside one logical provision; ordering matters for FK
sanity):

1. `bc_decode(barcode)` → identity; load `bc_templates[model]`.
2. If `new_location`/`new_page` present → `insert` those first, capture ids.
3. `insert` one `bc_devices` row.
4. Expand template points (`repeat` honoured) → `insert` N `bc_points`
   rows. `trend_on` / `alarm_on` resolved as
   `master_toggle ?? template_default`.
5. Materialise alarm rules where `alarm_on` → `insert` `bc_alarms`.
6. Generate widgets from `template.widget_group` + per-point `widget`
   → `insert` `bc_widgets` bound to `page_id`.
7. `insert` a `bc_provision_log` audit row.
8. Return a summary `{ device_id, points: N, widgets: N, page_id, warnings: [] }`.

> **Atomicity note (open question, see [§10](#10-open-questions)).** The
> SPI's write handle is per-statement; there is no extension-facing
> transaction yet. Provision is therefore *best-effort multi-insert*
> today: if step 4 fails after step 3, we get a half-provisioned device.
> Mitigation for v1: provision is **idempotent + re-runnable** keyed on
> `device_id` (re-scan repairs), and `bc_provision_log` records the last
> good step. A host-level "extension transaction" capability is the
> proper fix and is noted as an upstream-first ([R2](../../SCOPE.md)) ask.

---

## 6. Frontend / app

Two clients, same backend tools and the same shared `Place` step. The
admin panel is the built, primary surface today; the phone PWA reuses the
same flow for one-handed field use.

### 6.1 Phone app (PWA first, native later)

A thin PWA served by the extension UI bundle, designed for one-handed
field use:

1. **Scan** — `getUserMedia` + a JS barcode lib (QR + Code128). On a
   scanner-keyboard (USB/BT ring scanner) the same screen accepts
   wedge input. → calls `bc_decode`.
2. **Identify** — shows decoded device card ("Droplet · LoRa ·
   DRP-9F2C18", template-derived icon + point list preview).
3. **Place** — pick **Site** (or *＋ create one inline*) → pick
   **Location** or *＋ New location* → pick/create a **Dashboard page**.
   The page picker is **scoped to the chosen site** (a page belongs to a
   site), and only appears once a site is selected. Shared component:
   [`ui-src/pwa/place.tsx`](ui-src/pwa/place.tsx).
4. **Toggles** — two switches: *Trending* / *Alarming* (pre-filled from
   template defaults).
5. **Confirm** — one `bc_provision` call. Success screen shows the live
   device tile already rendering.

PWA because: installable, camera access, offline-queue a scan when the
gateway is unreachable and sync later. Native (Flutter — see the
`flutter-*` skills) is a later wrapper if BLE/NFC commissioning is
needed.

### 6.2 Admin panel (federated UI, in this extension) — **built**

The `./Provision` module (`contributes.ui[].entry: ui/remoteEntry.js`)
mounts under `/extensions/com.nubeio.rubixos/provision/*` with five tabs
(see [`ui-src/provision/index.tsx`](ui-src/provision/index.tsx)):

- **Devices** — list/search/filter `bc_devices`, inline rename, print
  label, decommission. Drill-in → points + per-point toggles.
- **Sites & Locations** — site + location tree with inline create.
- **Templates** — list YAML templates, edit-in-place, validate,
  `bc_template_upsert`. Add a device type without touching the repo.
- **Provision wizard** — *"add a device"* in two modes:
  - **Choose a type** (default) — a **dropdown of templates** with an
    auto-generated serial. **No barcode required** — this is the path a
    human uses with a sensor in hand. It synthesises the canonical
    `rubix://add?…` string and runs the same `bc_decode` → place →
    provision flow.
  - **Scan barcode** — paste/scan a `rubix://add?…` URL or serial.
  Then the shared `Place` step (incl. **inline site create**, so an empty
  system isn't a dead end), trend/alarm switches, and **Add device**.
  The button stays disabled until a site **and** a page are chosen, with
  an inline hint explaining why — a device with no page is invisible to
  viewers. See [`ui-src/provision/tabs/wizard-tab.tsx`](ui-src/provision/tabs/wizard-tab.tsx).
- **Page preview** — the **client view**: pick a **Site** → one of its
  **pages** → the page renders its widget tiles. This is the
  *Site → dashboards → sensors* surface the end user sees.

**Freshness:** every mutation re-reads the affected lists (a shared
refresh signal in [`ui-src/provision/refresh.ts`](ui-src/provision/refresh.ts)
plus always-fresh list reads in [`ui-src/api.ts`](ui-src/api.ts)), so a
just-added device or page shows up **without a page reload**. The ~100 ms
read-after-write window on a freshly-committed row is covered by staggered
re-reads.

### 6.3 Premade widgets (generated from YAML)

The UI ships a fixed set of widget renderers keyed by the template's
`widget` enum — `gauge`, `stat`, `battery`, `counter`, `led`, `toggle`,
`line`. `bc_provision` writes `bc_widgets` rows that say *"render widget
`gauge` for point `temp` on page `page-floor-3` at slot N"*; the page
renderer reads them and mounts the matching component. **No SDUI yet** —
the user's note ("SDUI looks like shit right now") is respected: this is
a curated widget catalog, and SDUI can replace the renderer later
without changing the data model (`bc_widgets` is already a
serialisable layout).

---

## 7. Lifecycle

```
   scan ──► provisioned ──► online ⇄ offline ──► decommissioned
              │                │                      │
              │                └─ heartbeat/last-seen updates status
              └─ re-scan = idempotent repair (diff template version)
```

- **Re-scan** an existing `device_id`: no duplicate; re-runs provision
  to repair missing points/widgets and re-applies the current template
  version (logs a diff to `bc_provision_log`).
- **Template upgrade**: bumping `bc_templates.version` does **not**
  auto-touch live devices; an explicit *"upgrade devices on template
  droplet"* admin action re-provisions them (adds new points, never
  silently drops data).
- **Decommission**: soft by default (`status=decommissioned`, history
  retained); hard delete cascades `bc_points` + `bc_widgets` + `bc_alarms`
  via `delete(table, key_column, keys)` and logs it.

---

## 8. End-to-end walkthrough (the smoke test)

The provisioning analogue of the six-step thin-slice in
[`rubix/README.md`](../../README.md). If these steps work, the feature
is real.

```bash
# 0. Seed templates (first boot loads templates/*.yaml into bc_templates)
make install            # registers block.yaml: bc_* tables + tools
make seed-templates     # bc_template_upsert for each templates/*.yaml

# 1. Decode a barcode (pure, no placement)
curl -b cookies.txt -H 'content-type: application/json' \
  -d '{"barcode":"rubix://add?v=1&id=DRP-9F2C18&model=droplet&network=lora&eui=70B3D5"}' \
  http://127.0.0.1:8088/api/v1/tools/com.nubeio.rubixos.bc_decode
# → { id, model:"droplet", network:"lora", template:{ display_name, points:[temp,humidity,battery,rssi] } }

# 2. Create a site + provision the device into a new location + page
curl -b cookies.txt -H 'content-type: application/json' \
  -d '{"barcode":"rubix://add?...","site_id":"site-bldg-a",
       "new_location":{"name":"Level 3 — North"},
       "new_page":{"name":"Floor 3"},"trend":true,"alarm":true}' \
  http://127.0.0.1:8088/api/v1/tools/com.nubeio.rubixos.bc_provision
# → { device_id:"DRP-9F2C18", points:4, widgets:3, page_id:"page-…", warnings:[] }

# 3. Read it back through the existing warehouse_query proxy
curl -b cookies.txt -H 'content-type: application/json' \
  -d '{"template":"com.nubeio.rubixos.bc_devices_list","params":{"site_id":"site-bldg-a"}}' \
  http://127.0.0.1:8088/api/v1/tools/com.nubeio.rubixos.warehouse_query
# → the Droplet, status=provisioned, 4 points, placed at Level 3 — North

# 4. Open the floor page in the frontend → the Droplet card renders
#    a temp gauge + humidity + battery, battery alarm armed.

# 5. Audit the provision
psql -c "SELECT event, device_id, step FROM com_nubeio_rubixos__bc_provision_log ORDER BY at DESC LIMIT 5"
```

---

## 9. Phasing

| phase | status | deliverable                                                                 | done when                                              |
|-------|--------|-----------------------------------------------------------------------------|--------------------------------------------------------|
| **B0**| ✅ | `bc_*` tables in `block.yaml` + write grant; boots, tables exist            | `\dt com_nubeio_rubixos__bc_*` shows 9 tables          |
| **B1**| ✅ | `bc_decode` + `templates/droplet.yaml` parse; `bc_template_upsert/_list`    | step 1 of [§8](#8-end-to-end-walkthrough) returns      |
| **B2**| ✅ | `bc_site/location/page` CRUD + `bc_provision` (device + points + log)       | step 2 provisions; re-add is idempotent                |
| **B3**| ✅ | widget generation + `bc_widgets` + page renderer reads them                 | step 4 — Droplet card renders from YAML                |
| **B4**| ⚠️ | trend/alarm toggles wired; `bc_alarms` armed at provision                   | toggles + `bc_alarms` rows land; **alarm *evaluation* depends on live ingest** (see [§10.2](#10-open-questions)) |
| **B5**| ✅ | admin panel (`./Provision` UI module): devices/sites/templates CRUD + wizard + page preview | rename + re-place + edit template + add device from the browser |
| **B6**| ✅ | phone PWA: scan/pick → place → confirm                                       | a phone add provisions end-to-end                      |
| **B7**| ✅ | micro_edge + io_22 templates; `repeat:` expansion; label render             | IO-22 provisions 22 points; printed label re-scans     |

Legend: ✅ built · ⚠️ partial (catalog side built; depends on a sibling
concern). B0–B4 is the spine ("add to dashboard"); B5–B7 is breadth.

Beyond the original B-list, two refinements landed: **pages are
site-scoped** ([§4.4](#44-bc_pages--a-page-belongs-to-a-site)) and the
wizard gained a **no-barcode "pick a type" path + inline site create**
([§6.2](#62-admin-panel-federated-ui-in-this-extension)).

---

## 10. Open questions

1. **Provision atomicity.** No extension-facing transaction in the SPI
   today (see [§5.1](#51-bc_provision--the-orchestrator)). Ship
   idempotent-best-effort for v1, or block on an upstream-first
   `warehouse_tx` capability? *Leaning: ship best-effort + idempotent
   re-scan; file the upstream ask.*
2. **Live ingest.** This scope provisions *catalog* rows. Getting actual
   readings into `histories` (the LoRa/BACnet driver → `warehouse_write`
   on `histories`) is a **sibling concern** — a streaming-ingest tool.
   In scope to *enable* (point rows + trend flag) but the driver itself
   may be a separate extension. Decide the boundary.
3. **Alarm evaluation engine.** `bc_alarms` stores rules; *who evaluates
   them*? Reuse the host's flow/anomaly-rule runtime (the
   `anomaly_rules[]` contribution `com.rubix.example` uses) vs. a small
   in-extension evaluator. *Leaning: reuse host rules — upstream-first.*
4. **Point ↔ history key.** Confirm `histories.point_uuid` can be the
   `bc_points.point_id` we mint (string), so trend templates need zero
   change. (Believed yes — `point_uuid` is `TEXT`.)
5. **Barcode standard.** Lock the QR/Code128 payload grammar + version
   field before printing physical stickers. `v=1` is reserved here.

---

## 11. Removability

The whole feature is contained and reversible — a hard requirement from
the user ("i want all this code to be separate so we can easily remove
it"):

- **Tables:** every new table is `bc_*`-prefixed. Drop the
  `warehouse_tables[]` `bc_*` entries + the `warehouse_write` grant from
  `block.yaml`, restart, and `boot::extension_tables` stops creating
  them. Data drop is one `DROP TABLE com_nubeio_rubixos__bc_* CASCADE`.
- **Tools:** all provisioning tools are `bc_*`-named; delete their
  handlers from `process/src/main.rs` and their `kinds/bc_*` schemas.
- **UI:** the `./Provision` module and the PWA are separate files under
  `ui-src/provision/`; removing the `contributes.ui[]` entry unmounts
  them.
- **Templates:** `templates/*.yaml` are seed only; deleting them and the
  `bc_templates` table leaves the energy/water dashboard untouched.

Nothing here forks `rubix-agent` or path-deps into host crates — it uses
only the extension SPI surface (`warehouse_read`, `warehouse_write`,
federated `ui`, `tools`), exactly like the existing read-only dashboard
and the `com.rubix.example` / `com.rubix.geo` CRUD references.

---

## 12. Files this feature adds (as built)

```
com.nubeio.rubixos/
├── BARCODE.md                      ← this scope
├── block.yaml                      ← bc_* tables (incl. bc_pages.site_id), write grant, bc_* tools, ./Provision ui
├── templates/                      ← seed YAML device templates
│   ├── droplet.yaml · micro_edge.yaml · io_22.yaml
├── kinds/                          ← bc_* tool schemas + bc_*_list read templates (SQL + params)
│   ├── bc_decode_in.json / bc_provision_in.json / …
│   ├── bc_pages_list.sql + _params.json   (optional $site_id filter)
│   └── bc_devices_list.sql, bc_widgets_by_page.sql, site/location/template SQL
├── process/src/provision/          ← backend (separate process binary)
│   ├── mod.rs                      ← tool handlers (thin proxies over warehouse_write)
│   ├── decode.rs · identity.rs     ← barcode → ScannedIdentity
│   ├── template.rs · template_store.rs ← YAML parse + bc_templates store
│   ├── orchestrate.rs              ← bc_provision: the multi-insert flow
│   ├── placement.rs                ← resolve/create site · location · page (stamps page.site_id)
│   ├── points.rs · widgets.rs · alarms.rs · device.rs · label.rs · crud.rs · ids.rs
└── ui-src/
    ├── api.ts                      ← callTool/fetchTemplate (fresh list reads)
    ├── provision/                  ← admin panel module (./Provision)
    │   ├── index.tsx               ← 5-tab shell
    │   ├── bc-api.ts · bc-types.ts · build-input.ts
    │   ├── refresh.ts              ← shared post-mutation refresh signal
    │   ├── tabs/                   ← devices · sites · templates · wizard · page-preview (+ detail/label/identify)
    │   ├── page-render/ · widgets/ ← widget renderers keyed by the `widget` enum
    └── pwa/                        ← phone flow: scan · identify · place (shared) · confirm
```

> **Runtime note:** this extension is `runtime: kind: process` — the
> backend is a separate binary (`rubix-nubeio-rubixos-extension`), not
> in-host. The host serves `ui/remoteEntry.js` and pins the `block.yaml`
> manifest hash at boot (SCOPE R3), so a `block.yaml` change requires
> rebuilding the binary against it and restarting the agent; table
> *column* additions are not auto-migrated (`boot::extension_tables`
> only CREATEs). UI-only changes just need the bundle redeployed.

---

*Companion docs:* [`README.md`](README.md) (the read-only dashboard this
sits beside), [`DB.md`](DB.md) (warehouse/perf playbook),
[`rubix/SCOPE.md`](../../SCOPE.md) (R2 upstream-first, R7 verbatim SQL,
R8 SDK-only dep rules this feature obeys).
