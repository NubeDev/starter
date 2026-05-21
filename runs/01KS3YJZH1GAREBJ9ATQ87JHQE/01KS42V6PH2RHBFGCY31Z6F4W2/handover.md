## Done

- packages/starter-sdui-react/src/components/Custom.tsx: lookup now reads `node.renderer_id` (matches IR wire shape `Component::Custom { renderer_id }`); fallback stub tagged with `data-sdui-custom-stub`; once-per-id structured `sdui.custom.unknown_renderer` warn via useEffect; `__resetCustomWarningCacheForTests` exported for tests
- packages/starter-sdui-react/src/components/Custom.test.tsx (new): R7 "Custom renderer falls back cleanly" smoke — three cases (unknown id + sibling still renders; registered id replaces stub; missing renderer_id → sentinel stub). Mounts customSpec.Component directly via renderToStaticMarkup to bypass the shadcn `@/...` alias chain
- packages/starter-sdui-react/vitest.config.ts (new): node environment, src/**/*.test.{ts,tsx}
- packages/starter-sdui-react/README.md: registration example switched to `renderer_id`; new "Custom is a reference, not a node" section drawing the IR-vocabulary vs custom-dispatch distinction + authorisation-boundary cross-reference to `starter-sdui-routes`
- Verified `cargo test -p starter-sdui-routes` (8 passed including the 4 capability tests) and `pnpm test` / `pnpm typecheck` in starter-sdui-react (3 smokes pass, typecheck clean)
- Committed as `03be5c4` on `codeless/starter-sdui`

## Next

- Stage 11 (next session): per `DOCS/frontend/sdui/SCOPE.md` § "12-stage plan", the remaining items are the falsification smoke suite and the scrub-plan trigger check

## What you need to know

- The structured warn fires from a `useEffect` — under `renderToStaticMarkup` (SSR) effects don't run, so the smoke pins the *markup* contract (stub element + sentinel id attribute) rather than asserting `console.warn`. The exactly-once contract still holds at runtime because `_warnedIds` is a module-scoped `Set`; a host-app `@testing-library/react` integration is the right place to pin the warn-call contract if/when one is set up
- The server-side `CapabilityFilter::rewrite_unknown_custom` (crates/starter-sdui-routes/src/capability.rs) was already in place from stage 6 and rewrites unknown ids to `Component::Dangling` server-side — the client-side stub in Custom.tsx is the second line of defence for trees that bypass the filter (or for late-unregistered ids)
- The crate-level threat-model paragraph in `crates/starter-sdui-routes/src/lib.rs` already documents the R7 threat model (renderer_id public, custom.props auth at handler/resolve boundary, filter is vocabulary not auth) — left unchanged

## Open questions

- The SCOPE doesn't have a literal section heading "Nodes vs IR components" (the stage spec references it); I inferred the content from R7's framing of `custom` as the escape hatch. Worth a sanity-check from the next session whether the README phrasing matches the intent
