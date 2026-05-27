//! Durable-scheduler boot wiring (Phase D.2 of Goal 6).
//!
//! Constructs a [`starter_flow_surfaces::service::FlowAsService`]
//! over the same Postgres pool the rest of the agent uses, seeds
//! `starter_scheduled_flows` from every bundled rubix flow whose
//! YAML carries `trigger: schedule` + `cron_expr`, and spawns the
//! 60-second tick task via [`FlowAsService::start`].
//!
//! Dispatch goes through a thin [`ToolRegistryRunner`] that looks
//! the flow id up in the MCP [`ToolRegistry`] (every bundled flow
//! is auto-surfaced as a `FlowAsTool` per SCOPE R7, so the tool
//! id equals the flow id) and calls `Tool::invoke({})`. That
//! keeps the scheduler completely decoupled from the engine /
//! adapter wiring — the same wrapper the MCP surface and REST
//! `/api/v1/tools/{id}` already drive is reused verbatim.
//!
//! The seeder is idempotent: `register_schedule` uses
//! `ON CONFLICT (tenant_id, flow_id) DO UPDATE`, so a second boot
//! with the same bundle either re-arms the row's `next_run_at`
//! (if the cron is unchanged the value is recomputed but the row
//! count stays steady) or rewrites the cron expression. No work
//! is needed to drop schedules whose flows are no longer
//! bundled in this stage; that lands when the registry learns to
//! unregister missing definitions.
//!
//! See `.codeless/jobs/rubix-goal-6-weekly-report/SCOPE.md`
//! Phase D.2 for the contract this file lands.

use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use tokio::task::JoinHandle;
use tracing::{info, warn};
use uuid::Uuid;

use starter_flow_surfaces::clock::SystemClock;
use starter_flow_surfaces::service::{FlowAsService, FlowRunner};
use starter_flow_surfaces::FlowRegistry;
use starter_mcp::registry::ToolRegistry;
use starter_store_postgres::pool::Pool;

use crate::boot::config::SchedulerConfig;
use crate::boot::flows_seed::SYSTEM_TENANT;

/// Outcome of [`spawn`]. The `JoinHandle` is the tick task; the
/// `seeded` count is how many `(tenant, flow)` rows the bundle
/// touched on this boot (re-arms count alongside fresh inserts
/// because `register_schedule` is upsert-shaped).
pub struct SchedulerHandle {
    /// Background tick task. The caller leaks this into the
    /// process lifetime exactly like the undo-sweep handle does —
    /// runtime shutdown drops the task.
    pub task: JoinHandle<()>,
    /// Number of bundled `trigger: schedule` flows registered on
    /// this boot.
    pub seeded: usize,
}

/// Construct [`FlowAsService`], seed schedules from the bundled
/// rubix flows, and spawn the tick task.
///
/// Returns `Ok(None)` (and logs at info) when the scheduler is
/// disabled in [`SchedulerConfig`] so the laptop / single-shot
/// CLI paths can opt out without a code change.
pub async fn spawn(
    pool: Pool,
    tools: Arc<ToolRegistry>,
    cfg: &SchedulerConfig,
) -> Result<Option<SchedulerHandle>> {
    if !cfg.enabled {
        info!(
            target: "rubix.boot.scheduler",
            "scheduler disabled via [scheduler].enabled = false — skipping",
        );
        return Ok(None);
    }

    // The scheduler's `FlowRegistry` is intentionally a fresh,
    // empty instance: the tick path dispatches through the
    // `FlowRunner` impl below (which reads the existing MCP
    // `ToolRegistry`), and the registry handle is only retained
    // by `FlowAsService` for future `register_schedule` callers
    // that want to introspect resolved metadata. Wiring the MCP
    // registry through here would force a refactor of
    // `build_mcp_surface` for no scheduler-side gain in this
    // stage; that consolidation is tracked alongside the goal-3
    // flow-programmer verbs.
    let registry = Arc::new(FlowRegistry::new());
    let runner: Arc<dyn FlowRunner> = Arc::new(ToolRegistryRunner { tools });

    let pool_for_prune = pool.clone();
    let svc = FlowAsService::new(pool, registry, runner).with_clock(Arc::new(SystemClock::new()));

    // Single source of truth for the bundled `(flow_id,
    // cron_expr)` list — shared with the always-on
    // `boot::flow_runtime` mounter so both surfaces never drift on
    // which flows are "scheduled today".
    let mut seeded = 0usize;
    let mut bundled_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
    for entry in super::flow_runtime::bundled_schedule_pairs()? {
        let next_run_at = svc
            .register_schedule(SYSTEM_TENANT, &entry.flow_id, &entry.cron_expr)
            .await
            .map_err(|e| {
                anyhow::anyhow!("register_schedule(`{flow}`): {e}", flow = entry.flow_id)
            })?;
        seeded += 1;
        bundled_ids.insert(entry.flow_id.clone());
        info!(
            target: "rubix.boot.scheduler",
            flow_id = %entry.flow_id,
            cron_expr = %entry.cron_expr,
            next_run_at = %next_run_at,
            "seeded scheduled flow"
        );
    }

    // Orphan-prune pass. Any `starter_scheduled_flows` row owned
    // by the bundled sentinel (`created_by = SYSTEM_TENANT`) whose
    // `flow_id` is no longer in the bundled schedule set is a
    // leftover from a previous binary version (the YAML was
    // removed or had its `trigger: schedule` stripped). DELETE it
    // so the tick loop stops claiming it every cycle and warning
    // about a missing tool. User-deployed schedules
    // (`created_by ≠ SYSTEM_TENANT`) are deliberately untouched.
    let live_system_ids: Vec<String> = sqlx::query_scalar(
        "SELECT flow_id
           FROM starter_scheduled_flows
          WHERE tenant_id = $1::uuid
            AND created_by = $1::uuid",
    )
    .bind(SYSTEM_TENANT)
    .fetch_all(pool_for_prune.sqlx())
    .await?;
    let mut pruned = 0usize;
    for live_id in live_system_ids {
        if bundled_ids.contains(&live_id) {
            continue;
        }
        let result = sqlx::query(
            "DELETE FROM starter_scheduled_flows
              WHERE tenant_id = $1::uuid
                AND created_by = $1::uuid
                AND flow_id = $2",
        )
        .bind(SYSTEM_TENANT)
        .bind(&live_id)
        .execute(pool_for_prune.sqlx())
        .await?;
        let affected = result.rows_affected() as usize;
        pruned += affected;
        if affected > 0 {
            info!(
                target: "rubix.boot.scheduler",
                flow_id = %live_id,
                "pruned orphan bundled schedule (YAML removed from binary)",
            );
        }
    }

    info!(
        target: "rubix.boot.scheduler",
        seeded,
        pruned,
        tick_interval_seconds = cfg.tick_interval_seconds,
        "scheduler spawning tick task"
    );

    let task = svc.start();
    Ok(Some(SchedulerHandle { task, seeded }))
}

/// `Tool::invoke({})` adapter implementing
/// [`FlowRunner`]. Looks the flow id up in the MCP
/// [`ToolRegistry`] — every bundled flow is exposed there via
/// `FlowAsTool` with `tool_id == flow_id` (see
/// `rubix-agent/src/boot/mcp/register.rs`). A missing tool is
/// reported as a runner error so the tick records `failed` and
/// the operator can spot the drift.
struct ToolRegistryRunner {
    tools: Arc<ToolRegistry>,
}

#[async_trait]
impl FlowRunner for ToolRegistryRunner {
    async fn run(
        &self,
        _tenant_id: Uuid,
        flow_id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let tool = self.tools.get(flow_id).ok_or_else(|| {
            Box::<dyn std::error::Error + Send + Sync>::from(format!(
                "scheduled flow `{flow_id}` not present in MCP tool registry"
            ))
        })?;
        match tool.invoke(serde_json::json!({})).await {
            Ok(_) => Ok(()),
            Err(e) => {
                // The outer `Display` for `SpiError::Internal` is
                // just the literal "internal error"; walk the
                // `source()` chain so the scheduler log has the
                // real failure (engine error, kind not registered,
                // bad config, etc.) instead of an opaque label.
                let mut detail = e.to_string();
                let mut src: Option<&(dyn std::error::Error + 'static)> =
                    std::error::Error::source(&e);
                while let Some(cause) = src {
                    detail.push_str(": ");
                    detail.push_str(&cause.to_string());
                    src = cause.source();
                }
                warn!(
                    target: "rubix.boot.scheduler",
                    flow_id = %flow_id,
                    error = %detail,
                    "scheduled flow dispatch failed"
                );
                Err(Box::<dyn std::error::Error + Send + Sync>::from(detail))
            }
        }
    }
}

// Bundled-YAML enumeration moved to
// `super::flow_runtime::bundled_schedule_pairs` so the always-on
// runtime and the durable scheduler share one definition.
