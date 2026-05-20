## Done

- Added `crates/starter-i18n/src/diagnostics.rs` implementing the Phase 5 rewriter: `DiagnosticBody` marker, `DiagnosticsLayer`/`Service`, JSON envelope rewrite at the two documented paths (`diagnostic` object + `diagnostics` array), SSE/chunked bypass, missing-translation pass-through, additive `message` field with `{name}` placeholder interpolation.
- Wired `pub mod diagnostics;` under `#[cfg(feature = "diagnostics")]` in `crates/starter-i18n/src/lib.rs`.
- Expanded the `diagnostics` cargo feature in `crates/starter-i18n/Cargo.toml` to pull axum/http/tower/futures/async-trait/http-body-util/bytes; added the matching optional deps.
- Integration tests cover all six SCOPE cases plus pure-function tests for `interpolate`, `is_json`, `is_streaming`. `cargo test -p starter-i18n --features diagnostics,routes` → 68 unit + 6 integration green. Default build (no feature) also compiles clean.
- Committed as `stage 20 — Phase 5 scope-limited diagnostics rewriter in starter-i18n.`

## Next

- Stage 21 of 22 picks up from here per the job plan.

## What you need to know

- Rewriter is opt-in per-response via `response.extensions_mut().insert(DiagnosticBody::new())`; absence is a no-op (verified).
- Bundle is supplied at layer-build time (`diagnostics_layer(Arc<MessageBundle>)`); the chosen language is read from `LocaleCtx` (set upstream by `accept_language_layer`). Without that upstream the rewriter falls back to `bundle.fallback()`.
- Rewritten envelopes ADD a `message` field; `code` and `params` are preserved verbatim so clients ignoring `Content-Language` behave identically.
- Streaming bypass keys off `Content-Type: text/event-stream` and `Transfer-Encoding: chunked` — body is forwarded byte-for-byte, no JSON parse attempted.
- Interpolation is named-placeholder only (`{name}`), not full ICU MessageFormat — react-intl handles select/plural client-side, matching the Phase 4 wiring.

## Open questions

- (none)
