## Done

- Reviewed cross-cut design against canon (user-pref.md D-NP.6/.7/.8/.9/.10, SCOPE.md telemetry table, perf-budget rules)
- Verified the four locked decisions match shipped code: fallback floor=en + left-truncating BCP-47, channel="starter-prefs", six frozen telemetry event names, render-budget=1/consumer + ≤8KB gz bundle growth
- Confirmed Layer-1 invariants over stages 1–6 diff: R1 dep direction intact (notes → starter-prefs/i18n; spi unchanged), R2 single transport (i18n catalog endpoint is same axum surface as ui.rs), R4/R5 trust boundary (read-only singleton; client-side translation; safe_join_root unit-tested), wire format intact (manifest still v: 1; contributes.i18n additive per R13; requires accepts bare-string + typed via untagged enum)

## Next

- Stage 7 (workflow) implements the hardening pass against these locked decisions: locale fallback resolver + telemetry, BroadcastChannel("starter-prefs") wiring in PreferencesProvider, dev catalog watcher in Vite plugin, render-budget Vitest, aria-live announcer, three docs (DOCS/extensions/guides/i18n.md, DOCS/user/guides/prefs-in-extensions.md, hello README)

## What you need to know

- PASS: the four review items (fallback, channel name, telemetry names, perf thresholds) are consistent across user-pref.md, SCOPE.md, and the shipped code from stages 1–6; Layer-1 invariants (R1/R2/R4/R5 + wire format) all hold
- No code or doc edits in this stage; nothing to commit (REVIEW gate)
- Stage 6 handover already flags that BroadcastChannel multi-tab is not yet wired — that is the Stage-7 deliverable, not a gate failure
- Stage 7 must NOT rename any of the six telemetry event names or the "starter-prefs" channel — they are dashboard/contract keys frozen on merge per SCOPE.md:97-100

## Open questions

- (none)

PASS: cross-cut decisions (fallback floor=en/left-truncating BCP-47, channel="starter-prefs", six frozen telemetry event names, render=1/consumer + ≤8KB gz) are consistent across user-pref.md, SCOPE.md and the shipped Stages 1–6 code, and Layer-1 invariants (R1 dep direction, R2 single transport, R4/R5 trust boundary, manifest wire format) are intact.
