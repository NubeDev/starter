# `com.rubix.geo` — generic map + pins over the warehouse

MapLibre-based map dashboard. Pins and layers live in two
extension-owned warehouse tables; pin click-actions can run a named
warehouse template or call any registered MCP tool (any other
extension's tool, gated by the host's normal capability checks).

Built against `starter-ext-sdk` only (SCOPE R8). Mirrors the layout of
[`com.rubix.example`](../com.rubix.example).

## What it ships

| surface                       | what it contributes                                                                                  |
|-------------------------------|------------------------------------------------------------------------------------------------------|
| `tools[]`                     | `warehouse_query` (proxy), `pin_create/update/delete`, `layer_create/update/delete`                  |
| `warehouse_tables[]`          | `com_rubix_geo__pins`, `com_rubix_geo__map_layers` (per-tenant, host stamps `tenant_id`)             |
| `warehouse_templates[]`       | `pins_list`, `pins_by_layer`, `pins_in_bbox`, `layers_list`                                          |
| `ui[]` (Module Federation)    | `Main` (map dashboard), `NavTree` (sidebar nav), `Sidebar` (status badge)                            |

## Data model — pins

```jsonc
{
  "pin_id": "ulid-or-uuid",                      // operator-supplied
  "layer_id": "warehouses",                      // optional; null = unassigned
  "name": "Main Warehouse",
  "description": "...",
  "lng": 153.0260, "lat": -27.4705,
  "geometry_type": "Point",                      // Point | LineString | Polygon
  "geometry": null,                              // GeoJSON when not Point
  "icon": "warehouse", "color": "#22c55e",
  "actions": [                                   // resolved client-side
    {
      "id": "history",
      "label": "Recent history",
      "kind": "template",
      "target": "com.nubeio.rubixos.history_recent",
      "params": { "point_uuid": "{{props.point_uuid}}" },
      "display": "table"
    },
    {
      "id": "decode",
      "label": "Decode BACnet",
      "kind": "tool",
      "target": "com.nubeio.rubixos.bacnet_decode",
      "params": { "device_id": "{{props.device_id}}" },
      "display": "json"
    },
    {
      "id": "open",
      "label": "Open device page",
      "kind": "url",
      "target": "/extensions/com.nubeio.rubixos/devices/{{props.device_id}}"
    }
  ],
  "props": { "device_id": "abc", "point_uuid": "..." }
}
```

`{{props.*}}` and `{{pin.*}}` substitution happens in the UI just
before the action is dispatched — params are still a JSON object on
the wire.

## Pin-action security

The browser calls `/api/v1/tools/<target>` with the operator's session
cookie. The host enforces:

- session must be valid (login required to see the map at all),
- the operator's role + the target tool's allowlist (admin / role
  gating is the host's job, not the extension's),
- audit row written for every tool call.

The extension does NOT bypass the gate — there is no
`run_pin_action` server proxy. A pin can never be configured to call
something the operator viewing the map isn't already allowed to call.

## Layout

```
com.rubix.geo/
├── block.yaml
├── Makefile
├── README.md
├── process/                          # Rust binary (tools)
│   ├── Cargo.toml
│   └── src/main.rs
├── kinds/                            # JSON Schemas + .md + .sql
│   ├── warehouse_query_*.{json,md}
│   ├── pin_{create,update,delete}_in.json + pin_write_out.json + pin_crud.md
│   ├── layer_{create,update,delete}_in.json + layer_write_out.json + layer_crud.md
│   └── pins_list / pins_by_layer / pins_in_bbox / layers_list — params + sql
└── ui-src/                           # MapLibre UI (built to ui/remoteEntry.js)
    ├── main.tsx                      # map dashboard
    ├── nav-tree.tsx, sidebar.tsx
    ├── components/map.tsx            # MapLibre wrapper + click-to-add
    ├── components/pin-editor.tsx
    ├── components/pin-popup.tsx
    ├── api.ts                        # /api/v1/tools/* helpers
    └── types.ts
```

## Build

```bash
make all          # ui-build + cargo build --release + install + load + test
```

Same target convention as
[`com.rubix.example/Makefile`](../com.rubix.example/Makefile).

## Known scope cuts (v1, deferred to peer review / follow-up)

- **Draw tools for lines/polygons.** v1 supports click-to-add-point
  only. Adding `@mapbox/mapbox-gl-draw` (works fine over MapLibre)
  enables LineString/Polygon authoring; the schema (`geometry_type`,
  `geometry` JSONB) already accommodates it.
- **Heatmap source.** Not wired. `pins_in_bbox` + MapLibre's
  `heatmap` layer type is the path.
- **Live updates via SSE.** Not wired. Refresh button only for v1.
- **Spatial index.** `pins_in_bbox` uses plain `BETWEEN` on
  `lng`/`lat`. For large pin sets, add a PostGIS geometry column and
  a GiST index in a follow-up migration.
- **Per-tenant style URL secret.** `style_url` is a plain TEXT
  column. For style providers needing tokens (Mapbox, MapTiler), the
  follow-up is to resolve a `secrets:` capability at render time.
