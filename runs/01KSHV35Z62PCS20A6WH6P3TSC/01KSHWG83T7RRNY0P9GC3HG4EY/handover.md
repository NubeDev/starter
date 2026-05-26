## Done

- Added `packages/starter-ui-sdui-puck/scripts/emit-schema-hash.mjs` (sha256 over the committed `crates/starter-ui-ir/schema/starter-ui-ir.schema.json`); writes `src/schema-hash.json` and exits non-zero if the committed sidecar was stale so CI catches drift.
- Wired the emitter into the package's `test` script (runs after the existing `check-schema-drift.mjs`, before vitest).
- Added `src/schema-hash.ts` consuming the JSON sidecar at build time; exported `IR_SCHEMA_HASH` + `IR_SCHEMA_HASH_ALGORITHM` from `src/index.ts`.
- Extended `PuckBuilder` with a `liveSchemaHash?: string` prop and a non-blocking amber banner ("schema drifted — refresh to reload the palette", `data-puck-builder-schema-drift`) that renders inside the canvas when the live hash differs from the bundled one.
- Added `src/__tests__/schema-hash.test.ts` asserting the sidecar matches `sha256(committed-schema-bytes)` and the hex shape.
- Frontend `rubix/frontend/src/routes/dashboards/$pageId_.edit.tsx` best-effort-fetches `GET /api/v1/ui/schema/hash` and threads the value into `<PuckBuilder liveSchemaHash>`; failures (404/network) silently keep the banner dormant.
- Updated `packages/starter-ui-sdui-puck/README.md` §B6 row from "⏳ CI-time guard only" to "✅" with notes, and dropped the matching "Next tasks" entry.
- `pnpm --filter @nube/starter-ui-sdui-puck test` (21/21) + `typecheck`, `pnpm --filter @nube/rubix-frontend typecheck`, and `pnpm --filter @nube/starter-ui-sdui-react test` (56/56) all green.
- Committed on `codeless/puck-builder-finish` as `80a2018` "stage 4 — §B6 runtime schema-hash banner".

## Next

- Stage 5 (scope 11 live-canvas SSE banner + revalidate-on-resume) — a fresh session picks this up.

## What you need to know

- The rubix-agent does not currently ship a schema-hash verb/endpoint. The route attempts the proposed REST endpoint `GET /api/v1/ui/schema/hash` (referenced in `rubix/docs/design/sdui/components/README.md`); without a server-side handler the banner never appears. Adding that endpoint (e.g. in `starter-sdui-routes` or a small rubix-agent route) is a one-off follow-up — it should return `{"hash": "<sha256-hex>"}` computed over `starter_ui_ir::schema::emit_tree_schema()` bytes so it matches the frontend's sidecar exactly.
- `src/schema-hash.json` is committed; CI's `pnpm test` will fail if it drifts from the schema bytes, mirroring the `check-schema-drift.mjs` pattern.
- The banner uses `role="status"` and `data-puck-builder-schema-drift` for selector hooks.

## Open questions

- (none)
