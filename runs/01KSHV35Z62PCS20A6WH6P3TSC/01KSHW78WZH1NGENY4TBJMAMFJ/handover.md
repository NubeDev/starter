## Done

- Added `VISUALS` map to `packages/starter-ui-sdui-react/src/headless/placeholder-render.tsx` with per-variant JSX fillers for the 32 IR variants that previously fell through to the dangling tile (text, heading, badge, diff, field_group, section, array_table, json_table, list, dialog, menu, tree, timeline, markdown, rich_text, markdown_editor, ref_picker, detail, card, date_range, wizard, drawer, button, text_field, number_field, textarea, select_field, radio_group, segmented, date_field, checkbox, action_widget). RESOLVER_ONLY_VARIANTS (forbidden/dangling/unknown) remain excluded.
- Each placeholder mirrors the live renderer's visual idiom (table header+rows, segmented control bar, wizard stepper, etc.) and is keyed by `data-sdui-placeholder="<variant>"`.
- Added 32 snapshot-style tests in `packages/starter-ui-sdui-react/src/renderer/placeholder-render.test.tsx` (one per variant) — pnpm --filter @nube/starter-ui-sdui-react test now reports 56/56 green.
- Updated `packages/starter-ui-sdui-puck/README.md` §B2 follow-up note from "open" to ✅ with the variant list.
- Commit `8af7801` on branch `codeless/puck-builder-finish`.

## Next

- Stage 4 — §B6 runtime schema-hash banner.
- Stage 5 — scope 11 (live-canvas SSE banner + revalidate-on-resume).

## What you need to know

- The variant enumeration was done by walking `crates/starter-ui-ir/schema/starter-ui-ir.schema.json` `definitions.Component.oneOf` (path differs from job brief's `packages/starter-ui-ir/schema/...` — the schema lives under `crates/`).
- `VISUALS` takes precedence over `FILLERS`/`lookupRenderer` in `PlaceholderRender`; variants without a live renderer no longer dispatch through `Render` and don't need a transport.
- Live `Render` walker is unchanged — `RenderForm`'s children still hit the live registry, so the existing form test (form children remain as raw input nodes) was unaffected.
- Typecheck via `pnpm --filter @nube/starter-ui-sdui-react exec tsc --noEmit` is clean.

## Open questions

- (none)
