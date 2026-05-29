## Done

- Reviewed the stage 1–3 diff (Issues / Process / Metrics) against Layer-1 invariants R1, R2, R4/R5 and wire-format stability.
- Confirmed `starter-ext-metrics` is a true leaf (deps: `starter-ext-spi` + `dashmap` only); supervisor does not depend on it; arrows are adapters → metrics ← supervisor.
- Confirmed new endpoints ride the existing axum admin router (`Role::Admin`) with thin handlers; no new transport, no domain logic in transport.
- Confirmed no rubix/warehouse/skill leakage into starter; metrics wiring is opt-in (`Option`, `None` default).
- Confirmed wire formats untouched: additive types only, stable `ext.issue.*` codes, no English on the wire; existing serde structs unchanged.
- GATE VERDICT: PASS.

## Next

- Operator sign-off on the CleanupProvider shape + namespace guarantees below, then start Stage 4 (Cleanup) in a fresh session: `CleanupProvider` trait + `CleanupItem` in `starter-ext-server/src/cleanup.rs`, built-in `EnablementRow`/`UiCache`/`I18nCache` providers, `DELETE /extensions/{id}?purge=true`, `GET /extensions/{id}/cleanup` dry-run, `restart_required` on the list projection.
- Stage 4 discipline: write the idempotency test FIRST; every purge path scoped to the extension's own namespace; never `404` on an already-clean id.

## What you need to know

- This is a REVIEW gate, not a code stage — no new commit was created; stages 1–3 are already committed and pushed on `codeless/comprehensive-extension-management`.
- CleanupProvider trait (as designed, §4): `async fn discover(&self, id: &ExtensionId, m: &Manifest) -> Vec<CleanupItem>` and `async fn purge(&self, id: &ExtensionId, items: &[CleanupItem]) -> Result<(), CleanupError>`; `CleanupItem { kind, label, bytes }`.
- CleanupKinds + namespace bounds: WarehouseTable → only `com_<id>__*` tables + their continuous aggregates (rubix provider); EnablementRow → only the extension's own row (`extension_id = $1`); UiCache/I18nCache → only the extension's own path-prefix/ETag cache keys (the literal "sidebar" cleanup); Skill → only skills contributed by that bundle (rubix `SkillRegistry::remove`); Subscription → the extension's own subscriptions.
- Provider split: EnablementRow/UiCache/I18nCache are built-in to `starter-ext-server`; WarehouseCleanupProvider + SkillCleanupProvider live in rubix-agent (only place warehouse/skill knowledge enters).
- Dry-run-first contract: `GET /extensions/{id}/cleanup` runs only `discover` and returns the `Vec<CleanupItem>` (with best-effort bytes) so the operator previews exactly what `purge` would drop. Worked example for an installed `com_rubix_geo`: `[WarehouseTable "com_rubix_geo__pins", EnablementRow "rubix_geo", UiCache "<prefix>/rubix_geo/*", I18nCache "rubix_geo", Skill "<contributed skill ids>"]`.
- Idempotency: `purge` is a no-op `200 cleanup.succeeded` returning the items actually removed (possibly empty) — never `404`; `purge` DELETES the enablement row (not a `disabled` ghost); plain uninstall without `?purge` keeps today's `disabled` behaviour. Every purge step logs `target: "starter_ext_server::cleanup"` with the caller principal.

## Open questions

- (none) — scope is locked; the four design "Locked decisions" resolve every question.
