//! Setup / Automation Builder wiring (Setup-Builder track).
//!
//! Composes the `starter-setup` run service over Postgres stores and the live
//! flow engine, then exposes the `/setup/*` REST router for `main` to mount
//! under the principal layer. This is the seam that takes the builder from
//! "green in-crate" to "runs on the live server":
//!
//! - **Stores** — `PgTemplateStore` (catalog) + `PgSetupRunStore` (run index),
//!   both on the metadata pool, behind the `setup` migration source applied in
//!   `bootstrap::migrate_all`.
//! - **Engine** — a `starter_flow::FlowRunner` configured with the §8b
//!   halt-on-node-failure policy (`SetupEngine::runner_config`), a durable
//!   Postgres `PgRunStore` SPI run store (the checkpoints resume replays and
//!   boot crash-recovery reads), and the shared `NodeKindRegistry` the extension
//!   boot populated with `ProcessNodeProxy`-bridged contributed nodes. The
//!   in-memory `GraphStore` is the intended composition: slot state is
//!   reconstructed from the durable checkpoint on resume / restart.
//! - **Authz** — `register_specs` registers the `setup.templates` / `setup.runs`
//!   resource kinds so the engine can evaluate the surface's rules.
//! - **Templates** — each enabled extension's `contributes.setup_templates[]`
//!   is imported into the catalog under the global (`tenant_id = None`) scope.
//!
//! Per DOCS §7 the team check (`allowed_teams`) is enforced inside the run
//! handler, not as an authz condition; that lives in `starter_setup` and needs
//! no host wiring here.

use std::sync::Arc;

use starter_ext_host::ExtensionRegistry;
use starter_flow::graph::InMemoryGraphStore;
use starter_flow::registry::NodeKindRegistry;
use starter_flow::run::{FlowRunner, InMemoryRunStore};
use starter_flow_spi::graph::GraphStore;
use starter_setup::service::{RunService, RunServiceConfig, SetupEngine};
use starter_store_postgres::flow::PgRunStore;
use starter_store_postgres::setup::{PgSetupRunStore, PgTemplateStore};
use starter_store_postgres::Pool;

/// Concrete run-service type behind the nexus `/setup/*` surface: Postgres
/// catalog + run index.
pub type NexusRunService = RunService<PgTemplateStore, PgSetupRunStore>;

/// Build the setup run service over the metadata pool and the shared flow
/// node-kind registry (already populated by the extension boot with each
/// enabled extension's `ProcessNodeProxy`-bridged contributed nodes).
///
/// The runner uses an in-memory `GraphStore` + in-memory engine `RunStore` for
/// live slot/propagation state, with the durable `PgRunStore` as the SPI run
/// store: that is where checkpoints persist, so resume-from-cursor (§8b) and
/// boot crash-recovery (§8a) survive a restart while hot-path slot writes stay
/// in memory.
pub fn build_service(pool: Pool, flow_node_kinds: Arc<NodeKindRegistry>) -> Arc<NexusRunService> {
    let graph: Arc<dyn GraphStore> = Arc::new(InMemoryGraphStore::new());
    let runner = Arc::new(
        FlowRunner::new(graph, Arc::new(InMemoryRunStore::new()))
            .with_config(SetupEngine::runner_config())
            .with_spi_run_store(Arc::new(PgRunStore::new(pool.clone()))),
    );
    let engine = SetupEngine::new(runner, flow_node_kinds);
    Arc::new(RunService::new(
        Arc::new(PgTemplateStore::new(pool.clone())),
        Arc::new(PgSetupRunStore::new(pool)),
        engine,
        RunServiceConfig::default(),
    ))
}

/// Register the setup resource specs (`setup.templates`, `setup.runs`) into the
/// authz registry. Call at boot before the engine evaluates rules.
pub fn register_authz(registry: &starter_authz::registry::StaticRegistry) {
    starter_setup::authz::register_specs(registry);
}

/// Import every enabled extension's `contributes.setup_templates[]` into the
/// catalog under the global scope, validating each against `flow_node_kinds`
/// (so a template referencing a node-kind no enabled extension provides is
/// rejected at boot rather than failing mid-run).
///
/// Best-effort per extension: a bundle whose template fails to read/parse/
/// validate is logged and skipped so one bad bundle never blocks boot. Returns
/// the total number of templates imported.
pub async fn import_extension_templates(
    registry: &ExtensionRegistry,
    service: &NexusRunService,
    flow_node_kinds: &Arc<NodeKindRegistry>,
) -> usize {
    let mut total = 0usize;
    for record in registry.iter_validated() {
        let Some(ext_id) = record.id.as_ref() else {
            continue;
        };
        let Some(manifest) = record.manifest.as_ref() else {
            continue;
        };
        if manifest.contributes.setup_templates.is_empty() {
            continue;
        }
        let contributions = starter_setup::extension::contributions_from_pairs(
            manifest
                .contributes
                .setup_templates
                .iter()
                .map(|t| (t.id.clone(), t.file.clone())),
        );
        match starter_setup::extension::import_bundled_templates(
            &record.bundle_dir,
            ext_id.as_str(),
            &contributions,
            service.templates().as_ref(),
            flow_node_kinds,
        )
        .await
        {
            Ok(ids) => {
                total += ids.len();
                tracing::info!(
                    target: "nexus_api::setup",
                    extension = %ext_id.as_str(),
                    templates = ids.len(),
                    "imported extension setup templates"
                );
            }
            Err(e) => {
                tracing::warn!(
                    target: "nexus_api::setup",
                    extension = %ext_id.as_str(),
                    error = %e,
                    "skipping extension's setup-template contribution"
                );
            }
        }
    }
    total
}
