## Done

- Created `crates/starter-tool-gmail` (Cargo.toml, README, src/{lib,config,error,message,metrics,send}.rs, tests/send.rs)
- `GmailSendTool` implements `Tool` for `users.messages.send`: builds RFC 5322, base64url-encodes into `{raw: …}`, POSTs with `Authorization: Bearer …`
- `GmailConfig { oauth_access_token: SecretString, user_id, base_url }` with `default_user_id()="me"` and `default_base_url()="https://gmail.googleapis.com"`
- Metrics: `starter_tool_gmail_send_duration_seconds` (histogram) + `starter_tool_gmail_send_errors_total{kind}` registered on consumer's `prometheus::Registry` (R7)
- 401/403 → `GmailError::Auth` → `SpiError::Unauthenticated`; 5xx → `HttpStatus`; 2xx without `id` → `MissingId`; build failures → `Invalid` via `Build` variant
- Message builder (text, html, multipart/alternative, encoded-word display names, no Bcc-in-headers) lifted verbatim (renamed) from `codeless-tools::email::message`
- Wire shape lifted from `codeless-tools::email::gmail::GmailMailer`
- wiremock integration tests: happy path, 401→Unauthenticated, 503→HttpStatus, missing-id, bad-input, missing-recipients, latency histogram observation (7 tests) + 6 unit tests for the MIME builder — all pass
- Added crate to workspace members + `[workspace.dependencies]` in root `Cargo.toml`
- `cargo build / test / clippy -p starter-tool-gmail` all clean
- Committed as `Phase 4 — starter-tool-gmail send-only: …` (a46e719)

## Next

- Stage 9 of 9 — next session picks up the final phase per the job workflow (see DOCS/tools/scope/)

## What you need to know

- v0.1 is send-only; no `starter-service-gmail` exists (inbound deferred per SCOPE one-line summary on Gmail)
- Token acquisition is the consumer's concern — `starter-auth-oauth` (Phase 6) is reserved but not a dep. The crate accepts a bearer token and uses it; refresh on 401 is the consumer's job and is enabled by the `Unauthenticated` mapping
- The crate stays consistent with `starter-tool-slack` / `starter-tool-telegram`: same dep set (`starter-spi` + `starter-observability` + reqwest/serde/tokio/tracing/prometheus/thiserror/async-trait), plus workspace `base64 = "0.22"` for the URL_SAFE_NO_PAD encoding
- Two-worktree note: an earlier `mkdir` and several `Write` calls landed in a sibling worktree (`job-01KRZA7KWM0JQ1RBH70FV0FM9P`) by absolute-path accident; everything was copied back into the active worktree before committing, so the commit contains the full file set. The sibling worktree still holds a stray copy under `crates/starter-tool-gmail/` but is untracked and harmless

## Open questions

- (none)
