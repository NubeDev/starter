//! nexus-watchdog — external liveness supervisor for nexus-api (WS-16 Phase C).
//!
//! Why a separate binary (not a thread inside nexus-api): the failure mode this
//! defends against is a wedged tokio runtime. When every worker in the server is
//! parked on a futex, *no tokio-scheduled task can fire* — including
//! `tokio::time::timeout`, including the runtime canary tick, including sqlx's
//! `acquire_timeout`. A defence that lives inside the wedged runtime is, by
//! construction, also wedged. The supervisor must be an OS-level peer process.
//!
//! Ported from `rubix/crates/rubix-watchdog`, which earned this design after a
//! runtime wedge that took hours to localise by absence-of-log archaeology.
//!
//! ## Behaviour
//!
//! Every [`Config::probe_interval`] (default 10s), GET `Config::probe_url`
//! (default `http://127.0.0.1:4780/livez`) with a 3-second HTTP timeout. Any
//! non-200 / timeout / connection error counts as one failure. On
//! [`Config::failure_threshold`] (default 3) consecutive failures — i.e. the
//! server has been unreachable for ~30s — escalate:
//!
//!   1. **SIGUSR1** the server (last-gasp metrics dump, if a handler is wired).
//!      If the server recovers between SIGUSR1 and the next probe, the failure
//!      counter resets and no kill happens.
//!   2. Wait `SIGNAL_GRACE_USR1` (2s).
//!   3. **SIGABRT** the server. Produces a core dump (if `ulimit -c unlimited`)
//!      for post-mortem `gdb -c core` work — the only way to recover the futex
//!      chain after the fact, so preferred over SIGKILL.
//!   4. Wait `SIGNAL_GRACE_ABRT` (5s) for the process to exit.
//!   5. **SIGKILL** if still alive — give up on graceful.
//!   6. Wait for the process to be reaped (poll `/proc/<pid>`).
//!   7. Run [`Config::restart_cmd`] via `sh -c`. The restart command is expected
//!      to daemonise (e.g. `nohup ... &`); the watchdog does not track the
//!      restarted process directly — it resumes probing `/livez` and finds the
//!      new server when it comes up.
//!
//! ## What the watchdog deliberately does *not* do
//!
//! - It does not parse the server's log. The probe URL is the contract.
//! - It does not retry the restart command on failure. If the restart command
//!   itself is broken, that's an operator problem, not something to paper over.
//! - It does not adapt its cadence. Constant cadence keeps behaviour boring and
//!   predictable; clever back-off hides bugs.
//! - It does not compete with systemd. Under systemd, set `Restart=no` on the
//!   nexus-api service and let the watchdog own lifecycle.
//!
//! ## Configuration
//!
//! Every knob is an env var so dev/prod parity is trivial:
//!
//! | Env var | Default | Meaning |
//! |---|---|---|
//! | `NEXUS_WATCHDOG_URL` | `http://127.0.0.1:4780/livez` | Probe target |
//! | `NEXUS_WATCHDOG_PROBE_INTERVAL_SECS` | `10` | Cadence |
//! | `NEXUS_WATCHDOG_HTTP_TIMEOUT_SECS` | `3` | Per-probe ceiling |
//! | `NEXUS_WATCHDOG_FAILURE_THRESHOLD` | `3` | Consecutive failures before kill |
//! | `NEXUS_WATCHDOG_AGENT_PID_FILE` | _(unset)_ | If set, file containing PID to signal |
//! | `NEXUS_WATCHDOG_AGENT_PROCESS_NAME` | `nexus-api` | If pid-file unset, `pgrep -f $name$` |
//! | `NEXUS_WATCHDOG_RESTART_CMD` | _(required)_ | Shell command to spawn after kill |
//!
//! ## Running it
//!
//! ```bash
//! NEXUS_WATCHDOG_RESTART_CMD='cd /home/user/code/rust/starter/nexus && make dev-be' \
//!   cargo run -p nexus-watchdog
//! ```
//!
//! The watchdog itself never panics; any unexpected error in the probe / kill /
//! restart path is logged and the loop continues. Its own SIGTERM/SIGINT handler
//! exits cleanly without killing the server (so an operator can stop the
//! watchdog without taking nexus-api down).

use std::path::PathBuf;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use tokio::process::Command;
use tokio::signal::unix::{signal, SignalKind};
use tokio::time::{sleep, Instant};
use tracing::{error, info, warn};

const DEFAULT_URL: &str = "http://127.0.0.1:4780/livez";
const DEFAULT_PROBE_INTERVAL_SECS: u64 = 10;
const DEFAULT_HTTP_TIMEOUT_SECS: u64 = 3;
const DEFAULT_FAILURE_THRESHOLD: u32 = 3;
const DEFAULT_PROCESS_NAME: &str = "nexus-api";

const SIGNAL_GRACE_USR1: Duration = Duration::from_secs(2);
const SIGNAL_GRACE_ABRT: Duration = Duration::from_secs(5);
const REAP_POLL_INTERVAL: Duration = Duration::from_millis(200);
const REAP_TIMEOUT: Duration = Duration::from_secs(10);
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(60);

struct Config {
    probe_url: String,
    probe_interval: Duration,
    http_timeout: Duration,
    failure_threshold: u32,
    pid_source: PidSource,
    restart_cmd: String,
}

enum PidSource {
    /// Read the server PID from this file each time we need to signal. Lets the
    /// restart command write a fresh PID after it daemonises the new server.
    File(PathBuf),
    /// Discover the PID via `pgrep -f <name>$`. The trailing `$` anchor matches
    /// the freeze-investigation lookup.
    ProcessName(String),
}

impl Config {
    fn from_env() -> Result<Self> {
        let probe_url = std::env::var("NEXUS_WATCHDOG_URL").unwrap_or_else(|_| DEFAULT_URL.into());
        let probe_interval = Duration::from_secs(parse_env_u64(
            "NEXUS_WATCHDOG_PROBE_INTERVAL_SECS",
            DEFAULT_PROBE_INTERVAL_SECS,
        )?);
        let http_timeout = Duration::from_secs(parse_env_u64(
            "NEXUS_WATCHDOG_HTTP_TIMEOUT_SECS",
            DEFAULT_HTTP_TIMEOUT_SECS,
        )?);
        let failure_threshold = parse_env_u64(
            "NEXUS_WATCHDOG_FAILURE_THRESHOLD",
            DEFAULT_FAILURE_THRESHOLD as u64,
        )? as u32;
        let pid_source = match std::env::var("NEXUS_WATCHDOG_AGENT_PID_FILE").ok() {
            Some(p) if !p.is_empty() => PidSource::File(PathBuf::from(p)),
            _ => PidSource::ProcessName(
                std::env::var("NEXUS_WATCHDOG_AGENT_PROCESS_NAME")
                    .unwrap_or_else(|_| DEFAULT_PROCESS_NAME.into()),
            ),
        };
        let restart_cmd = std::env::var("NEXUS_WATCHDOG_RESTART_CMD").map_err(|_| {
            anyhow!("NEXUS_WATCHDOG_RESTART_CMD is required; set it to the shell command that re-launches nexus-api")
        })?;
        Ok(Self {
            probe_url,
            probe_interval,
            http_timeout,
            failure_threshold,
            pid_source,
            restart_cmd,
        })
    }
}

fn parse_env_u64(key: &str, default: u64) -> Result<u64> {
    match std::env::var(key) {
        Ok(v) => v
            .parse::<u64>()
            .with_context(|| format!("{key} must be a non-negative integer; got `{v}`")),
        Err(_) => Ok(default),
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .compact()
        .init();

    let cfg = Config::from_env().context("loading watchdog config")?;
    info!(
        probe_url = %cfg.probe_url,
        probe_interval_secs = cfg.probe_interval.as_secs(),
        http_timeout_secs = cfg.http_timeout.as_secs(),
        failure_threshold = cfg.failure_threshold,
        pid_source = ?match &cfg.pid_source {
            PidSource::File(p) => format!("file:{}", p.display()),
            PidSource::ProcessName(n) => format!("process_name:{n}"),
        },
        "nexus-watchdog starting",
    );

    let client = reqwest::Client::builder()
        .timeout(cfg.http_timeout)
        // No connection-pool benefit at one probe per 10s; setting
        // pool_max_idle_per_host(0) keeps the watchdog from holding sockets
        // between probes, which matters when the server has just been killed and
        // we don't want a stale keep-alive on the restarted listener.
        .pool_max_idle_per_host(0)
        .build()
        .context("building reqwest client")?;

    let mut sigterm = signal(SignalKind::terminate()).context("install SIGTERM handler")?;
    let mut sigint = signal(SignalKind::interrupt()).context("install SIGINT handler")?;

    let mut consecutive_failures: u32 = 0;
    let mut next_probe = Instant::now();
    let mut next_heartbeat = Instant::now() + HEARTBEAT_INTERVAL;

    loop {
        let now = Instant::now();
        let sleep_until = next_probe.min(next_heartbeat);
        let sleep_dur = sleep_until.saturating_duration_since(now);

        tokio::select! {
            _ = sigterm.recv() => {
                info!("SIGTERM received — exiting watchdog (server left running)");
                return Ok(());
            }
            _ = sigint.recv() => {
                info!("SIGINT received — exiting watchdog (server left running)");
                return Ok(());
            }
            _ = sleep(sleep_dur) => {}
        }

        let now = Instant::now();
        if now >= next_heartbeat {
            info!(consecutive_failures, "watchdog heartbeat");
            next_heartbeat = now + HEARTBEAT_INTERVAL;
        }
        if now < next_probe {
            continue;
        }
        next_probe = now + cfg.probe_interval;

        match probe(&client, &cfg.probe_url).await {
            Ok(()) => {
                if consecutive_failures > 0 {
                    info!(
                        recovered_after = consecutive_failures,
                        "probe succeeded — failure counter reset",
                    );
                }
                consecutive_failures = 0;
            }
            Err(e) => {
                consecutive_failures += 1;
                warn!(
                    error = %e,
                    consecutive_failures,
                    threshold = cfg.failure_threshold,
                    "probe failed",
                );
                if consecutive_failures >= cfg.failure_threshold {
                    if let Err(err) = handle_wedge(&cfg).await {
                        error!(error = %err, "wedge handler failed — will retry on next probe cycle");
                    }
                    // Reset regardless of handler outcome — we don't want to fire
                    // SIGABRT+restart twice back-to-back if the new server takes
                    // a moment to come up.
                    consecutive_failures = 0;
                }
            }
        }
    }
}

async fn probe(client: &reqwest::Client, url: &str) -> Result<()> {
    let response = client.get(url).send().await.context("send probe request")?;
    let status = response.status();
    if status.is_success() {
        Ok(())
    } else {
        Err(anyhow!("non-success status: {status}"))
    }
}

async fn handle_wedge(cfg: &Config) -> Result<()> {
    error!(
        consecutive_failures = cfg.failure_threshold,
        probe_url = %cfg.probe_url,
        "server unresponsive past threshold — initiating kill + restart",
    );

    let pid = match resolve_pid(&cfg.pid_source).await {
        Ok(p) => p,
        Err(e) => {
            warn!(
                error = %e,
                "could not resolve server PID — skipping signal escalation, going straight to restart",
            );
            return run_restart(&cfg.restart_cmd).await;
        }
    };
    info!(pid, "server PID resolved");

    // Step 1: SIGUSR1 — last-gasp metrics dump.
    info!(pid, "sending SIGUSR1 (last-gasp metrics dump)");
    if let Err(e) = send_signal(pid, libc::SIGUSR1) {
        warn!(pid, error = %e, "SIGUSR1 failed (continuing escalation)");
    }
    sleep(SIGNAL_GRACE_USR1).await;

    // Optional fast-recovery check: if the server answered between SIGUSR1 and
    // now, skip the kill. Cheap insurance — one extra HTTP request before doing
    // something destructive.
    if let Ok(client) = reqwest::Client::builder().timeout(cfg.http_timeout).build() {
        if probe(&client, &cfg.probe_url).await.is_ok() {
            info!(pid, "server recovered after SIGUSR1 — aborting kill");
            return Ok(());
        }
    }

    // Step 2: SIGABRT — core dump for forensics.
    info!(pid, "sending SIGABRT (core dump)");
    if let Err(e) = send_signal(pid, libc::SIGABRT) {
        warn!(pid, error = %e, "SIGABRT failed");
    }
    if wait_for_exit(pid, SIGNAL_GRACE_ABRT).await {
        info!(pid, "server exited after SIGABRT");
    } else {
        // Step 3: SIGKILL — give up on graceful.
        warn!(pid, "server still alive after SIGABRT grace — sending SIGKILL");
        if let Err(e) = send_signal(pid, libc::SIGKILL) {
            error!(pid, error = %e, "SIGKILL failed");
        }
        if !wait_for_exit(pid, REAP_TIMEOUT).await {
            return Err(anyhow!(
                "pid {pid} still alive after SIGKILL + {REAP_TIMEOUT:?} — refusing to restart"
            ));
        }
    }

    // Step 4: restart.
    run_restart(&cfg.restart_cmd).await
}

async fn resolve_pid(source: &PidSource) -> Result<i32> {
    match source {
        PidSource::File(path) => {
            let raw = tokio::fs::read_to_string(path)
                .await
                .with_context(|| format!("read pid file {}", path.display()))?;
            raw.trim()
                .parse::<i32>()
                .with_context(|| format!("parse pid from {}: `{}`", path.display(), raw.trim()))
        }
        PidSource::ProcessName(name) => {
            // `pgrep -f <name>$` mirrors the freeze-investigation workflow. We
            // take the single match — if multiple match, that's an operator
            // misconfiguration the watchdog surfaces rather than guessing.
            let output = Command::new("pgrep")
                .arg("-f")
                .arg(format!("{name}$"))
                .output()
                .await
                .context("spawn pgrep")?;
            if !output.status.success() {
                return Err(anyhow!(
                    "pgrep exited {} — no process matches `{name}$`",
                    output.status
                ));
            }
            let stdout = String::from_utf8_lossy(&output.stdout);
            let lines: Vec<&str> = stdout.lines().filter(|l| !l.trim().is_empty()).collect();
            match lines.len() {
                0 => Err(anyhow!("pgrep matched zero processes for `{name}$`")),
                1 => lines[0]
                    .trim()
                    .parse::<i32>()
                    .with_context(|| format!("parse pid from pgrep output `{}`", lines[0])),
                n => Err(anyhow!(
                    "pgrep matched {n} processes for `{name}$` — refusing to guess; set NEXUS_WATCHDOG_AGENT_PID_FILE"
                )),
            }
        }
    }
}

fn send_signal(pid: i32, sig: libc::c_int) -> Result<()> {
    // SAFETY: kill(2) is async-signal-safe and takes a process id + signal
    // number. Return value -1 means errno is set; we surface it as a Rust error.
    let rc = unsafe { libc::kill(pid, sig) };
    if rc == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error()).context(format!("kill({pid}, {sig})"))
    }
}

/// Poll `/proc/<pid>` until it disappears or `budget` elapses. Returns `true` if
/// the process exited inside the budget.
async fn wait_for_exit(pid: i32, budget: Duration) -> bool {
    let deadline = Instant::now() + budget;
    let proc_path = PathBuf::from(format!("/proc/{pid}"));
    while Instant::now() < deadline {
        if !proc_path.exists() {
            return true;
        }
        sleep(REAP_POLL_INTERVAL).await;
    }
    !proc_path.exists()
}

async fn run_restart(cmd: &str) -> Result<()> {
    info!(cmd, "spawning restart command");
    // `sh -c` so the operator can use shell features (cd, &&, env assignments,
    // nohup). The restart command is expected to daemonise — we do not wait for
    // it to finish, only check that the spawn itself succeeded.
    let status = Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .status()
        .await
        .with_context(|| format!("spawn restart command: {cmd}"))?;
    if status.success() {
        info!(cmd, "restart command exited 0");
        Ok(())
    } else {
        Err(anyhow!("restart command exited with status {status}: {cmd}"))
    }
}
