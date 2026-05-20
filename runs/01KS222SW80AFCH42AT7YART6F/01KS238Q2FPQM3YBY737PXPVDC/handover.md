## Done

- Added `src/ai_runtime.rs`: owns `Arc<starter_ai::Registry>` from `Registry::with_defaults()`, exposes `list_providers()` and `run_agent(agent, prompt, history)` returning an SSE `Event` stream.
- Wired `POST /api/agents/{id}/run` in `rest.rs` returning `Sse` with 15s keepalive; emits `{type:"text",text}` / `{type:"tool-call",toolCall:{…}}` / `{type:"error",error}` and a terminal `[DONE]` sentinel matching `createSseAdapter`'s default parser.
- Added `GET /api/providers` returning `ProviderStatusDto[]` (Claude CLI session, ANTHROPIC_API_KEY, OPENAI_API_KEY) with `available` + `hint`.
- Provider resolution accepts `anthropic.claude` / `claude` → `Provider::Claude`, plus `anthropic`, `openai`, `codex`, `copilot`. Unknown / unavailable maps to 422.
- Claude CLI runs use `RunnerInput::Cli` with `PermissionMode::Bypass` and the agent's `model`; history is folded into `system_prompt`. REST providers (if/when their features are enabled) get the history list as-is.
- Replaced the `AgentChat.tsx` Phase-1 stub with `<Chat adapter={createSseAdapter({url})} />` keyed on agent id; pulls agent metadata via react-query for the title/empty state.
- Rebuilt `Settings.tsx` to render the live provider probe with a badge per row.
- Added `providers.list` to `frontend/src/lib/api.ts` + `ProviderStatus` type.
- `cargo build -p flow-agent` green; `pnpm --filter flow-agent-frontend typecheck` green.

## Next

- Stage 5 — agent-as-tool bridge (flows callable from the agent prompt).

## What you need to know

- The Claude runner is the only AI provider compiled in (`provider-claude` feature). `Registry::with_defaults()` lights up exactly the cargo-feature-enabled runners, so 422 surfaces if a user creates an Anthropic/OpenAI agent without enabling the feature — by design.
- `run_agent` spawns the runner on a Tokio task and returns the receiver stream; the SSE adapter on the frontend calls AbortController on unmount but the runner currently isn't cancelled (its `TokenCancel` lives inside the task). Fine for the smoke target; the cancel plumbing is a follow-up.
- Conversation history is react-query-only — refreshing the chat page resets it. Persistence is explicitly deferred per the stage brief.
- Workspace `pnpm -r typecheck` reports a pre-existing failure in `starter-extensions/examples/notes` (StarterClient.get/post), unrelated to this stage.

## Open questions

- (none)
