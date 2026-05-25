## Done

- added rubix/crates/rubix-flows/flows/tick-counter.yaml: schedule trigger (`*/5 * * * * *`) → counter (step=1, initial=0, reset_on_redeploy=false) → log (info, `tick {value}`); links `tick.fire → count.in` and `count.out → emit.value`
- extended rubix/crates/rubix-flows/tests/load_test.rs with `bundled_tick_counter_parses_with_three_nodes_and_two_edges` asserting parse + convert produces 3 nodes (in order) and 2 edges with the expected slot strings
- relaxed `every_bundled_yaml_parses_with_ai_agent_root` and `load_all_converts_every_bundled_flow` to skip the ai-agent-root check for entries in a new `NON_AI_AGENT_FLOW_IDS` allowlist (currently just `com.rubix.tick-counter`); added the flow to `EXPECTED_FLOW_IDS` so load_all coverage stays exhaustive
- `cargo test -p rubix-flows`: 4 passed
- committed as `phase D — bundled com.rubix.tick-counter flow YAML` (d7764b6)

## Next

- (none) — fresh session picks up the next stage

## What you need to know

- `include_dir`'s `BUNDLED.get_file()` paths are relative to the `flows/` root, so the lookup is `tick-counter.yaml`, not `flows/tick-counter.yaml`
- `convert()` passes non-`ai-agent` node kinds through verbatim, so `starter.flow.trigger.schedule|counter|log` survive into the `FlowBody`; node ids get the `com.rubix.` reverse-DNS prefix
- LinkDecl strings (`tick.fire`, `count.in`, …) are passed verbatim — they reference the *short* YAML ids, not the prefixed NodeIds; if the resolver expects fully-qualified slot refs that will surface downstream (no failure observed in this stage's tests)

## Open questions

- whether `FlowBody.links` should reference short ids (as the SCOPE YAML literally shows) or `com.rubix.<id>.<slot>` — convert() does not rewrite link strings; downstream resolution may need either a convert-time prefix pass or short-id support in the resolver
