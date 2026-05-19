# starter-tool-gmail — outbound Gmail messages as a `Tool`

A sibling provider crate per
[`DOCS/tools/scope/SCOPE.md`](../../DOCS/tools/scope/SCOPE.md): one
integration per crate, selected by Cargo dependency and constructed in
the consumer's `main.rs`. Depending on this crate compiles a single
`Tool` impl wrapping the Gmail REST
[`users.messages.send`](https://developers.google.com/gmail/api/reference/rest/v1/users.messages/send)
endpoint. Nothing in `starter-*` reaches for this crate; the consumer
wires it.

**v0.1 is send-only.** Inbound Gmail (`users.watch` + Cloud Pub/Sub
or `history.list` long-poll) is explicitly deferred per the source
SCOPE one-line summary; there is no `starter-service-gmail` yet.

## What it extends

| Surface  | How it's extended                                                                | Where to look |
|----------|----------------------------------------------------------------------------------|---------------|
| MCP      | `GmailSendTool` registered into the consumer's `ToolRegistry` next to starter tools | [src/send.rs](src/send.rs) |
| Config   | `GmailConfig` carries an already-resolved `SecretString` access token — no env reads here (R5) | [src/config.rs](src/config.rs) |
| Metrics  | `starter_tool_gmail_send_{duration_seconds,errors_total}` on the consumer's `prometheus::Registry` (R7) | [src/metrics.rs](src/metrics.rs) |
| Errors   | `GmailError` distinguishes `Transport` / `Auth` / `HttpStatus` / `MissingId` / `Build`; maps into `starter_spi::Error` | [src/error.rs](src/error.rs) |
| MIME     | `GmailMessage` / `GmailMailbox` render RFC 5322 (text, html, multipart/alternative) lifted from `codeless-tools::email::message` | [src/message.rs](src/message.rs) |
| Tests    | Integration tests against `wiremock` covering happy path + 401 + 5xx + missing-id + build/bad-input | [tests/send.rs](tests/send.rs) |

## Wire it into `main.rs`

```rust
use std::sync::Arc;
use prometheus::Registry;
use starter_spi::SecretString;
use starter_tool_gmail::{GmailConfig, GmailSendTool};

# fn ex(registry: Arc<Registry>, tool_registry: &mut starter_mcp::ToolRegistry) -> Result<(), Box<dyn std::error::Error>> {
let cfg = GmailConfig {
    // Resolved by the consumer — interactive OAuth, refresh-token
    // exchange, GSA, or whatever flow fits. starter-auth-oauth is
    // reserved at Phase 6 of the plan but is not a hard dep.
    oauth_access_token: SecretString::from(std::env::var("GMAIL_ACCESS_TOKEN")?),
    user_id:            GmailConfig::default_user_id(),  // "me"
    base_url:           GmailConfig::default_base_url(),
};
let tool = GmailSendTool::new(cfg, &registry)?;
tool_registry.register(tool);
# Ok(())
# }
```

The consumer flips the integration off by *not* calling `register(...)`
— the R3 `if cfg.gmail.enabled { … }` pattern, no special config knob
inside this crate.

## What's NOT here

- **No token acquisition.** Per the SCOPE one-line summary for Gmail
  the acquisition flow is the consumer's concern —
  `starter-auth-oauth` (Phase 6) when it lands, or a custom flow until
  then. This crate takes a bearer token and uses it.
- **No token refresh on 401.** A 401/403 surfaces as
  `starter_spi::Error::Unauthenticated` (via `GmailError::Auth`); a
  wrapping layer can match on that variant and trigger a refresh.
- **No inbound listener.** Inbound Gmail is deferred; there is no
  `starter-service-gmail` paired with this tool.
- **No attachments / inline images.** v0.1 supports a single text
  body, a single HTML body, or both as `multipart/alternative` —
  lifted verbatim from `codeless-tools::email::message`.
- **No env / file reads.** Per R5 the consumer resolves the access
  token and hands it in via `GmailConfig`.
- **No `secrecy` direct dep.** The crate imports `SecretString` /
  `ExposeSecret` from `starter_spi::*` — R5.
- **No `google-*` SDK.** Plain `reqwest` + `serde` + `base64`; R8.

## Test plan

```bash
cargo test -p starter-tool-gmail
```

The integration suite covers, against a `wiremock` server:

1. `200 { "id": "…" }` — happy path; asserts the `Authorization:
   Bearer …` header and `application/json` content-type are sent.
2. `401` with a Google-style `{"error": …}` body — maps to
   `starter_spi::Error::Unauthenticated`, bumps `kind="auth"`.
3. `503` — surfaces `GmailError::HttpStatus { status: 503, … }`,
   bumps `kind="http_status"`.
4. `200 {}` (no `id`) — surfaces `GmailError::MissingId`, bumps
   `kind="missing_id"`.
5. Bad JSON input (`subject` missing) → `Invalid`, bumps
   `kind="bad_input"`.
6. Message with no recipients → `Invalid` via `GmailError::Build`,
   bumps `kind="message_build"`.
7. Latency histogram observes exactly one sample on the happy path
   (R7 dashboard regression guard).
8. The crate's own unit tests (under `cargo test -p
   starter-tool-gmail --lib`) cover RFC 5322 rendering — text-only,
   html-only, multipart/alternative, Bcc-header omission, encoded-word
   display names.
