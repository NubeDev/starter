## Done

- Resolved all four open questions in DOCS/extensions/scope/SCOPE.md by promoting them into the "Decisions made" section with explicit revisit triggers: (1) bundle on-disk convention defaults to `$XDG_DATA_HOME/<binary>/extensions/<id>/` overridable via starter-config; (2) admin endpoints ship behind `Role::Admin` with `Role::ExtensionManage` deferred; (3) enable/disable state persists in a DB row keyed by extension id; (4) JSON-RPC wire-schema versioning via `host_capabilities` in the init handshake, deferred to v0.2's first new host method.
- Added the two post-R13 follow-ups as decisions: streaming convention (`stream.event` / `stream.end` / `stream.error` / `stream.cancel` notifications tagged with `stream_id`) lives in `starter-ext-spi` alongside `JsonRpcEnvelope`; per-entry `require_role` / `require_scope` in the manifest, enforced by the adapter never by the extension.
- Replaced the "Open questions" body with an explicit "(none)" sentinel that keeps the section as the landing zone for future questions.
- Committed as `a619f52` with the stage title prefix.

## Next

- Stage 2 of 16 will be picked up by a fresh session.

## What you need to know

- SCOPE.md is at `DOCS/extensions/scope/SCOPE.md` (the repo also has a top-level `SCOPE.md` from the starter workspace — do not edit that one for extensions decisions).
- The SCOPE document does not yet contain an R13 rule by number; the job description's "post-R13 follow-ups" was treated as guidance about which decisions to record, not a reference to existing prose. The follow-ups were filed under a new "Post-R13 follow-ups" subsection inside Decisions.
- Every new decision carries an explicit "Revisit trigger" sentence so future sessions know when to reopen it.

## Open questions

- (none)
