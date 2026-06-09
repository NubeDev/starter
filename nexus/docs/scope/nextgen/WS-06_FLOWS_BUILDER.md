# WS-06 — Flows: Visual Builder, Node Palette, Dry-Run & Metrics

> **Status:** Not started · **Wave:** 2 · **Owner:** _unassigned_
> **Depends on:** WS-08 (registered connectors enrich the palette) — can start with current nodes
> **Migration:** none required (flow config is JSONB); maybe `00xx_flow_runs.sql` for metrics
> **Read first:** GAP_ANALYSIS §2.6, ROADMAP §0 + §6 · this is the user's "ArkFlow admin tooling" ask
> **Verified:** `82a6a19a` on 2026-06-09 — re-grep this WS's file:line claims before building (ROADMAP §0).

## Goal
Make flows authorable by a human, not a JSON-savant. Today flows are **real and run**
(`FlowManager` executes ArkFlow `Stream`s) but are authored as **three raw-JSON textareas**. Give
admins a **visual node-graph editor** with a discoverable **node palette**, **schema-driven config
forms**, and a **validate + dry-run + preview** loop so they can "set this up, test it, and get it
working" (verbatim user ask).

## Current state (evidence)
- Backend flows are real: `nexus-engine/src/flow/manager.rs` builds + runs streams; CRUD +
  start/stop wired (`routes/flows/**`). Config is opaque `{input, pipeline, output}` JSONB
  (`nexus-store/src/flow/record.rs`).
- UI authoring = three `<Textarea>`s with JSON-syntax-only validation
  (`features/flows/FlowFormDialog.tsx`, `flowDraft.ts`). No palette, graph, schema, dry-run, preview,
  or metrics.
- **Tiny palette**: only `http_poll` + `simulator` inputs (`registry/inputs.rs`) and
  `collector`/`sse`/`postgres` outputs (`registry/outputs.rs`) are registered. (WS-08 adds more.)

## Scope
1. **Node-type registry endpoint** — `GET /api/v1/flows/node-types` → for each registered
   input/processor/output: `{ kind, category, label, description, configSchema (JSON Schema),
   inputs/outputs arity }`. Source of truth = the engine registry; add a metadata/schema layer next
   to `register_*_builder` so the registry can *describe* itself, not just build. (Backend work in
   `nexus-engine/src/registry/**` — coordinate the shared `registry/{inputs,outputs}.rs` 🔶 edit.)
2. **Visual graph editor** (`ui/src/features/flows/builder/**`, **React Flow** / @xyflow/react):
   - **Palette** populated from the node-types endpoint, grouped by category (inputs/processors/outputs).
   - Drag a node onto the canvas; **connect** input→processor(s)→output with edges.
   - Click a node → **schema-driven config form** (json-schema → rhf/zod form) instead of raw JSON.
   - Serialise the graph to the existing ArkFlow `{input, pipeline:{processors[]}, output}` shape so
     the backend is unchanged. Keep a "raw JSON" escape-hatch tab (round-trips with the graph).
3. **Validate + dry-run** — `POST /api/v1/flows/dry-run` (no persistence): builds the StreamConfig
   (real ArkFlow validation, not just JSON syntax), runs it against a **bounded sample** (the
   collector sink + row/time caps already exist — reuse `sink/cap.rs`), returns sample output rows +
   any build/runtime error. Surfaces errors *before* save. A "Test" button in the editor.
4. **Live preview** — show the dry-run sample rows in a result grid; for streaming inputs, tail a few
   batches via the existing SSE path with a hard time/row cap.
5. **Flow run metrics** — surface per-flow throughput / last-batch-at / last-error / running-state on
   the flows list and detail (the `FlowManager` task already has the lifecycle; expose counters).
   Optional `00xx_flow_runs.sql` for a run/event log.
6. **Templates** — a few starter flow templates (e.g. `http_poll → sql transform → postgres`,
   `mqtt → sse`) selectable as a starting graph.

## Design notes
- **Don't change the on-the-wire flow config** (`{input,pipeline,output}` JSONB) — the graph editor
  is a *better front-end* over the same shape, so the engine, FlowManager, and existing flows keep
  working. The editor compiles graph↔JSON both ways.
- **The registry must describe itself.** The cleanest path: each builder registration also registers
  a small descriptor (kind, category, JSON-schema for config). This is the one non-trivial backend
  change; keep it additive so it doesn't disturb the build path.
- **Dry-run reuses the bounded collector** (`nexus-engine/src/sink/cap.rs`) — never run an unbounded
  test stream from the editor. Same caps as panel queries.
- **Security**: dry-run executes a flow against real connectors → same tenant/authz/secret-decrypt
  boundary as a saved flow. No new credential exposure.

## Acceptance criteria
- [ ] Node-types endpoint lists every registered node with a config schema.
- [ ] An admin builds `http_poll → processor → postgres` entirely in the graph UI, no raw JSON.
- [ ] Config forms are generated from each node's schema; invalid config is caught in-form.
- [ ] "Test" runs a bounded dry-run and shows sample output rows (or a clear build error) without saving.
- [ ] The graph round-trips to the existing JSON config; existing flows still load + run.
- [ ] Flow list shows running state + last-error + basic throughput.
- [ ] Tests: graph↔JSON serialisation, schema-form generation, dry-run cap enforcement.

## Out of scope (hand off)
- Adding the actual new connectors (MQTT/Modbus/Kafka) → **WS-08** (this WS *displays* whatever is
  registered; coordinate so palette + connectors land together).
- Heavy ETL/CDC → explicitly out of v1 per NEXUS.md §13.
