//! [`WorkersScheduler`] — one task per worker, no shared queue.
//!
//! The scheduler enumerates every validated extension record in the
//! [`ExtensionRegistry`] at construction time and, for every entry in
//! `contributes.workers`, spawns one self-pacing tokio task. Each task
//! computes its own `next_due`, sleeps to it, fires the dispatcher,
//! updates the shared [`WorkerState`] map, and loops.
//!
//! State for every worker lives in the same `Arc<Mutex<HashMap>>`,
//! which the scheduler also exposes as a [`WorkerStateSource`] so the
//! admin route can surface it on `GET /extensions/<id>` without
//! talking to the running tasks.
//!
//! Testing seam: [`WorkersSchedulerHandle::tick_now`] sends through
//! a per-worker `tokio::sync::Notify`; the task wakes immediately,
//! runs the dispatcher, and re-arms `next_due` against the policy.
//! Deterministic tests use this to assert
//!
//! - first-failure → `BackingOff`, attempt = 1, next_due = now + initial_backoff.
//! - `max_attempts` failures → `Stopped`, next_due = None.
//! - a success after failures → `Healthy`, attempt = 0.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use rand::Rng;
use starter_ext_host::ExtensionRegistry;
use starter_ext_spi::{ContributeWorker, ExtensionId, OnErrorPolicy, RetryStrategy};
use tokio::sync::Notify;
use tokio::task::JoinHandle;

use crate::dispatcher::{WorkerDispatcher, DEFAULT_WORKER_TIMEOUT};
use crate::state::{WorkerState, WorkerStateSource, WorkerStatus};

// ---------------------------------------------------------------------------
// SchedulerOptions
// ---------------------------------------------------------------------------

/// Knobs the consumer hands to [`WorkersScheduler::start`].
#[derive(Debug, Clone)]
pub struct SchedulerOptions {
    /// Timeout passed to every [`WorkerDispatcher::run`] call. A
    /// dispatch exceeding it counts as a `Timeout` failure.
    pub request_timeout: Duration,
    /// If `true`, the scheduler fires each worker once immediately on
    /// start (after a small `initial_delay`). If `false`, it waits a
    /// full `interval_seconds` before the first run. Defaults to
    /// `false` so a host startup does not stampede external systems.
    pub fire_immediately: bool,
    /// Delay before the first scheduled tick when `fire_immediately`
    /// is `true`. Spreads load when many workers share an interval.
    pub initial_delay: Duration,
}

impl Default for SchedulerOptions {
    fn default() -> Self {
        Self {
            request_timeout: DEFAULT_WORKER_TIMEOUT,
            fire_immediately: false,
            initial_delay: Duration::from_millis(250),
        }
    }
}

// ---------------------------------------------------------------------------
// Per-worker descriptor (frozen at start)
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct WorkerDescriptor {
    extension: ExtensionId,
    worker_id: String,
    interval: Duration,
    jitter: Duration,
    on_error: OnErrorPolicy,
}

impl WorkerDescriptor {
    fn from_manifest(extension: ExtensionId, w: &ContributeWorker) -> Self {
        Self {
            extension,
            worker_id: w.id.clone(),
            interval: Duration::from_secs(u64::from(w.interval_seconds)),
            jitter: Duration::from_secs(u64::from(w.jitter_seconds)),
            on_error: w.on_error.clone(),
        }
    }
}

// ---------------------------------------------------------------------------
// WorkersScheduler
// ---------------------------------------------------------------------------

/// Build-time view of the scheduler. Hand it the registry and the
/// dispatcher; call [`WorkersScheduler::start`] to spawn the tasks.
pub struct WorkersScheduler {
    registry: Arc<ExtensionRegistry>,
    dispatcher: Arc<dyn WorkerDispatcher>,
}

impl WorkersScheduler {
    /// New scheduler. Does not spawn anything yet — call
    /// [`Self::start`] once the host is ready to take periodic work.
    pub fn new(registry: Arc<ExtensionRegistry>, dispatcher: Arc<dyn WorkerDispatcher>) -> Self {
        Self {
            registry,
            dispatcher,
        }
    }

    /// Enumerate workers and spawn one task per worker. Returns a
    /// cheap-to-clone handle the host uses to observe state and
    /// invoke the [`WorkersSchedulerHandle::tick_now`] testing seam.
    pub fn start(self, options: SchedulerOptions) -> WorkersSchedulerHandle {
        let states: Arc<Mutex<HashMap<(ExtensionId, String), WorkerState>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let notifies: Arc<Mutex<HashMap<(ExtensionId, String), Arc<Notify>>>> =
            Arc::new(Mutex::new(HashMap::new()));

        let mut tasks: Vec<JoinHandle<()>> = Vec::new();

        for rec in self.registry.list() {
            let Some(ext_id) = rec.id.clone() else {
                continue;
            };
            let Some(manifest) = rec.manifest.as_ref() else {
                continue;
            };
            for w in &manifest.contributes.workers {
                let desc = WorkerDescriptor::from_manifest(ext_id.clone(), w);
                let now = SystemTime::now();
                let first_due = if options.fire_immediately {
                    Some(now + options.initial_delay)
                } else {
                    Some(now + desc.interval)
                };
                let initial = WorkerState {
                    worker_id: desc.worker_id.clone(),
                    extension_id: desc.extension.clone(),
                    status: WorkerStatus::Healthy,
                    last_run: None,
                    last_error: None,
                    next_due: first_due,
                    attempt: 0,
                    total_runs: 0,
                };
                states
                    .lock()
                    .unwrap()
                    .insert((desc.extension.clone(), desc.worker_id.clone()), initial);

                let notify = Arc::new(Notify::new());
                notifies
                    .lock()
                    .unwrap()
                    .insert((desc.extension.clone(), desc.worker_id.clone()), notify.clone());

                let task = tokio::spawn(run_worker(
                    desc,
                    self.dispatcher.clone(),
                    states.clone(),
                    notify,
                    options.clone(),
                    first_due,
                ));
                tasks.push(task);
            }
        }

        WorkersSchedulerHandle {
            inner: Arc::new(SchedulerInner {
                states,
                notifies,
                tasks: Mutex::new(tasks),
            }),
        }
    }
}

/// Cheap-to-clone handle on a running scheduler. Hand a clone to the
/// admin layer for [`WorkerStateSource`] and to tests for
/// [`Self::tick_now`].
#[derive(Clone)]
pub struct WorkersSchedulerHandle {
    inner: Arc<SchedulerInner>,
}

struct SchedulerInner {
    states: Arc<Mutex<HashMap<(ExtensionId, String), WorkerState>>>,
    notifies: Arc<Mutex<HashMap<(ExtensionId, String), Arc<Notify>>>>,
    tasks: Mutex<Vec<JoinHandle<()>>>,
}

impl WorkersSchedulerHandle {
    /// Force an immediate tick of a single worker. Returns `true` if
    /// the worker was found and notified; `false` if no such worker
    /// is scheduled (unknown extension or worker_id). The task wakes,
    /// runs the dispatcher, applies `on_error`, and re-arms
    /// `next_due` exactly as a normal periodic firing would.
    ///
    /// Deterministic-tests-only seam (the scheduler does not assume
    /// this is called in production). It is also useful to admins
    /// who want to "run this worker now" — once the admin route
    /// surface lands.
    pub fn tick_now(&self, extension: &ExtensionId, worker_id: &str) -> bool {
        let key = (extension.clone(), worker_id.to_owned());
        let n = self.inner.notifies.lock().unwrap().get(&key).cloned();
        match n {
            Some(n) => {
                n.notify_one();
                true
            }
            None => false,
        }
    }

    /// Snapshot every known worker state for the given extension.
    pub fn snapshot_for(&self, extension: &ExtensionId) -> Vec<WorkerState> {
        self.inner
            .states
            .lock()
            .unwrap()
            .iter()
            .filter(|((e, _), _)| e == extension)
            .map(|(_, v)| v.clone())
            .collect()
    }

    /// Snapshot of one worker, if it exists.
    pub fn snapshot_one(&self, extension: &ExtensionId, worker_id: &str) -> Option<WorkerState> {
        self.inner
            .states
            .lock()
            .unwrap()
            .get(&(extension.clone(), worker_id.to_owned()))
            .cloned()
    }

    /// Cancel every task. Used by tests / clean shutdown. The handle
    /// is unusable afterwards (snapshots remain readable but no new
    /// ticks fire).
    pub async fn shutdown(&self) {
        let mut guard = self.inner.tasks.lock().unwrap();
        for h in guard.drain(..) {
            h.abort();
        }
    }
}

impl WorkerStateSource for WorkersSchedulerHandle {
    fn workers_for(&self, extension: &ExtensionId) -> Vec<WorkerState> {
        self.snapshot_for(extension)
    }
}

// ---------------------------------------------------------------------------
// Per-worker task body
// ---------------------------------------------------------------------------

async fn run_worker(
    desc: WorkerDescriptor,
    dispatcher: Arc<dyn WorkerDispatcher>,
    states: Arc<Mutex<HashMap<(ExtensionId, String), WorkerState>>>,
    notify: Arc<Notify>,
    options: SchedulerOptions,
    mut next_due: Option<SystemTime>,
) {
    let key = (desc.extension.clone(), desc.worker_id.clone());

    loop {
        // Sleep to `next_due` (or sleep forever if Stopped). Either
        // way, an explicit `tick_now` wakes us up.
        match next_due {
            Some(t) => {
                let now = SystemTime::now();
                let wait = t.duration_since(now).unwrap_or(Duration::ZERO);
                tokio::select! {
                    _ = tokio::time::sleep(wait) => {}
                    _ = notify.notified() => {}
                }
            }
            None => {
                // Stopped: only `tick_now` wakes us.
                notify.notified().await;
            }
        }

        let result = dispatcher
            .run(&desc.extension, &desc.worker_id, options.request_timeout)
            .await;

        let now = SystemTime::now();
        next_due = {
            let mut guard = states.lock().unwrap();
            let st = guard.get_mut(&key).expect("worker state row missing");
            st.last_run = Some(now);
            st.total_runs = st.total_runs.saturating_add(1);
            match result {
                Ok(()) => {
                    st.attempt = 0;
                    st.last_error = None;
                    st.status = WorkerStatus::Healthy;
                    let bump = desc.interval + jitter(desc.jitter);
                    let nd = now + bump;
                    st.next_due = Some(nd);
                    Some(nd)
                }
                Err(e) => {
                    st.attempt = st.attempt.saturating_add(1);
                    st.last_error = Some(e.to_string());

                    if e.is_fatal_config() {
                        // Configuration error — no amount of retries helps.
                        st.status = WorkerStatus::Stopped;
                        st.next_due = None;
                        tracing::warn!(
                            extension = %desc.extension.as_str(),
                            worker = %desc.worker_id,
                            err = %e,
                            "worker stopped: configuration error"
                        );
                        None
                    } else {
                        apply_error_policy(&desc.on_error, st, now)
                    }
                }
            }
        };
    }
}

/// Compute the next-due time given the on-error policy + current
/// attempt count on `state`. Mutates `state.status` / `state.next_due`
/// and returns the new `next_due` (or `None` for stopped).
fn apply_error_policy(
    policy: &OnErrorPolicy,
    state: &mut WorkerState,
    now: SystemTime,
) -> Option<SystemTime> {
    if matches!(policy.retry, RetryStrategy::Never) {
        state.status = WorkerStatus::Stopped;
        state.next_due = None;
        return None;
    }

    if state.attempt >= policy.max_attempts {
        state.status = WorkerStatus::Stopped;
        state.next_due = None;
        tracing::warn!(
            extension = %state.extension_id.as_str(),
            worker = %state.worker_id,
            attempt = state.attempt,
            "worker stopped: max_attempts reached"
        );
        return None;
    }

    // Exponential: initial * 2^(attempt-1), capped by max.
    let initial_ms = u64::from(policy.initial_backoff_ms);
    let max_ms = u64::from(policy.max_backoff_ms);
    let exp = (state.attempt - 1).min(31);
    let backoff_ms = initial_ms.saturating_mul(1u64 << exp).min(max_ms);
    let nd = now + Duration::from_millis(backoff_ms);
    state.status = WorkerStatus::BackingOff;
    state.next_due = Some(nd);
    Some(nd)
}

fn jitter(max: Duration) -> Duration {
    if max.is_zero() {
        return Duration::ZERO;
    }
    let max_ms = max.as_millis() as u64;
    let r = rand::thread_rng().gen_range(0..=max_ms);
    Duration::from_millis(r)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dispatcher::{BuiltinWorkerDispatcher, BuiltinWorkerRegistry};
    use starter_ext_spi::Error;
    use std::sync::atomic::{AtomicU32, Ordering};

    fn registry_with_one_worker() -> (Arc<ExtensionRegistry>, ExtensionId, String) {
        use starter_ext_host::ExtensionRecord;
        use starter_ext_spi::{LifecycleState, Manifest};
        use std::collections::HashMap;
        use std::path::PathBuf;

        let yaml = r#"
v: 1
id: com.acme.tick
version: 0.1.0
display_name: "Tick"
runtime: { kind: builtin, crate_name: tick }
contributes:
  workers:
    - id: com.acme.tick.heartbeat
      interval_seconds: 60
      description_file: w.md
      on_error:
        retry: exponential
        initial_backoff_ms: 10
        max_backoff_ms: 100
        max_attempts: 3
"#;
        let manifest: Manifest = serde_yaml::from_str(yaml).unwrap();
        let ext = ExtensionId::new("com.acme.tick").unwrap();
        let record = ExtensionRecord {
            id: Some(ext.clone()),
            id_hint: "com.acme.tick".into(),
            bundle_dir: PathBuf::from("/tmp/com.acme.tick"),
            state: LifecycleState::Validated,
            manifest: Some(manifest),
            failure: None,
        };
        let mut registry = ExtensionRegistry::new();
        let mut map = HashMap::new();
        map.insert("com.acme.tick".to_string(), record);
        registry.install(map);
        registry.seal();
        (Arc::new(registry), ext, "com.acme.tick.heartbeat".to_string())
    }

    #[tokio::test]
    async fn tick_now_runs_handler_and_records_success() {
        let (registry, ext, wid) = registry_with_one_worker();
        let count = Arc::new(AtomicU32::new(0));
        let c = count.clone();
        let reg = BuiltinWorkerRegistry::new().register(ext.clone(), wid.clone(), move |_| {
            c.fetch_add(1, Ordering::SeqCst);
            Ok(())
        });
        let dispatcher = Arc::new(BuiltinWorkerDispatcher::new(Arc::new(reg)));
        let h = WorkersScheduler::new(registry, dispatcher).start(SchedulerOptions::default());

        assert!(h.tick_now(&ext, &wid));
        // Let the task run.
        for _ in 0..50 {
            if count.load(Ordering::SeqCst) == 1 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert_eq!(count.load(Ordering::SeqCst), 1);
        let st = h.snapshot_one(&ext, &wid).unwrap();
        assert_eq!(st.status, WorkerStatus::Healthy);
        assert_eq!(st.attempt, 0);
        assert!(st.last_run.is_some());
        assert!(st.next_due.is_some());
        h.shutdown().await;
    }

    #[tokio::test]
    async fn repeated_failure_reaches_stopped_after_max_attempts() {
        let (registry, ext, wid) = registry_with_one_worker();
        let reg = BuiltinWorkerRegistry::new().register(ext.clone(), wid.clone(), |_| {
            Err(Error::extension_internal("boom"))
        });
        let dispatcher = Arc::new(BuiltinWorkerDispatcher::new(Arc::new(reg)));
        let h = WorkersScheduler::new(registry, dispatcher).start(SchedulerOptions::default());

        // Drive max_attempts (3) ticks deterministically.
        for _ in 0..3 {
            assert!(h.tick_now(&ext, &wid));
            // Wait for the tick to land.
            for _ in 0..50 {
                let st = h.snapshot_one(&ext, &wid).unwrap();
                if st.last_run.is_some() && st.attempt >= 1 {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        }
        // The third failure should have transitioned to Stopped.
        // We may have to wait a beat for the last update.
        for _ in 0..50 {
            let st = h.snapshot_one(&ext, &wid).unwrap();
            if st.status == WorkerStatus::Stopped {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        let st = h.snapshot_one(&ext, &wid).unwrap();
        assert_eq!(st.status, WorkerStatus::Stopped);
        assert_eq!(st.attempt, 3);
        assert!(st.next_due.is_none());
        assert_eq!(st.last_error.as_deref(), Some("extension internal: boom"));
        h.shutdown().await;
    }

    #[tokio::test]
    async fn success_after_failure_resets_attempt_and_clears_error() {
        let (registry, ext, wid) = registry_with_one_worker();
        let fail_first = Arc::new(AtomicU32::new(0));
        let f = fail_first.clone();
        let reg = BuiltinWorkerRegistry::new().register(ext.clone(), wid.clone(), move |_| {
            let n = f.fetch_add(1, Ordering::SeqCst);
            if n == 0 {
                Err(Error::extension_internal("once"))
            } else {
                Ok(())
            }
        });
        let dispatcher = Arc::new(BuiltinWorkerDispatcher::new(Arc::new(reg)));
        let h = WorkersScheduler::new(registry, dispatcher).start(SchedulerOptions::default());

        h.tick_now(&ext, &wid);
        for _ in 0..50 {
            let st = h.snapshot_one(&ext, &wid).unwrap();
            if st.attempt == 1 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        let st = h.snapshot_one(&ext, &wid).unwrap();
        assert_eq!(st.attempt, 1);
        assert_eq!(st.status, WorkerStatus::BackingOff);

        h.tick_now(&ext, &wid);
        for _ in 0..50 {
            let st = h.snapshot_one(&ext, &wid).unwrap();
            if st.attempt == 0 && st.status == WorkerStatus::Healthy {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        let st = h.snapshot_one(&ext, &wid).unwrap();
        assert_eq!(st.attempt, 0);
        assert_eq!(st.status, WorkerStatus::Healthy);
        assert!(st.last_error.is_none());
        h.shutdown().await;
    }
}
