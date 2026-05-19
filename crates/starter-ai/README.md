# starter-ai

`AiRunner` impls for five providers. Clean lift from
`codeless-workspace/ai-runner`; this crate is the source of truth
going forward.

## Providers (all default-off)

- `provider-claude` — Claude via the `claude-wrapper` CLI (pinned
  `=0.5.1` for stream-json stability).
- `provider-codex` — OpenAI Codex CLI wrapper.
- `provider-copilot` — GitHub Copilot CLI wrapper.
- `provider-anthropic` — REST via `anthropic-ai-sdk`.
- `provider-openai` — REST via `async-openai`.

Enable only what the consumer ships; the registry's `with_defaults()`
populates each provider behind `#[cfg(feature = "provider-*")]`.

## Usage

```rust
use starter_ai::{cancel::TokenCancel, registry::Registry, secret::api_key_for};
use starter_spi::ai::{AiRunner, Provider, RunnerInput};

let registry = Registry::with_defaults();
let runner   = registry.get(&Provider::OpenAi).expect("provider feature on");

let api_key  = api_key_for(&secrets, &Provider::OpenAi);
let (tx, mut rx) = tokio::sync::mpsc::channel(32);
let cancel = TokenCancel::new();

let input = RunnerInput::Rest(/* ... */);
let result = runner.run(input, tx, &cancel).await;
while let Some(event) = rx.recv().await { /* stream */ }
```

## Secret integration

`api_key_for(secrets, provider)` checks
`SecretStore::get("ai:<provider>:api_key")` first; falls back to env
vars (`ANTHROPIC_API_KEY`, `OPENAI_API_KEY`).

## Cancellation

`TokenCancel` wraps `tokio_util::sync::CancellationToken` and
implements `starter_spi::ai::Cancel`. CLI runners propagate via
`kill_on_drop(true)`; REST runners select on `cancel.cancelled()`.
