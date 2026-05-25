# 08 — Open questions

> **Tier:** scope (plan). Lifetime: weeks. Not referenced from code.

All ten open questions raised during the Goal-1 scope (Q1–Q10)
were resolved during phases A–E and folded into the corresponding
promoted design doc:

| # | Question | Resolved in |
|---|---|---|
| Q1 | Page storage location | [`docs/design/sdui/storage/`](../../design/sdui/storage/README.md) — rubix-owned PG table. |
| Q2 | AI-authored page owner principal | [`docs/design/sdui/tools/`](../../design/sdui/tools/README.md) — invoking operator. |
| Q3 | Bundled-page upsert vs operator collision | [`docs/design/sdui/storage/`](../../design/sdui/storage/README.md) — operator wins. |
| Q4 | Live subscription transport | [`docs/design/sdui/renderer/`](../../design/sdui/renderer/README.md) — SSE with polling fallback. |
| Q5 | Per-tenant isolation | [`docs/design/sdui/storage/`](../../design/sdui/storage/README.md) — row filter on `tenant_id` for v1. |
| Q6 | Schema authoring contract for the LLM | [`docs/design/sdui/ai-builder/`](../../design/sdui/ai-builder/README.md) — pruned schema per `skill_hint`. |
| Q7 | Renderer-id session cache | [`docs/design/sdui/renderer/`](../../design/sdui/renderer/README.md) — deferred to v2. |
| Q8 | Bundled-page strings | [`docs/design/sdui/bindings/`](../../design/sdui/bindings/README.md) — `$msg.<key>` source (G6). |
| Q9 | `Component::Custom` renderer-id listing | [`docs/design/sdui/renderer/`](../../design/sdui/renderer/README.md) — hard-coded in `@nube/starter-ui-sdui-react` for v1. |
| Q10 | RSQL-backed `Table` source | [`docs/design/sdui/host-glue/`](../../design/sdui/host-glue/README.md) — kind-only filter in v1. |

No open questions remain on Goal 1. New questions raised by the
v2 fetch-plan work belong in
[`docs/scope/dashboards/07-fetch-plan.md`](./07-fetch-plan.md).
