## Done

- Committed stage 1 (PR 3) as c6f457d on codeless/rubix-thin-slice-v2.
- Added rubix-agent/src/lib.rs barrel so tests reach boot::mcp; main.rs now imports via rubix_agent::{boot, health, registry}.
- rubix-agent/src/boot/mcp.rs builds a fresh FlowRegistry, registers a programmatically-constructed FlowBody for `com.rubix.scheduled-system-check` with a non-reserved `com.rubix.diag-render` NodeBehavior, and wraps it as a FlowAsTool via the one-line `FlowAsTool::from_registry(&registry, &flow_id, &rev, engine).await?` contract. Mounts starter-mcp's `mcp_router` for the binary.
- Seed adapter reads `starter_mcp::current_locale()` (no manual LanguageTag threading; no Accept-Language parsing in rubix); maps en-US → America/New_York+MdySlash, es-AR → America/Argentina/Buenos_Aires+DmySlash, snapshots ResolvedPreferences onto the seed slot.
- tests/mcp_disk_test.rs uses `starter_mcp::testing::pair` (U2 InMemoryTransport), drives initialize → tools/list → tools/call twice; asserts EN "Disk is nearly full" + 01/15/2024 + 07:00 and ES "El disco está casi lleno" + 15/01/2024 + 09:00, plus catalogue presence under `com.rubix.scheduled-system-check`. Both pass.
- docs/design/i18n-prefs/README.md replaces "Accept-Language initial handshake header" with the U1 mechanism (`_meta.acceptLanguage` → task-local → `current_locale()`) and adds an es-AR worked example.
- `./rubix/scripts/lint-doc-refs.sh` clean; `cargo test -p rubix-agent --test mcp_disk_test` passes both locale cases.

## Next

- Stage 2: REVIEW gate 1 of 3 for PR 3.
- Stage 3: PR 4 ClickHouse history + insights rule + alert via `starter-store-clickhouse::MigrationRunner` (on master), per-row `tenant_id`, hardcoded `disk_used > 90` Rust rule.

## What you need to know

- The bundled YAML at rubix/crates/rubix-flows/flows/scheduled-system-check.yaml predates the typed FlowBody projection (uses rubix-native `kind: ai-agent`, `config:`, `trigger: explicit`). PR 3 sidesteps it by building the FlowBody programmatically with a one-node renderer kind. Later phases that wire the real ai-agent body should replace this stub.
- The diag-render node id is `com.rubix.render` and the kind id is `com.rubix.diag-render` — both reverse-DNS, both outside the reserved `starter.flow.*` prefix so `NodeKindRegistry::register` accepts the kind.
- Timestamp fixture in the seed adapter is hard-coded to 2024-01-15 12:00:00 UTC so the test assertions are deterministic across timezones.
- The MCP HTTP router is built in `build_mcp_surface()` but `main.rs` currently keeps it in `_mcp_router` (not yet merged into `health::serve`). That's a future-phase wiring; the test path uses `starter_mcp::testing::pair` and doesn't need HTTP.
- rubix-agent now has both `[lib]` (auto from src/lib.rs) and two `[[bin]]` entries; cargo accepts both with the same name.

## Open questions

- (none) — stage's pre-answered T1–T5/Q6 + MCP-locale task-local decision were honoured; no scope was re-litigated.
