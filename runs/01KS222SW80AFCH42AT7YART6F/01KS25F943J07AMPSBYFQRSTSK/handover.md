## Done

- Replaced raw `#94a3b8` hex literal in `FlowEditor.tsx` palette swatch with a `bg-muted-foreground/60` Tailwind fallback (spec-provided colours still applied via inline style, but no hex authored in TSX).
- Cards in `FlowsList`, `AgentsList`, and `Settings` now carry `rounded-xl border border-border/60 shadow-sm ring-0` (twMerge wins over the kit's `rounded-4xl`/`shadow-md` defaults).
- `FlowsList` + `AgentsList` empty states now use `<Empty>` / `<EmptyMedia>` / `<EmptyTitle>` / `<EmptyDescription>` / `<EmptyContent>` from `@nube/starter-ui-kit` with a centred SVG slot, one-line helper, and a primary action that focuses the create input.
- `AgentChat` passes `suggestions=[…]` to `<Chat>` so the empty state has actionable affordances.
- `FlowCanvas` wrapper in `FlowEditor` now sets `bg-background`, satisfying the `--background`-token requirement.
- Upstream fix in `packages/starter-ui-chat/src/components/chat-message.tsx`: bubble max width 85 % → 70 % so user-right / assistant-left bubbles match macOS Messages — flows to every consumer (R2 + F2 compliant).
- New `examples/flow-agent/README.md` documenting `cargo run -p flow-agent`, `pnpm --filter flow-agent-frontend dev`, provider setup, demo flow, layout, and the CI command list.
- Cleaned up clippy: added `FlowRow` / `AgentRow` / `RunRow` aliases in `src/store.rs`, replaced `.map(|t| serde_json::to_string(t))` with `.map(serde_json::to_string)`, and unwrapped the lazy doc-continuation in `src/main.rs`.
- `pnpm --filter flow-agent-frontend typecheck`, `pnpm -F @nube/starter-ui-chat typecheck`, `cargo build -p flow-agent`, and `cargo clippy -p flow-agent -- -D warnings` all green.
- Committed as `stage 7 — F6 visual polish + README` (commit `2731b09`).

## Next

- (none) — this was the final stage.

## What you need to know

- The frosted-glass 56 px topbar, sidebar icon-rail at ≤ 1024 px, caret rotation (`transition-transform duration-150 group-data-[state=open]/group-label:rotate-90`), and `bg-accent/60` active rows were already in `@nube/starter-ui-kit`'s sidebar primitive from stage 6 — this stage didn't have to touch them.
- The chat bubble width change was made upstream (per F2) rather than overridden per-app; that's intentional and affects all consumers of `@nube/starter-ui-chat`.
- The Card primitive itself still defaults to `rounded-4xl`/`shadow-md`; the F6 look is enforced via call-site classes plus twMerge. If a future change adds a fresh `<Card>` without those classes it will look off — convention only, not a primitive change.
- `spec.color` from the node registry is still passed to inline `style` when present. That's data flowing through, not a hex authored in TSX — interpreting F6's "no raw hex" rule as a TSX-authoring rule, not a runtime-style rule.
- Pre-existing typecheck failure in `starter-extensions/examples/notes` is unrelated to flow-agent and was already present.

## Open questions

- (none)
