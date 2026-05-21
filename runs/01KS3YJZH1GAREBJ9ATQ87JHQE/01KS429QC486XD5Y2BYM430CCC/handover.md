## Done

- Ported remaining IR variants to React renderer: chart (with sources[] + dual-field source/sources tolerance and $page.chart_range zoom round-trip), sparkline, tree (with $node.id substitution), timeline (streamable, append/replace), markdown + code (streamable), wizard, drawer, rich_text (textarea fallback per "wrapper only counts" rule), diff (with annotations + line_action / $line + $side substitution), ref_picker, date_range.
- Added `useStreaming` hook + `StreamEndSentinel` type honouring S-D5 (`reason: done | error | timeout | gone`); transport-agnostic, host-provided. On unmount the subscription is dropped so the server's 60s inactivity GC closes the channel within the same window.
- Plumbed `streamingTransport?: SubscriptionTransport` through `SduiCtx` and `SduiPage` (additive, no signature change to existing fields, Renderer dispatcher untouched, IR untouched -- stage 7 freeze holds).
- chart_range round-trip works without code change: `setPageState({ [page_state_key]: range })` triggers a re-resolve because `pageState` is in `SduiPage`'s React-Query key (R9 -- server is authoritative).
- Verified all seven R8 DoS-limit smokes (`page_state_bytes`, `render_tree_bytes`, `tree_nodes`, `tree_depth`, `component_types`, `handler_timeout`, `table_rows_per_page`) pass in `crates/starter-sdui-routes/tests/limits_413.rs`.
- Total built-in component LoC = 1937 (target 3000, red line 4000); `Renderer.tsx` = 68 lines (gate 800).
- pnpm typecheck + cargo test -p starter-sdui-routes both green.
- Committed as `c3779ac` with message starting with the stage title.

## Next

- Stage 10 (Phase 7 -- `custom` escape-hatch wiring per SCOPE.md table: registry, capability filter, fallback stub; server-side rewrite of unknown renderer_id to `dangling` per R7). The Custom.tsx fallback stub already exists from Phase 4; stage 10 wires the server-side capability filter and the unknown-id rewrite.

## What you need to know

- IR variants for all these were already present in `crates/starter-ui-ir/src/component.rs` from stage 2 -- stage 9 was strictly the React side plus the streaming hook.
- `streamingTransport` reuses the existing `SubscriptionTransport` interface from `useSubscriptions.ts`. Hosts can pass one transport that handles both slot updates and streaming chunks (the sentinel `{ type: "stream_end" }` is the discriminator).
- `text` dispatches to streaming when `node.subscribe` is set; static `text` is unchanged. Kept the registry single-spec-per-kind invariant.
- `rich_text` and `diff` wrappers are intentionally thin -- the SCOPE explicitly excludes the heavy library payload from the LoC budget. Hosts that want monaco-diff / tiptap register a `custom` renderer with the same kind id.
- Stage 7's freeze is respected: Component IR, EntityGraph, builder DSL, Renderer dispatcher signature, HandlerRegistry / QueryEngine traits, and the three /ui/* routes were not changed. `SduiCtx` gained one optional field (`streamingTransport`); the `Kind` union gained additive variant ids. Neither is a signature change to the listed surfaces.

## Open questions

- (none)
