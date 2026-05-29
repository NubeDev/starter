## Done

- Registered rubix cleanup providers on ExtensionAdminBuilder: WarehouseCleanupProvider (namespace-scoped drop of com_<id>__* tables + continuous aggregates) and SkillCleanupProvider (SkillRegistry::remove); built-in starter providers auto-register
- build_extension_admin now takes Vec<Arc<dyn CleanupProvider>>; services.rs connects warehouse before the bundle and wires the warehouse reclaimer with the live pool
- admin/extensions.rs maps IssueCode -> rubix.extension.issue.* MessageKeys (total match); catalogue keys added to rubix-spi en/es and synced to frontend
- Added SkillRegistry::remove(&SkillId) -> bool in starter-skills (+ test)
- Frontend hooks useExtensionIssues/Process/Metrics/CleanupPreview/Purge; admin tabs (Issues/Process/Metrics) + Uninstall dry-run dialog in extensions.$extId.$.tsx
- rust build + clippy + fmt green; frontend tsc -b + vite build green; new Rust unit tests pass
- Committed as f41d51d

## Next

- (none) — this was the final stage (P5/stage 6 of 6)

## What you need to know

- rubix-agent has no live SkillRegistry today (extensions_flow.rs notes it "will land in the starter-skills track"), so SkillCleanupProvider is implemented + exported + unit-tested but not yet wired at boot; WarehouseCleanupProvider IS wired whenever a warehouse is configured. When the live registry lands, push a SkillCleanupProvider::new(skills, registry) into the cleanup_providers Vec in services.rs
- The frontend admin tabs talk directly to starter-ext-server endpoints (/api/v1/extensions/{id}/{issues,process,metrics,cleanup} + DELETE ?purge=true) via fetchJson, matching the existing useExtensionsList pattern — there is no separate rubix detail-envelope endpoint
- Pre-existing, NOT introduced here: dead_code warning on BootedServices.bundle/mcp_pool (destructured to _ in compose.rs), and 12 check:i18n flows.* es-drift violations. Frontend node_modules were absent in the worktree — `pnpm install --offline` restores them from cache
- continuous-aggregate discovery is best-effort: if timescaledb_information is absent the query errors and is treated as "no caggs". Could not exercise the warehouse provider against a live DB here (unit tests cover the skill provider + namespace prefix only)

## Open questions

- (none) — scope was locked
