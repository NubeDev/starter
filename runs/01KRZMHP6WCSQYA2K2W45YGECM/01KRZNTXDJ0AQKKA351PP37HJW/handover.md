## Done

- Added `crates/starter-tool-slack` sibling crate: `SlackConfig` (bot_token/signing_secret SecretString + base_url), `SlackPostTool` implementing `Tool::invoke` for `chat.postMessage` (text/channel/optional blocks/optional thread_ts), latency histogram + per-kind error counter registered on the consumer-supplied `prometheus::Registry`, and a README mirroring `examples/notes/README.md`'s "how it's extended" table.
- HTTP request shape lifted from `codeless/crates/codeless-slack/src/web_api.rs`; architecture (Tool/Registry, error mapping into `starter_spi::Error`, metric surface) is starter's.
- `tests/post.rs` covers success, 429 (with `Retry-After`), 5xx, and `invalid_auth` (mapped to `SpiError::Unauthenticated`) against `wiremock`, plus a latency-histogram smoke check. All 5 tests pass; `cargo build --workspace` is clean.
- Added `starter-tool-slack` to workspace members and `[workspace.dependencies]`.
- Added `pub use secrecy::ExposeSecret;` to `starter-spi/src/lib.rs` next to the existing `SecretString` re-export — provider crates need it to read the plaintext token without naming `secrecy` directly (R5). Documented as a small additive fix-forward to stage 2.

## Next

- Stage 6 (next session): per the source SCOPE phasing, this is `starter-tool-telegram` (outbound) — same shape as this crate, mining `codeless/crates/codeless-telegram` for the `sendMessage` call.

## What you need to know

- `SlackConfig` is NOT `#[non_exhaustive]` — the test suite needs struct-literal construction, and the consumer is the constructor anyway. Adding a field is a minor-bump-breaking change documented in the crate-level rustdoc; a future builder API can be layered on if the field count grows.
- The 429 path returns `SlackError::RateLimited { retry_after_secs }` (parsed from the header) but at the SPI boundary it lands as `SpiError::Internal { source: SlackError::RateLimited }` — the test walks the source chain. `invalid_auth` and `not_authed` are the only Slack error labels mapped to `SpiError::Unauthenticated`; everything else is `Internal`.
- Tool name is the constant `starter_tool_slack::post::TOOL_NAME = "slack.post_message"`, used in `ToolDefinition::name` and as the `tool.name` tracing field. Don't rename — dashboards key off it.
- Latency buckets in `metrics.rs` stretch 5ms–30s (wider than starter-server's request histogram) so Slack-incident tail latency is visible without re-bucketing.
- Re-export change in `starter-spi/src/lib.rs` is additive; no consumer breaks. The `starter-spi-deps.baseline.txt` did NOT need updating — `secrecy` was already a direct dep.

## Open questions

- SCOPE R7 says "the prometheus::Registry the consumer passes in via McpHttpOptions (R7)" but `McpHttpOptions` does not currently carry a `Registry` field. This stage worked around that by having `SlackPostTool::new` take `&Registry` directly — the consumer threads the same registry into both. A later stage may want to extend `McpHttpOptions` to actually hold the registry so MCP-dispatched tools can have it injected at registration time.
