# Rubix PMS — Project Builder (POC)

A **standalone, no-backend** React POC for building BMS / electrical-EMS
projects from device templates, then exporting them to **PDF**, **Excel**,
and **provision JSON** importable by rubixos
(`com.nubeio.rubixos/process/src/provision`).

This is a sibling experiment to the `com.nubeio.rubixos` barcode/provision
feature — it reuses the same *shape* (site → location/gateway → device →
points, with `repeat:` point expansion and alarm rules) but stands entirely
on its own (Vite + React 19 + TS + Tailwind 4, state in `localStorage`).

## Concept

```
Admin                                   Client
─────                                   ──────
• add Clients & Sites                   • create a Project against a Site
• load Device Templates                 • drop Gateways (network head-ends)
    – gateways (network)                • add End Devices under each gateway
    – end devices (network,             • configure settings per instance
      settings, points)                 • export → PDF / Excel / Provision JSON
```

## Hierarchy / schema

`Client → Site → Gateway → NetworkBus(network, cap) → EndDevice(settings, points)`

- **Gateway** template: supported `networks` + `settings`. When a gateway is
  added, one **NetworkBus** is created per supported network (the head-end's
  physical ports). A gateway that speaks Modbus RTU *and* BACnet MS/TP has two
  buses.
- **NetworkBus**: a single field bus — `network`, a `maxDevices` cap (default
  from `NETWORK_META`, editable), and its `devices[]`. Caps reflect the
  physical limit: Modbus RTU / RS-485 → 32, BACnet MS/TP → 127, Modbus TCP →
  247, LoRa/IP → large.
- **End-device** template: `networks` (which buses it may join), `settings`,
  and `points`. Points carry `kind`, `widget`, `writable`, `trend`, optional
  `repeat:N` (expands to `key1..keyN`, like rubixos `io_22`), and `alarms`.

### Network rules ([src/lib/networks.ts](src/lib/networks.ts))

- **Compatibility** — a device can only join a bus whose `network` is in the
  device template's `networks[]`. A Modbus-only meter cannot drop onto a
  BACnet bus; the drop is rejected with a reason.
- **Capacity** — each bus enforces `maxDevices`. Drops/bulk-adds clamp to the
  free slots; a full bus rejects with a reason.
- **Addressing** — addressed networks (Modbus/BACnet) auto-assign the next
  free numeric address; non-addressed (LoRa) use an `idTag` (DevEUI).

## Visual builder ([src/pages/NetworkCanvas.tsx](src/pages/NetworkCanvas.tsx))

The **Canvas** view (React Flow) is a CAD-style schematic of
`gateway → bus → devices`, color-coded per network, on a dark engineering
grid with a network **legend**. A left palette lists device templates you
**drag onto a bus** — incompatible or full buses reject the drop and flash a
reason. Each bus head shows a `used/max` meter that turns red at capacity.
Selecting a bus opens its **bulk-add** panel.

### Wiring topology (daisy-chain vs star)

Each network has a physical `topology` ([NETWORK_META](src/types/index.ts))
that drives how its bus is drawn ([src/lib/canvasLayout.ts](src/lib/canvasLayout.ts)):

- **`bus` (daisy-chain)** — RS-485, Modbus RTU, BACnet MS/TP, M-Bus. Devices
  are laid out **left → right in series** along a single colored trunk rail
  (`head → dev1 → dev2 → …`), and the run ends in a **120 Ω terminator**
  symbol — the way a real multidrop serial segment is wired.
- **`star`** — Ethernet/IP, Wi-Fi, LoRaWAN, Modbus TCP, BACnet/IP. Devices
  **fan out** from the head independently, each on its own branch.

Addresses render on each node (`addr N` for addressed buses, the `idTag` /
DevEUI for id-based ones), with a `#seq` position index.

The **Form** view is the same project as nested cards (gateway → bus → device
table) for keyboard-driven editing. Toggle between them in the header; both
mutate the same state via [src/lib/projectEdits.ts](src/lib/projectEdits.ts).

### Bulk add ([src/lib/bulkAdd.ts](src/lib/bulkAdd.ts)) — two modes

- **Count + start address** — add N devices from a template, sequential
  addresses from a start, skipping taken ones, clamped to the cap.
- **Address range** — fill an explicit range like `1-32`; taken/over-cap
  addresses are skipped and reported.

See [`src/types/index.ts`](src/types/index.ts) for the full schema and
[`src/data/seed.ts`](src/data/seed.ts) for the sample templates (Rubix Edge
gateway, RS-485/Modbus gateway, LoRaWAN gateway, 3-phase energy meter, Droplet
sensor, IO-22, VAV controller).

## Exports

| Format | File | Use |
|--------|------|-----|
| **PDF** | `<project>.pdf` | Human-readable design document: site summary, per-gateway device schedule, full points schedule. |
| **Excel** | `<project>.xlsx` | 4 sheets — Site / Gateways / Devices / Points. |
| **Provision JSON** | `<project>.provision.json` | `rubix.provision/v1` bundle: `site → locations[] → {gateway, buses[] → devices[] → points[]}`. Maps a POC project onto the rubixos provision import shape. |

The mapping lives in [`src/lib/provision.ts`](src/lib/provision.ts): a gateway
lives at the site, so `Gateway → location` (with its own device row), each
`NetworkBus → a bus segment` inside the location, `EndDevice → device`, and
template points → repeat-expanded point rows with alarms. The Excel workbook
gains a **Buses** sheet alongside Site / Gateways / Devices / Points.

## Run

```bash
cd rubix/extensions/com.nubeio.pms/poc
npm install
npm run dev      # http://localhost:5174
npm run build    # tsc + vite production build
```

All data persists in `localStorage` (`pms-poc-state-v1`). "Reset demo data"
in the sidebar restores the seed. Templates can be exported/imported as JSON
on the Templates page.

## Next steps (not in POC)

- YAML template authoring matching rubixos `templates/*.yaml` exactly.
- A `bc_*`-style importer in `process/src/provision` that reads the
  `rubix.provision/v1` bundle and creates the site/devices/points rows.
- Per-user authz gating (see the rubix `authz_scope` design).
