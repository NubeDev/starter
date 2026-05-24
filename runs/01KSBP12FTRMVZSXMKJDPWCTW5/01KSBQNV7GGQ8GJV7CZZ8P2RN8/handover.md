## Done

- Audited Stage 2 (block A) wiring against all nine gate criteria
- Confirmed MCP/auth/gated-tools routers merged, env::var discharged from main.rs, changelog wraps only tools_router (single write per call), no direct clickhouse dep, doc-refs lint clean, design doc rewritten in present tense

## Next

- FAIL sentinel below halts the job; next ramp must add a precedence unit test in rubix/crates/rubix-agent/src/boot/config.rs that materially exercises file < env (and ideally flags) overrides — e.g., write a temp agent.toml with bind=A, set RUBIX_BIND=B via env, call AgentConfig::load(), assert cfg.bind == "B"; repeat asserting an env-unset case falls through to file

## What you need to know

- All other Layer-1 invariants hold: R1 (no rubix → clickhouse direct dep), R2 (single axum Router composed at main.rs:89-126), R4/R5 trust boundary (gate wraps audited tools router only — auth router stays outside the authz layer, healthz + MCP stay outside auth as intended)
- The `else` branch at main.rs:120-126 mounts an ungated tools router when DATABASE_URL is unset; this is documented as a laptop-dev escape hatch — acceptable for thin slice but worth flagging if production hardening surfaces later
- Two cosmetic notes (not gate-blockers): boot/clickhouse.rs and boot/migrations.rs still read RUBIX_CH_URL / RUBIX_DSN directly instead of going through the loaded AgentConfig; criterion only checked main.rs so this passes literally, but the layered loader is partly bypassed

## Open questions

- (none)
