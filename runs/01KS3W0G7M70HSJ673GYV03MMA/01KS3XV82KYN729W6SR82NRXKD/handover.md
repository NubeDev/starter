## Done

- Added `crates/starter-skills/src/selector.rs` with `SelectorStrategy` trait + three impls: `LlmSkillSelector` (Haiku default, 2s hard timeout via builder, no retries, WARN+metric on every failure mode tagged `skill_selector_failed_total{reason}`), `KeywordSkillSelector` (deterministic, no LLM), `FirstSkillSelector` (test fixture).
- `SkillRegistry` now implements `starter_flow_spi::skill::SkillSelector`: filters quarantined via `self.list()` and dispatches to the configured strategy.
- Extended `SkillRegistryBuilder` with `with_default_selector`, `with_default_selector_arc`, `with_ai_runner`; default resolution at build = explicit > Llm > Keyword.
- `Cargo.toml`: promoted `tokio` to normal dep (`sync`, `time`, `rt`) and `starter-spi` to normal dep; dep graph unchanged.
- `tests/stage6_phase4_selectors.rs`: two normative smokes pass (`selection_is_frozen_per_run_against_the_real_registry`, `quarantined_skill_never_reaches_selector_strategy`), plus a bonus default-strategy smoke. `cargo test -p starter-skills` green (36 tests).
- Committed as `0bcf6a5` on `codeless/starter-skills`.

## Next

- Stage 7 picks up Phase 4b: ai-agent node on-mount `content_hash` verification (so a concurrent `reload()` can never mount drifted resource bytes). Wiring belongs in `crates/starter-flow-nodes/src/ai_agent.rs`, gated against the `SkillRegistry::get()` hash.

## What you need to know

- The `Engine::with_skill_selector(Arc::new(registry))` path requires no engine-side change; the registry's `SkillSelector` impl plugs straight in. No starter-flow test was added in this stage — the brief explicitly retargets the existing `stage5_skill_threading.rs` pattern *inside* starter-skills, which is what `tests/stage6_phase4_selectors.rs` does.
- `LlmSkillSelector` parses the model response as "first non-empty trimmed line == candidate id, or `none`". Anything else → `unknown_id` / `parse_error` and `None`. The metric is currently a `tracing::warn!` on the `skill_selector_failed_total` target with a `reason` field — a real Prometheus counter will plug into the same target name later.
- The frozen-selection smoke proves the registry's content-hash machinery makes engine-side pinning possible (captured `SkillSelection` is immutable, post-reload re-select sees H2). It does NOT exercise the engine's pin — that test already lives in `crates/starter-flow/tests/stage5_skill_threading.rs`.
- Tracing field names use bare identifiers (`outcome`, `reason`) rather than dotted (`skill.selector.outcome`) — the `tracing` macro grammar rejects dotted paths in event args. The dotted namespace is preserved via the event `target`.
- Pre-existing clippy warnings in `parser.rs` / etc are not from this stage.

## Open questions

- (none)
