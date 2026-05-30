//! Layered agent configuration.
//!
//! Replaces the ad-hoc `std::env::var(...)` calls in `main.rs` with
//! a single [`starter_config::Loader`] pipeline:
//!
//!   defaults  <  `$XDG_CONFIG_HOME/rubix/agent.toml`  <  `RUBIX_*`
//!
//! The struct is intentionally small — it carries only the wiring
//! knobs the binary itself needs at boot. Domain knobs (e.g. the
//! disk tool's history host id) stay on their own types per the
//! verb-per-file rule. See
//! [docs/design/config/](../../../docs/design/config/README.md).
//!
//! All fields are optional inside the loader. Unset values fall
//! through to the per-field defaults below so a developer can boot
//! the agent with `cargo run -p rubix-agent` against no config file
//! and no env vars at all (Postgres + ClickHouse are then skipped
//! by the migration steps as documented in their own boot files).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use starter_config::Loader;

/// What `main.rs` reads at boot. The fields cover the four wiring
/// inputs the binary needs and nothing more.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AgentConfig {
    /// Bind address for the HTTP listener. Mirrors the legacy
    /// `RUBIX_BIND` env var. Default `127.0.0.1:8088`.
    pub bind: String,

    /// Postgres DSN (changelog + auth-users tables + authz). When
    /// `None` the binary boots without DB-backed features; see
    /// [`crate::boot::migrations`].
    pub database_url: Option<String>,

    /// ClickHouse HTTP endpoint (history + warehouse). When
    /// `None` the binary boots without the warehouse; see
    /// [`crate::boot::clickhouse`].
    pub clickhouse_url: Option<String>,

    /// Postgres DSN that **ClickHouse itself** should use to reach
    /// the dimensions database when the shared
    /// `0005_entities_dict.sql` dictionary is queried. Differs from
    /// [`Self::database_url`] when ClickHouse runs inside docker and
    /// Postgres runs as a sibling container — `127.0.0.1` from the
    /// CH container resolves to the CH container itself, not the
    /// host, so the agent's own DSN does not work for the dictionary
    /// host. Falls back to [`Self::database_url`] when `None`.
    ///
    /// Example (compose service name):
    /// `postgres://rubix:rubix-dev@postgres:5432/rubix`.
    pub clickhouse_pg_url: Option<String>,

    /// TimescaleDB DSN for the warehouse history plane (`samples`,
    /// `events`, `raw_events`, `documents`). When `None` the binary
    /// boots without the warehouse plane: the `rubix.warehouse.ingest`
    /// tool degrades to a logging no-op and the SDUI `analytics_template`
    /// resolver returns empty (KPIs show `—`, charts show "no data").
    /// Honors the `RUBIX_WAREHOUSE_URL` env var as a fallback.
    pub warehouse_url: Option<String>,

    /// Path to the on-disk secrets directory. Reserved for the
    /// upcoming JWT signing key / OAuth client secret material.
    pub secrets_path: Option<PathBuf>,

    /// Explicit path to the config file. When `None` the loader
    /// falls back to `$XDG_CONFIG_HOME/rubix/agent.toml` then
    /// `$HOME/.config/rubix/agent.toml`.
    pub config_path: Option<PathBuf>,

    /// Which `AiRunner` [`crate::boot::ai::build_runner`] should
    /// construct. Defaults to `"claude-cli"` (the operator's
    /// `claude` binary on PATH). Parses any
    /// `starter_spi::ai::Provider` variant string; `"anthropic"`
    /// returns [`crate::boot::ai::AiError::Unimplemented`] in v0.
    pub ai_provider: Option<String>,

    /// Insights-gate knobs. The disk verb's post-dispatch hook
    /// reads [`InsightsConfig::disk_warn_threshold`] to decide when
    /// a percent-used reading is high enough to fire
    /// `rubix.alert.send`. Carried as a nested struct so the
    /// `[insights]` TOML section and the `RUBIX_INSIGHTS__*` env
    /// names match the loader's double-underscore convention.
    pub insights: InsightsConfig,

    /// Retention knobs for the `undo_snapshots` sweep task. See
    /// [`UndoConfig`] for field defaults; the sweep itself lives
    /// in [`crate::boot::undo_sweep`].
    pub undo: UndoConfig,

    /// Durable-scheduler knobs. See [`SchedulerConfig`] for
    /// defaults; the scheduler itself lives in
    /// [`crate::boot::scheduler`] and dispatches every bundled
    /// flow whose YAML carries `trigger: schedule` + `cron_expr`.
    pub scheduler: SchedulerConfig,

    /// Extension-host knobs. See [`ExtensionsConfig`]. The host
    /// itself lives in [`crate::boot::extensions`] and is
    /// constructed by `build_extension_admin` when `enabled = true`.
    pub extensions: ExtensionsConfig,

    /// Always-on flow runtime knobs. See [`FlowRuntimeConfig`]. The
    /// runtime itself lives in [`crate::boot::flow_runtime`] and owns
    /// the shared SSE-subscription registry plus the per-node state
    /// store backing every `NodeCtx::state` call.
    pub flow_runtime: FlowRuntimeConfig,

    /// Root directory for the filesystem blob store used by
    /// `rubix.analytics.report`. Blobs land at
    /// `<blob_root>/reports/<template>/<ulid>.<ext>`. Created on
    /// first boot if it does not exist. When `None`, defaults to
    /// `/tmp/rubix-blobs` so a developer can boot without any config.
    /// Override via `RUBIX_BLOB_ROOT` env var.
    pub blob_root: Option<String>,
}

/// Tunables for the always-on flow runtime
/// ([`crate::boot::flow_runtime`]).
///
/// Per-node durable state lives in the rubix Postgres database
/// (table `node_state`, applied via the upstream
/// `starter_store_postgres::flow::FLOW_MIGRATION_SOURCE`). When
/// `RUBIX_DATABASE_URL` is unset the runtime falls back to
/// [`starter_flow::state::in_memory::InMemoryNodeStateStore`] so a
/// laptop boot without Postgres keeps working — at the cost of
/// volatile node state.
///
/// The legacy SQLite path (`~/.rubix/node_state.db`) used by older
/// rubix-agent builds is migrated into Postgres at first boot of
/// the PG-backed runtime; see [`crate::boot::flow_runtime::build`]
/// and `rubix/docs/scope/sqlite-to-postgres.md`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct FlowRuntimeConfig {}

/// Tunables for the extension host.
///
/// The host is opt-out via `enabled = false` so an operator running
/// rubix-agent purely as a REST/MCP frontend can skip the
/// `extensions_enablement` migration and the on-boot supervisor
/// spawns. Per the installed-only model
/// (`rubix/docs/scope/extensions/installed-only-model.md`), bundles
/// reach the runtime only by being unpacked into `installs_dir` via
/// `POST /api/v1/extensions/install` — there is no dev-source-tree
/// scan. `installs_dir` defaults to `$RUBIX_DATA_ROOT/extensions/
/// installed/` (or the OS XDG default); production deployments may
/// override it to `/var/lib/rubix/extensions/installed`.
///
/// `autostart_enabled_records` controls whether the boot path reads
/// the `extensions_enablement` table and spawns a supervisor for
/// every persisted-enabled record. Turn it off in integration tests
/// that want to drive lifecycle transitions explicitly from the
/// admin routes without racing against autostart spawns.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ExtensionsConfig {
    /// When `false` the host is never constructed at boot. The
    /// `/api/v1/extensions/*` routes are not mounted and no
    /// migration runs against the Postgres pool. Default `true`.
    pub enabled: bool,

    /// Installed-bundle root. `POST /api/v1/extensions/install`
    /// unpacks bundles here and `DELETE /api/v1/extensions/<id>`
    /// removes them. When unset, the boot path resolves it from
    /// `starter_paths::Paths` — i.e. `$RUBIX_DATA_ROOT/extensions/
    /// installed/` (or the OS XDG default). Set it explicitly to
    /// override.
    pub installs_dir: Option<PathBuf>,

    /// When `true` the boot path reads every `Enabled` row from the
    /// PG store and spawns a supervisor for the matching record so
    /// the extension is `Running` before the HTTP listener starts
    /// accepting traffic. Default `true`.
    pub autostart_enabled_records: bool,
}

impl Default for ExtensionsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            installs_dir: None,
            autostart_enabled_records: true,
        }
    }
}

/// Tunables for the durable cron scheduler (Goal 6).
///
/// `enabled = false` lets an operator disable scheduled flows
/// without uninstalling the bundle — useful when running a
/// rubix-agent purely for the REST / MCP surface, or under
/// integration tests that don't want a 60-second tick task in
/// the background. `tick_interval_seconds` shadows the upstream
/// `FlowAsService::start` default (60s) for documentation; the
/// concrete tick cadence lives upstream in
/// `starter-flow-surfaces` and currently runs at the fixed 60s
/// default — this knob is parsed today so the TOML section is
/// self-describing for the moment the upstream surface exposes
/// an interval override.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SchedulerConfig {
    /// When `false` the scheduler is never constructed at boot.
    /// Default `true`.
    pub enabled: bool,

    /// Tick interval in whole seconds. Default `60`. Operators
    /// rarely need to tune this; the value is exposed so the
    /// `[scheduler]` TOML section is self-describing.
    pub tick_interval_seconds: u64,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            tick_interval_seconds: 60,
        }
    }
}

/// Bounds the on-disk size of `undo_snapshots`. The sweep keeps
/// **the smaller of** the two limits per
/// `(tenant_id, resource_kind, resource_id)`, so the limit that
/// bites first wins — a chatty resource hits `max_rows_per_resource`
/// before its oldest row turns `max_age_days` old, while a sleepy
/// resource ages out before it accumulates `max_rows_per_resource`
/// snapshots. Both knobs are exposed in `agent.toml` under
/// `[undo]` so operators can shrink the window without a rebuild.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UndoConfig {
    /// Maximum live snapshot rows to retain per
    /// `(tenant_id, resource_kind, resource_id)`. Older rows
    /// beyond this count are deleted on each sweep tick.
    /// Default `50` — large enough to cover a normal session's
    /// undo depth with headroom, small enough to keep the table
    /// from unbounded growth on hot resources.
    pub max_rows_per_resource: u32,

    /// Maximum age (in days) of any snapshot row. Rows older
    /// than this are deleted on each sweep tick regardless of
    /// how many remain for the resource. Default `90`.
    pub max_age_days: u32,
}

impl Default for UndoConfig {
    fn default() -> Self {
        Self {
            max_rows_per_resource: 50,
            max_age_days: 90,
        }
    }
}

/// Tunables for the v0 insights gate. Kept on its own type so the
/// `[insights]` TOML section nests cleanly and so additional rule
/// thresholds (CPU, memory, etc.) land here without re-flattening
/// the root config. The single field today maps the threshold the
/// hardcoded `if response.percent_used > N` in
/// `rubix_tools::system::disk::run_insights_gate` consults.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct InsightsConfig {
    /// Percent-used reading above which the disk verb fires an
    /// `rubix.alert.send` alert. Default `90` mirrors the v0
    /// hardcoded constant in `rubix-tools`. Lowering it (e.g. `50`
    /// in the alert-path integration test) deterministically makes
    /// the gate fire on a synthetic 60%-used response.
    pub disk_warn_threshold: u8,
}

impl Default for InsightsConfig {
    fn default() -> Self {
        Self {
            disk_warn_threshold: 90,
        }
    }
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            bind: "127.0.0.1:8088".to_owned(),
            database_url: None,
            clickhouse_url: None,
            clickhouse_pg_url: None,
            warehouse_url: None,
            secrets_path: None,
            config_path: None,
            ai_provider: None,
            insights: InsightsConfig::default(),
            undo: UndoConfig::default(),
            scheduler: SchedulerConfig::default(),
            extensions: ExtensionsConfig::default(),
            flow_runtime: FlowRuntimeConfig::default(),
            blob_root: None,
        }
    }
}

impl AgentConfig {
    /// Compose the loader chain and return the resolved config.
    ///
    /// Order (later wins):
    ///   1. [`AgentConfig::default`]
    ///   2. TOML file at the resolved [`Self::default_config_path`]
    ///   3. Env vars prefixed `RUBIX_` (double-underscore = nested)
    pub fn load() -> Result<Self, starter_config::ConfigError> {
        let cfg_path = std::env::var_os("RUBIX_CONFIG")
            .map(PathBuf::from)
            .unwrap_or_else(Self::default_config_path);
        let mut loaded: Self = Loader::with_defaults(Self::default())
            .with_file(cfg_path.to_string_lossy().into_owned())
            .with_env("RUBIX_")
            .load()?;

        // The two legacy env names predate the loader and stay
        // honored verbatim so existing deployments keep working
        // without editing systemd unit files. The loader's
        // `RUBIX_DATABASE_URL` / `RUBIX_CLICKHOUSE_URL` paths win
        // when both are set.
        if loaded.database_url.is_none() {
            loaded.database_url = std::env::var("RUBIX_DSN").ok();
        }
        if loaded.clickhouse_url.is_none() {
            loaded.clickhouse_url = std::env::var("RUBIX_CH_URL").ok();
        }
        if loaded.warehouse_url.is_none() {
            loaded.warehouse_url = std::env::var("RUBIX_WAREHOUSE_URL").ok();
        }
        if loaded.config_path.is_none() {
            loaded.config_path = Some(cfg_path);
        }
        Ok(loaded)
    }

    /// `$XDG_CONFIG_HOME/rubix/agent.toml`, falling back to
    /// `$HOME/.config/rubix/agent.toml`. Missing parents are not an
    /// error — [`Loader::with_file`] silently skips an absent file.
    pub fn default_config_path() -> PathBuf {
        if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
            return PathBuf::from(xdg).join("rubix/agent.toml");
        }
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home).join(".config/rubix/agent.toml");
        }
        PathBuf::from("agent.toml")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_bind_is_localhost_8088() {
        let cfg = AgentConfig::default();
        assert_eq!(cfg.bind, "127.0.0.1:8088");
        assert!(cfg.database_url.is_none());
        assert!(cfg.clickhouse_url.is_none());
    }

    #[test]
    fn default_config_path_uses_xdg_when_set() {
        let prior = std::env::var_os("XDG_CONFIG_HOME");
        std::env::set_var("XDG_CONFIG_HOME", "/tmp/xdg");
        let p = AgentConfig::default_config_path();
        assert!(p.ends_with("rubix/agent.toml"));
        assert!(p.to_string_lossy().contains("/tmp/xdg"));
        match prior {
            Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }
    }
}
