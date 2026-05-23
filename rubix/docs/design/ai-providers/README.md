# AI PROVIDERS — how `rubix-agent` selects an `AiRunner`

## The model

`rubix-agent` runs every flow through starter's `ai-agent` node
kind. That node holds one `Arc<dyn AiRunner>` (see
`starter_spi::ai::AiRunner`). Rubix does not build a second LLM
seam — it picks one runner at boot and hands it to the flow
engine. See [docs/design/agent/](../agent/README.md) for where the
runner plugs into the boot composition.

## Selection at boot

The provider is chosen by a single config knob (`ai.provider`,
loaded via `starter-config` once that wiring lands — until then,
the binary reads `RUBIX_AI_PROVIDER`, defaulting to `claude`):

| `ai.provider` | Resolved runner |
|---|---|
| `claude` (default) | Claude Code CLI via `starter-ai`'s `provider-claude` feature |
| `codex` | OpenAI Codex CLI via `provider-codex` |
| `copilot` | GitHub Copilot CLI via `provider-copilot` |
| `anthropic` | Anthropic REST via `provider-anthropic` |
| `openai` | OpenAI REST via `provider-openai` |

Resolution is one call:

```text
starter_ai::registry::Registry::with_defaults()
    .get(&Provider::from_config(&cfg.ai.provider))
    .ok_or_else(|| Error::ProviderNotCompiled)?
```

If the requested provider's cargo feature was not compiled into
the binary, `get()` returns `None` and the agent fails fast at
boot. There is no runtime fallback to a different provider — a
silent fallback would hide an operator misconfiguration.

## What rubix does NOT do

- **No multi-provider fan-out.** One provider per binary, chosen
  at boot. Flows do not pick per-turn.
- **No per-skill override.** Skills steer behaviour through their
  prompt, not through provider choice.
- **No auto-fallback chain.** If Claude CLI is not installed and
  `ai.provider = claude`, the binary refuses to start. The
  operator must change the config explicitly.
- **No auth handling.** CLI providers manage their own auth
  (`claude login`, `gh auth`); REST providers read the standard
  env var (`ANTHROPIC_API_KEY`, `OPENAI_API_KEY`) or take a key
  from `starter-secrets-*`.

## Headless appliance guarantee

Per starter-ai's per-provider feature flags, a deployment that
only needs Anthropic REST builds with
`features = ["provider-anthropic"]`. The Claude/Codex/Copilot CLI
wrappers and the OpenAI SDK do not enter the dependency graph.
This matters for rubix containerised deployments where dragging
in five client SDKs would bloat the image needlessly.

## The `claude-wrapper` pin

`starter-ai` pins `claude-wrapper = "=0.5.1"` because that crate
parses the `claude` binary's stream-json output, which Anthropic
does not promise to keep stable. Rubix inherits the pin — when
`starter-ai` bumps it, rubix consumes the bump through a normal
`cargo update`. Rubix never pins `claude-wrapper` directly.

## Failure modes

| Failure | Surface |
|---|---|
| Provider feature not compiled | `rubix-agent` exits non-zero at boot with the requested provider name |
| CLI binary missing on PATH | First tool dispatch fails with `AiRunner::Error::TransportUnavailable`; logged structurally |
| REST API key missing | Same — surfaces on first turn, not at boot (auth check is per-call) |
| Wrong API key | Surfaces on first turn as a 401-equivalent in the SSE error event |

Boot-time failures stop the binary. Per-call failures surface
through the SSE event taxonomy (R13) as `agent.turn.error`
events; the flow may retry or abort per the skill's policy.
