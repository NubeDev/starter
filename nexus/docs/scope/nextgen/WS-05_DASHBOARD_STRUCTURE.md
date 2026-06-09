# WS-05 — Dashboard Structure (Folders · Rows · Repeat · JSON Model · Versioning · Share)

> **Status:** Not started · **Wave:** 0 (owns the JSON model contract C1) + Wave 3 (repeat/versioning)
> **Owner:** _unassigned_ · **Depends on:** WS-02 (repeat-by-variable); owns C1
> **Migrations:** `0006_dashboard_model.sql`, `0009_dashboard_versions.sql` · **Read first:** GAP_ANALYSIS §2.5, ROADMAP §6 (C1)

## Goal
Turn a flat list of dashboards into an organised, scalable, **dashboard-as-code** system: folders,
collapsible rows, repeat-by-variable, a stable importable/exportable **JSON model**, version history
with restore, and richer sharing (public/snapshot/embed). This is also what makes **AI-generated
dashboards** (the "Ask Nexus" vision in `data/types.ts`) and **GitOps** possible.

## Current state (evidence)
- Flat sidebar list (`features/dashboards/SidebarDashboards.tsx`); routes by slug.
- No folders, rows, repeat, import/export, versioning, or duplicate. `starred` exists in the type
  (`data/types.ts:111`) but isn't wired.
- Sharing is solid (view/edit/delete grants, `authz/dashboard_instances.rs`) but has **no public
  link, snapshot, or embed**.
- The data model is intentionally "stack-agnostic" (`data/types.ts` header) — a good basis for a
  serialisable JSON model, but no import/export consumes it yet.

## Scope
### Wave-0 deliverable: C1 dashboard JSON model (do this first — others depend on it)
Freeze the serialised shape. Extend `Dashboard` (`ui/src/data/types.ts`) + mirror in `nexus-spi`:
```
Dashboard {
  schemaVersion: number,
  id, name, slug, description?, icon, accent, starred?,
  folderId?: string,
  timeDefaults?: { from, to, refresh },      // WS-01 writes
  variables?: Variable[],                     // WS-02 writes
  rows?: Row[],                               // collapsible sections
  widgets: Widget[],                          // widgets reference rowId?
  updatedAt, version
}
Widget { ..., fieldConfig?, transforms?, repeat?: { varName, direction } }  // WS-04 / repeat
Row { id, title, collapsed, repeat?: { varName } }
```
Add `0006_dashboard_model.sql`: a `definition JSONB` column on the dashboard table (or per-section
columns) + a `folders` table. **This JSON shape is the contract WS-01/02/04 serialise into and
import/export validates against.** Publish it as `schemaVersion: 1` with a migration note.

### Folders
- `folders` table (tenant-scoped, RLS), nestable (parent_id). CRUD endpoints. Move dashboards
  between folders. Folder tree in the sidebar; folder-level authz grants (coordinate with authz).

### Rows
- Collapsible row sections grouping panels on the canvas; persisted in the model; collapse state
  per dashboard. Drag panels into rows.

### Repeat-by-variable
- A panel or row with `repeat.varName` renders once per resolved value of that variable (from
  **WS-02**). Lay out repeated instances in the grid. The killer fleet feature.

### JSON import/export
- `GET /dashboards/:id/export` → the C1 JSON; `POST /dashboards/import` validates against
  `schemaVersion` and creates/updates. UI "Export JSON" / "Import JSON" / copy-to-clipboard.
- This is the seam the **AI "Ask Nexus" generator** emits into.

### Versioning
- `0009_dashboard_versions.sql`: snapshot the JSON model on each save with `{version, author,
  message?, created_at}`. `GET /dashboards/:id/versions`, `GET .../versions/:v`, restore endpoint.
  UI: version list + **diff view** + restore.

### Sharing extensions (next to existing authz, don't replace it)
- **Public/anonymous link** — opt-in, read-only, token-scoped URL (tenant-gated config).
- **Snapshot** — a frozen point-in-time copy (data embedded) shareable without datasource access.
- **Embed** — iframe/embed snippet for a panel or dashboard with a scoped token.
- **Duplicate** dashboard; **star/favorite** wired to the existing `starred` field.

## Design notes
- **Self-contained export**: variables + time defaults + panel fieldConfig all live *in* the JSON so
  an exported dashboard is portable. This is why C1 must land before WS-01/02/04 serialise.
- **Versioning stores the whole JSON model** (simple, diffable) rather than deltas — storage is cheap
  vs. the complexity of delta-reconstruction.
- **Public/snapshot are a security surface** — read-only, token-scoped, RLS-aware, no datasource
  credential exposure (snapshots embed *data*, not connections). Coordinate with WS-09 audit.
- Reuse `authz/dashboard_instances.rs` patterns for folder grants; don't fork the grant model.

## Acceptance criteria
- [ ] C1 JSON model published as `schemaVersion: 1`; round-trips export→import losslessly.
- [ ] Folders: create/nest/move; sidebar tree; folder grants enforced.
- [ ] Collapsible rows persist; panels group correctly.
- [ ] A row/panel repeats once per `$region` value (with WS-02 present).
- [ ] Version history records saves; diff + restore work.
- [ ] Public link renders read-only without auth; snapshot needs no datasource; duplicate + star work.
- [ ] Tests: model (de)serialisation + back-compat, repeat layout, version diff, import validation.

## Out of scope (hand off)
- The variable model → WS-02 (this WS consumes it for repeat).
- Time defaults semantics → WS-01 (this WS just stores them in the model).
- Annotations → could live here or a small follow-up WS; note as a gap if deferred.
