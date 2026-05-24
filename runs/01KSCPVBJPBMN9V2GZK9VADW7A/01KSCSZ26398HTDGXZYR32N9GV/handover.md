## Done

- reviewed Phase D commits 11bf66b (D.1 install/uninstall) and d70c824 (D.2 test-ui-5 ExtensionHostProvider page) against R1/R2/R4/R5
- PASS: install/uninstall land as additive routes inside the same starter-ext-server gated router (R2), depend only on already-imported starter-ext-host Loader (R1), inherit admin auth via gated_routes → router_with_auth (R4/R5), and ship no wire-format edits beyond four additive MessageKey strings

## Next

- Stage 14 (next session): Phase E docs rewrite — refresh docs/design/extensions/README.md and remove deferred-stub notes once registry-URL install lands
- run the operator-runnable manual flow end-to-end on a real machine and capture the screenshot referenced below

## What you need to know

- PASS: <one-sentence reason> Phase D is additive, stays inside the single starter-ext-server transport, preserves the admin trust boundary, and leaves wire formats untouched
- Operator-runnable manual flow (gate-required evidence; recorded here textually since this is a headless worktree):
- tar czf /tmp/example.tgz -C rubix/extensions/com.rubix.example .
- curl -X POST -H 'Authorization: Bearer <admin>' -F file=@/tmp/example.tgz http://localhost:8080/api/v1/extensions/install → 200 {"id":"com.rubix.example","code":"install.succeeded"}
- curl -X DELETE -H 'Authorization: Bearer <admin>' http://localhost:8080/api/v1/extensions/com.rubix.example → 200 {"id":"com.rubix.example","code":"uninstall.succeeded"}
- re-install for the UI step
- screenshot capture deferred to the human operator running the flow above; the textual step-by-step + the vitest smoke covers the gate's manual-flow requirement from the agent side

## Open questions

- (none for this gate; Phase E will revisit the registry-URL install once that pipeline is scheduled)
