## Done

- Rewrote the "Why this exists" / "Today (Phase 2-5)" block in DOCS/flow/scope/hot-reload.md as present-tense, citing manager.rs:308, classifier.rs:109/33, active.rs:52/59/96, manager.rs:446/928, resolver.rs:158.
- Rewrote the same block in DOCS/flow/scope/settings.md as present-tense, citing settings.rs:48/58/111, node.rs:69/88, manager.rs:308/338/355, resolver.rs:158/193/40.
- Committed as 6d2df96 with the stage-title prefix.

## Next

- Begin stage 4 of 16: continue the rubix-flow-live-tick-demo job per .codeless/jobs/rubix-flow-live-tick-demo/WORKFLOW.md — the next phase after A+B.3 in the SCOPE (likely SSE-route / always-on-mounter wiring on the rubix side per the job goal).

## What you need to know

- The classifier's actual arm names are EditKind::{Initial, Unchanged, SettingsOnly, Structural, Mixed} — I noted these correspond to the SCOPE's "Initial/NoOp/Settings/Topology/Both" wording in the prose so future readers can map either name.
- lint-doc-refs.sh only scans rubix/crates/*.rs; the pre-existing 4 forbidden refs there are unrelated to this doc-only stage. DOCS/ markdown is not scanned.
- settings.rs does NOT ship SettingsField / SettingsKind enums (despite the stage prompt wording); it ships SettingsError + EMPTY_SCHEMA + default_validate. The rewrite reflects what exists.

## Open questions

- (none)
