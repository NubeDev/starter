## Done

- Wired `<FlowCanvas>` into `examples/flow-agent/frontend/src/pages/FlowEditor.tsx` with a module-scope `NodeKindRegistry` seeded by `BUILTIN_NODE_KINDS`.
- Loads via `useQuery(["flow", id])`, mirrors the graph into local state, marks dirty on canvas `onChange`, and PUTs (with the loaded `version`) on Save; success path updates the react-query cache and clears dirty.
- 409 surfaces as an Alert: "keep editing" leaves edits intact, "reload server graph" pulls the latest and resets dirty. Generic errors render as a separate save-failed Alert.
- Added a top palette strip (and a floating `<NodePalette>` overlay) that inserts new nodes with unique ids and offset positions; positions persist through `graph_json` round-trips.
- `api.ts` now throws `ApiError` with `.status` so callers can branch on 409.
- Fixed three pre-existing strict-TS errors in `packages/starter-ui-flow` that the new consumer surfaced (`RFNodeData.kindSpec` optional, `useTypedConnect` accepts `Edge | Connection`, `builtins.tsx` casts through `unknown`).
- Committed as `18a8b78` on `codeless/flow-agent-example`. `pnpm typecheck` green for `flow-agent-frontend` and `@nube/starter-ui-flow`.

## Next

- Stage 2 (next session): per `examples/flow-agent/SCOPE.md` — likely AgentsList/AgentChat wiring against the existing REST surface.

## What you need to know

- The canvas owns its own React Flow state seeded once from the `initial` prop, so the editor force-remounts it (via a `canvasKey` counter) whenever the graph changes from outside (initial load, palette insert, server reload). Internal canvas `onChange` events do NOT bump the key — they just update the editor's mirror state.
- `pnpm -r typecheck` still shows a pre-existing failure in `starter-extensions/examples/notes` (`Panel.tsx` uses `client.get/post` which don't exist on `StarterClient`). Unrelated to this stage.
- `starter-ui-kit` has no toast primitive yet; the 409 / error UX uses `<Alert>` banners. Swap to a toast later if one lands.
- Node ids generated via `crypto.randomUUID()` with a kind prefix (`ai-agent-<8hex>`). Edge ids come from React Flow's `addEdge`.

## Open questions

- Backend currently returns a generic error message on version mismatch; the alert just shows "Server has a newer version (vN)" using the refetched flow. If the backend ever exposes a structured 409 body (server version, server graph), we could skip the extra GET.
- Whether the floating `<NodePalette>` overlay should stay once Stage F6's polish pass lands — it duplicates the top strip but gives the categorized view. Leaving both for now.
