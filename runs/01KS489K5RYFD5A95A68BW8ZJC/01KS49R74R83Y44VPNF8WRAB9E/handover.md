## Done

- `PagesList.tsx` — cards grid with the spec'd empty-state copy ("No pages yet — hit + New page to start"); "+ New page" → `/pages/new`. Driven by `usePages()` so in-tab and cross-tab updates flow live.
- `PageBuilder.tsx` — composes `useBuilder` + `<BuilderTranscript>` + `<AiBuilderCanvas>` directly (does **not** wrap `<AiBuilder>`). Host owns `tree` + writes a save button that calls `pages-store.savePage({ id, name, tree })` then navigates to `/pages/:id`. Accepts an `initialTree` prop, and also hydrates from `:id` route param for `/pages/:id/edit`.
- `PageView.tsx` — `<Renderer node={page.tree.root} />` inside `<SduiHost>` with Edit / Duplicate / Delete (delete behind an `<AlertDialog>`).
- `Skills.tsx` — drop-in `<SkillsManager>` over `createInMemorySkillsAdapter` seeded with the two reference bundles (`starter.ai-builder.dashboards` approved, `starter.ai-builder.themes` quarantined so acceptance #5 — moving it to Approved without a refresh — is exercisable).
- Committed as `8905e28` with the stage-4 title.

## Next

- Stage 5: wire routes in `src/app.tsx` (`/pages`, `/pages/new`, `/pages/:id`, `/pages/:id/edit`, `/skills`) and add the new sidebar entries in `src/layout/Shell.tsx` (Pages section driven by `usePages()`; Skills entry). Verify the Lighthouse / acceptance checklist in `PAGE-BUILDER.md`.

## What you need to know

- `PageBuilder.tsx` reads from `pages-store.getPage(id)` synchronously inside a `useMemo` keyed by the route param — fine because the store is localStorage-backed and the value is stable across renders. The seeded `tree`/`name` only flow into `useBuilder`'s `initialTree` and `useState(name)` on first mount, matching React's expected initial-value semantics.
- The fixture skill bundles use placeholder 64-char hex blake3-shaped strings for `bundleHash` and `contentHash`; the in-memory adapter only checks `bundleHash` equality on `approve`, so the round-trip works.
- Skills body text is a verbatim excerpt of the SKILL.md bodies under `/skills/starter.ai-builder.*/SKILL.md` — kept inline rather than imported because the bundles aren't on the JS module graph.
- Typecheck errors that surface in `pnpm typecheck` (`starter-sdui-react/src/components/Display.tsx`, `starter-ui-kit/src/components/ui/{command,sidebar,toggle-group}.tsx`) are pre-existing in workspace packages and untouched by this stage.

## Open questions

- (none)
