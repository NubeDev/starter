//! Process-level hooks: panic tracing, runtime canary, USR1 metrics.

use anyhow::Result;
use tracing::warn;

use rubix_agent::boot;

/// Opaque bundle of background task handles that must live for the
/// process lifetime. Dropping them aborts their tasks.
pub(crate) struct RuntimeDiagnostics {
    pub(crate) canary: boot::runtime_canary::Canary,
    pub(crate) _canary_task: tokio::task::JoinHandle<()>,
    pub(crate) _metrics_task: Option<tokio::task::JoinHandle<()>>,
}

/// Install a panic hook that logs every panic via `tracing` before
/// the default hook runs.
pub(crate) fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let loc = info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "<unknown>".into());
        let payload = info
            .payload()
            .downcast_ref::<&'static str>()
            .map(|s| (*s).to_owned())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "<non-string panic payload>".into());
        tracing::error!(
            target: "rubix.panic",
            location = %loc,
            thread = %std::thread::current().name().unwrap_or("<unnamed>"),
            payload = %payload,
            "panic caught",
        );
        default_hook(info);
    }));
}

/// Spawn the runtime liveness canary and the optional USR1 metrics
/// dump task. Returns handles that must be held for the process
/// lifetime.
pub(crate) fn install_runtime_diagnostics() -> Result<RuntimeDiagnostics> {
    let (canary, canary_task) = boot::runtime_canary::spawn();
    let _canary_task = boot::task_watchdog::watch("runtime_canary", canary_task);

    let _metrics_task = match boot::runtime_metrics::spawn() {
        Ok(h) => Some(boot::task_watchdog::watch("runtime_metrics", h)),
        Err(e) => {
            warn!(
                target: "rubix.boot.runtime_metrics",
                error = %e,
                "failed to install SIGUSR1 handler — metrics dump disabled",
            );
            None
        }
    };

    Ok(RuntimeDiagnostics {
        canary,
        _canary_task,
        _metrics_task,
    })
}
