//! `starter-flow-watch` — host-dir file watcher that publishes
//! flow definitions through
//! [`starter_flow::definition::DefinitionManager`].
//!
//! Per `DOCS/flow/scope/hot-reload.md` HR7 (*"file-watch is one
//! publisher among many, not a special case"*):
//!
//! 1. A [`notify`]-backed watcher fires on file change (debounced
//!    to coalesce editor-saves; default 200 ms, tunable for
//!    networked filesystems whose atomic-rename windows can run
//!    up to ~1 s).
//! 2. The body parser reads the file (YAML or JSON), normalises to
//!    JSON, and hands the value to
//!    [`DefinitionManager::publish`](starter_flow::definition::DefinitionManager::publish)
//!    with `source = DefinitionSource::File { path }`.
//! 3. HR1's idempotent short-circuit drops no-op edits silently —
//!    an editor that touches a file without changing bytes does
//!    not churn the registry.
//! 4. A removed file calls
//!    [`DefinitionManager::publish_delete`](starter_flow::definition::DefinitionManager::publish_delete)
//!    which emits [`FlowDefinitionEvent::Removed`](starter_flow_spi::definition::FlowDefinitionEvent::Removed)
//!    and unmounts the active topology per the flow's
//!    `apply_policy`.
//!
//! The crate is **default-off**: enable the `watch` cargo feature
//! to pull in [`notify`] and `serde_yaml`. The non-`watch` build
//! only exposes the pure file-parsing helpers
//! ([`parse_flow_file`], [`FileEvent`]) so hosts can unit-test their
//! own watch loops without binding to a particular file-system
//! library.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use thiserror::Error;

use starter_flow::definition::DefinitionManager;
use starter_flow_spi::definition::DefinitionSource;
use starter_flow_spi::flow::FlowId;

/// Default debounce window for the file watcher.
///
/// 200 ms coalesces the typical "save-then-immediately-save-again"
/// editor pattern without making the operator wait visibly. Tunable
/// per [`WatchConfig::debounce`].
pub const DEFAULT_DEBOUNCE_MS: u64 = 200;

/// Errors returned by the watch layer's file-parsing path.
///
/// The watch task surfaces these to its `tracing` log and to the
/// optional event channel; it does NOT abort on individual file
/// failures (one bad YAML file does not stop the watcher — same
/// posture as HR6's *"one bad revision never poisons the flow"*).
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum WatchError {
    /// I/O error reading the file (e.g. the file was removed
    /// between the watch event firing and the read).
    #[error("io error reading {path}: {error}")]
    Io {
        /// The path that failed to read.
        path: PathBuf,
        /// The underlying I/O error rendered as a string so the
        /// error is `Clone` / `PartialEq` for the event channel.
        error: String,
    },

    /// The file's extension wasn't one of the recognised ones
    /// (`.yaml`, `.yml`, `.json`). The watcher silently ignores
    /// files that don't match — this variant is only returned
    /// when a host explicitly calls [`parse_flow_file`] on a path
    /// that doesn't have a known extension.
    #[error("unsupported file extension on {path}: expected .yaml, .yml, or .json")]
    UnsupportedExtension {
        /// The offending path.
        path: PathBuf,
    },

    /// YAML parse error.
    #[cfg(feature = "watch")]
    #[error("yaml parse error in {path}: {error}")]
    Yaml {
        /// The path that failed to parse.
        path: PathBuf,
        /// Underlying parse error.
        error: String,
    },

    /// JSON parse error.
    #[error("json parse error in {path}: {error}")]
    Json {
        /// The path that failed to parse.
        path: PathBuf,
        /// Underlying parse error.
        error: String,
    },

    /// The parsed body didn't carry a `flow_id` field, and the
    /// caller didn't derive one from the file path.
    #[error("flow body in {path} has no `flow_id` field")]
    MissingFlowId {
        /// The path whose body was missing the id.
        path: PathBuf,
    },

    /// The `flow_id` field wasn't a valid reverse-DNS identifier.
    #[error("flow body in {path} carries invalid flow_id `{value}`: {error}")]
    InvalidFlowId {
        /// The path whose body carried the bad id.
        path: PathBuf,
        /// The raw id string.
        value: String,
        /// Validation error.
        error: String,
    },
}

/// One parsed flow-file: the validated [`FlowId`] and the JSON body
/// suitable for [`DefinitionManager::publish`].
///
/// Returned by [`parse_flow_file`]. Hosts that want to drive their
/// own watch loop publish this through the manager themselves.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ParsedFlow {
    /// Validated flow id.
    pub flow_id: FlowId,
    /// Canonical JSON body — the value to pass to
    /// [`DefinitionManager::publish`].
    pub body: serde_json::Value,
    /// Originating file path.
    pub path: PathBuf,
}

/// Read and parse a flow file at `path`. Supports `.yaml`, `.yml`,
/// and `.json`. The `flow_id` is read from the body's `flow_id`
/// field; callers that want filename-based ids can override after
/// parsing.
pub fn parse_flow_file(path: &Path) -> Result<ParsedFlow, WatchError> {
    let bytes = std::fs::read(path).map_err(|e| WatchError::Io {
        path: path.to_path_buf(),
        error: e.to_string(),
    })?;

    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase());

    let body: serde_json::Value = match ext.as_deref() {
        #[cfg(feature = "watch")]
        Some("yaml") | Some("yml") => {
            serde_yaml::from_slice(&bytes).map_err(|e| WatchError::Yaml {
                path: path.to_path_buf(),
                error: e.to_string(),
            })?
        }
        #[cfg(not(feature = "watch"))]
        Some("yaml") | Some("yml") => {
            return Err(WatchError::UnsupportedExtension {
                path: path.to_path_buf(),
            });
        }
        Some("json") => serde_json::from_slice(&bytes).map_err(|e| WatchError::Json {
            path: path.to_path_buf(),
            error: e.to_string(),
        })?,
        _ => {
            return Err(WatchError::UnsupportedExtension {
                path: path.to_path_buf(),
            });
        }
    };

    let flow_id_str = body
        .get("flow_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| WatchError::MissingFlowId {
            path: path.to_path_buf(),
        })?;
    let flow_id = FlowId::new(flow_id_str).map_err(|e| WatchError::InvalidFlowId {
        path: path.to_path_buf(),
        value: flow_id_str.to_string(),
        error: e.to_string(),
    })?;

    Ok(ParsedFlow {
        flow_id,
        body,
        path: path.to_path_buf(),
    })
}

/// File-system event the watcher surfaces to its task.
///
/// Hosts that drive their own loop construct these directly and
/// hand them to [`apply_file_event`]; the built-in
/// [`watch_dir`] (feature `watch`) translates [`notify`] events
/// into these.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileEvent {
    /// A file was created or modified.
    Upsert(PathBuf),
    /// A file was removed.
    Remove(PathBuf),
}

/// Apply one [`FileEvent`] through the [`DefinitionManager`].
///
/// - `Upsert(path)` parses the file (silently ignoring unsupported
///   extensions) and calls
///   [`DefinitionManager::publish`](starter_flow::definition::DefinitionManager::publish).
///   Parse failures and publish failures are logged at `warn!` and
///   returned to the caller, but do not propagate as panics —
///   the watch loop keeps running.
/// - `Remove(path)` extracts the flow id from the *path* (the file
///   is already gone). Hosts that need a different mapping can
///   call [`DefinitionManager::publish_delete`] directly.
pub async fn apply_file_event(
    manager: &Arc<DefinitionManager>,
    event: FileEvent,
    flow_id_for_removal: impl FnOnce(&Path) -> Option<FlowId>,
) -> Result<(), WatchError> {
    match event {
        FileEvent::Upsert(path) => {
            let parsed = match parse_flow_file(&path) {
                Ok(p) => p,
                Err(WatchError::UnsupportedExtension { .. }) => {
                    // Silently ignore non-flow files in the watch dir.
                    return Ok(());
                }
                Err(e) => {
                    tracing::warn!(
                        path = %path.display(),
                        error = %e,
                        "flow-watch: failed to parse file"
                    );
                    return Err(e);
                }
            };
            let source = DefinitionSource::File { path: parsed.path.clone() };
            match manager.publish(parsed.flow_id, parsed.body, source).await {
                Ok(outcome) => {
                    tracing::info!(
                        path = %parsed.path.display(),
                        outcome = ?outcome,
                        "flow-watch: published"
                    );
                    Ok(())
                }
                Err(e) => {
                    tracing::warn!(
                        path = %parsed.path.display(),
                        error = %e,
                        "flow-watch: publish failed"
                    );
                    // Surface the error to the host as an Io-flavoured
                    // wrapper — the DefinitionManager error types are
                    // not re-exported here on purpose.
                    Err(WatchError::Io {
                        path: parsed.path,
                        error: e.to_string(),
                    })
                }
            }
        }
        FileEvent::Remove(path) => {
            let Some(flow_id) = flow_id_for_removal(&path) else {
                tracing::debug!(
                    path = %path.display(),
                    "flow-watch: remove ignored (no flow_id mapping)"
                );
                return Ok(());
            };
            let source = DefinitionSource::File { path: path.clone() };
            if let Err(e) = manager.publish_delete(flow_id.clone(), source).await {
                tracing::warn!(
                    path = %path.display(),
                    flow = %flow_id,
                    error = %e,
                    "flow-watch: publish_delete failed"
                );
                return Err(WatchError::Io {
                    path,
                    error: e.to_string(),
                });
            }
            Ok(())
        }
    }
}

/// Walk `dir` once and publish every flow file found, returning
/// the list of `(path, flow_id)` pairs so the caller can build a
/// path → flow-id map for the watch loop's `Remove` handler.
///
/// Per `DOCS/flow/scope/hot-reload.md` HR5 step 5: the boot walk
/// re-publishes any file whose on-disk content differs from the
/// `FlowStore` head; HR1's idempotent short-circuit makes this a
/// no-op when they match.
pub async fn boot_walk(
    manager: &Arc<DefinitionManager>,
    dir: &Path,
) -> Vec<(PathBuf, FlowId)> {
    let mut out = Vec::new();
    let read = match std::fs::read_dir(dir) {
        Ok(r) => r,
        Err(err) => {
            tracing::warn!(
                dir = %dir.display(),
                error = %err,
                "flow-watch: boot_walk read_dir failed"
            );
            return out;
        }
    };
    for entry in read.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        match parse_flow_file(&path) {
            Ok(parsed) => {
                let flow_id = parsed.flow_id.clone();
                let source = DefinitionSource::File { path: parsed.path.clone() };
                if let Err(e) = manager.publish(parsed.flow_id, parsed.body, source).await {
                    tracing::warn!(
                        path = %path.display(),
                        error = %e,
                        "flow-watch: boot_walk publish failed"
                    );
                }
                out.push((path, flow_id));
            }
            Err(WatchError::UnsupportedExtension { .. }) => continue,
            Err(e) => {
                tracing::warn!(
                    path = %path.display(),
                    error = %e,
                    "flow-watch: boot_walk parse failed"
                );
            }
        }
    }
    out
}

/// Configuration for [`watch_dir`].
#[cfg(feature = "watch")]
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct WatchConfig {
    /// Directory to watch.
    pub dir: PathBuf,
    /// Debounce window for coalescing rapid-fire editor saves.
    /// Defaults to [`DEFAULT_DEBOUNCE_MS`].
    pub debounce: std::time::Duration,
}

#[cfg(feature = "watch")]
impl WatchConfig {
    /// Construct a [`WatchConfig`] for `dir` with the default
    /// debounce window.
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self {
            dir: dir.into(),
            debounce: std::time::Duration::from_millis(DEFAULT_DEBOUNCE_MS),
        }
    }
}

/// Start a [`notify`]-backed watch loop on `config.dir`, publishing
/// every change through `manager`. Returns a [`tokio::task::JoinHandle`]
/// — drop it (or call `abort`) to stop the watcher.
///
/// The watcher first runs [`boot_walk`] so the in-memory state
/// matches whatever's on disk at startup, then subscribes to
/// file-system notifications.
///
/// Only available with the `watch` cargo feature.
#[cfg(feature = "watch")]
pub async fn watch_dir(
    manager: Arc<DefinitionManager>,
    config: WatchConfig,
) -> Result<tokio::task::JoinHandle<()>, WatchError> {
    use std::collections::HashMap;
    use std::sync::Mutex;

    use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};

    // Boot walk first so the path → flow_id map is populated.
    let initial = boot_walk(&manager, &config.dir).await;
    let mapping: Arc<Mutex<HashMap<PathBuf, FlowId>>> =
        Arc::new(Mutex::new(initial.into_iter().collect()));

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<FileEvent>();

    // Background blocking watcher. `notify` calls the handler from a
    // dedicated thread; we forward into the async channel.
    let tx_for_watcher = tx.clone();
    let mut watcher: RecommendedWatcher = notify::recommended_watcher(
        move |res: notify::Result<notify::Event>| {
            let Ok(event) = res else {
                return;
            };
            for path in event.paths {
                let emitted = match event.kind {
                    EventKind::Create(_) | EventKind::Modify(_) => {
                        Some(FileEvent::Upsert(path))
                    }
                    EventKind::Remove(_) => Some(FileEvent::Remove(path)),
                    _ => None,
                };
                if let Some(ev) = emitted {
                    let _ = tx_for_watcher.send(ev);
                }
            }
        },
    )
    .map_err(|e| WatchError::Io {
        path: config.dir.clone(),
        error: e.to_string(),
    })?;

    watcher
        .watch(&config.dir, RecursiveMode::NonRecursive)
        .map_err(|e| WatchError::Io {
            path: config.dir.clone(),
            error: e.to_string(),
        })?;

    let debounce = config.debounce;
    let handle = tokio::spawn(async move {
        // Keep the watcher alive for the task lifetime.
        let _watcher = watcher;
        // Per-path coalescing: a save that fires N events within
        // `debounce` collapses to one publish.
        let mut pending: HashMap<PathBuf, FileEvent> = HashMap::new();
        loop {
            tokio::select! {
                maybe = rx.recv() => {
                    match maybe {
                        Some(ev) => {
                            let key = match &ev {
                                FileEvent::Upsert(p) | FileEvent::Remove(p) => p.clone(),
                            };
                            pending.insert(key, ev);
                            // Drain any back-to-back events up to debounce.
                            let flush_at = tokio::time::Instant::now() + debounce;
                            loop {
                                tokio::select! {
                                    next = rx.recv() => {
                                        let Some(next_ev) = next else { return; };
                                        let key = match &next_ev {
                                            FileEvent::Upsert(p) | FileEvent::Remove(p) => p.clone(),
                                        };
                                        pending.insert(key, next_ev);
                                    }
                                    _ = tokio::time::sleep_until(flush_at) => break,
                                }
                            }
                            // Flush.
                            for (_path, event) in pending.drain() {
                                let map = mapping.clone();
                                // Track Upserts in the mapping so a
                                // subsequent Remove can find the
                                // flow_id.
                                if let FileEvent::Upsert(ref p) = event {
                                    if let Ok(parsed) = parse_flow_file(p) {
                                        map.lock()
                                            .expect("flow-watch mapping mutex poisoned")
                                            .insert(p.clone(), parsed.flow_id);
                                    }
                                }
                                let lookup = {
                                    let map = map.clone();
                                    move |p: &Path| -> Option<FlowId> {
                                        let mut guard =
                                            map.lock().expect("mapping mutex poisoned");
                                        guard.remove(p)
                                    }
                                };
                                let _ = apply_file_event(&manager, event, lookup).await;
                            }
                        }
                        None => return,
                    }
                }
            }
        }
    });
    Ok(handle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn write(path: &Path, contents: &str) {
        let mut f = std::fs::File::create(path).unwrap();
        f.write_all(contents.as_bytes()).unwrap();
    }

    #[test]
    fn parses_json_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("a.json");
        write(
            &path,
            r#"{"flow_id": "examples.watch.a", "nodes": [], "links": []}"#,
        );
        let parsed = parse_flow_file(&path).unwrap();
        assert_eq!(parsed.flow_id.as_str(), "examples.watch.a");
    }

    #[cfg(feature = "watch")]
    #[test]
    fn parses_yaml_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("a.yaml");
        write(
            &path,
            "flow_id: examples.watch.a\nnodes: []\nlinks: []\n",
        );
        let parsed = parse_flow_file(&path).unwrap();
        assert_eq!(parsed.flow_id.as_str(), "examples.watch.a");
    }

    #[test]
    fn missing_flow_id_errors() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("a.json");
        write(&path, r#"{"nodes": [], "links": []}"#);
        let err = parse_flow_file(&path).unwrap_err();
        assert!(matches!(err, WatchError::MissingFlowId { .. }));
    }

    #[test]
    fn unsupported_extension_errors() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("a.toml");
        write(&path, "");
        let err = parse_flow_file(&path).unwrap_err();
        assert!(matches!(err, WatchError::UnsupportedExtension { .. }));
    }
}
