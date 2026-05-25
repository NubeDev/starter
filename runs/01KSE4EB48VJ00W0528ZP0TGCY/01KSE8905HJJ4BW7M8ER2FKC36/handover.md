## Done

- Replaced /flows/$flowId synthetic placeholder graph with real body_yaml parsing from useFlowsList() (cached at FLOW_OPS_KEY+'list'); wired useFlowEvents(flowId) runOverlay into <FlowCanvas overlay>; xyflow single-node selection tracked via reactFlowProps.onSelectionChange.
- Added rubix/frontend/src/routes/flows/settings-sidebar.tsx (~310 lines): looks up node by id in parsed YAML, fetches kind via useFlowKinds, renders primitive JSON-Schema form (string/number/integer/boolean/enum) with raw-JSON textarea fallback for non-simple schemas. Save calls flowDeploy with the round-tripped YAML (uses yaml@2 parseDocument to preserve comments/ordering); conflict surfaces inline.
- Added `yaml@^2.9` dep to rubix-frontend; ran pnpm install.
- pnpm --filter @nube/rubix-frontend typecheck green; no test script defined for the frontend (only e2e/playwright), so `pnpm test` exits 0 with no output.
- Committed as `stage 13: phase E.3 — live values overlay + settings sidebar on /flows/$flowId`.

## Next

- (none) — stage 14 will be picked up by a fresh session.

## What you need to know

- FlowGraph layout is naive column-major (x = 80 + i*280, y = 160). Good enough for the linear tick-counter demo; swap for dagre when richer flows arrive.
- Settings sidebar uses `yaml.parseDocument` for write-side mutation (preserves comments/ordering) and a plain `YAML.parse` for the canvas read path.
- `useFlowEvents` `runOverlay` is structurally compatible with `@nube/starter-ui-flow`'s `RunOverlay`; the route does a single `as unknown as RunOverlay` cast at the mount site per the hook's docstring.
- Bundled tick-counter node kinds (`starter.flow.counter`, `starter.flow.trigger.schedule`, `starter.flow.log`) — only `starter.flow.counter` has a registry entry in `lib/flow-registry.ts` (via BUILTIN). The other two render with the registry's generic fallback; consider registering matching specs in a later stage.
- xyflow selection plumbing uses the `reactFlowProps` escape hatch instead of adding a dedicated `onSelectionChange` to `<FlowCanvas>` — keeps the upstream package surface untouched.

## Open questions

- The route imports `Empty` is no longer needed (we render a custom sidebar empty state instead). Left dependency untouched; nothing to clean up here.
- No frontend test framework present (Playwright e2e only). Stage required "test green" — interpreted as no-op since `pnpm test` has no script and exits 0.
