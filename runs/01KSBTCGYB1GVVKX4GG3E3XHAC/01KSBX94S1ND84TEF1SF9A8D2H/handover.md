## Done

- Stage 1 (Block A YAML loader) was already committed at 542b918 on branch codeless/rubix-agent-runtime before this session started. The commit adds rubix-flows/src/load.rs with load_all() + tests, rewrites boot/mcp.rs to delete the hand-rolled flow + fake diag-render kind, deletes tests/mcp_disk_test.rs, and rewrites docs/design/flows/README.md.

## Next

- Stage 2: REVIEW gate for Block A, then Block B (starter-ai-agent + starter-flow-node-loop crates upstream).

## What you need to know

- Working tree is clean (only an untracked runs/ directory from this session). No new commit was needed.
- The committed loader prefixes short YAML node ids with `com.rubix.` to satisfy NodeId reverse-DNS, and maps `kind: ai-agent` → registered KindId `com.rubix.ai-agent`. Block B's KIND_ID constant should match this string or Stage 3 will need to adjust the mapping.
- An AiAgentStubNode is bound under `com.rubix.ai-agent` so FlowRegistry::register resolves the kind; invocation returns a "not wired yet" NodeError that Block C replaces.

## Open questions

- (none)
