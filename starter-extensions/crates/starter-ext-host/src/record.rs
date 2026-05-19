//! [`ExtensionRecord`] — one entry in the registry.
//!
//! Records exist for both successfully-validated extensions (`state =
//! Validated`) and extensions whose manifest failed to parse or
//! validate (`state = Failed`, `failure` set). The bad-manifest smoke
//! test (SCOPE "Bad manifest is isolated to its own extension") relies
//! on this shape: the failure is *isolated to the bad record*, not
//! the entire registry.

use std::path::PathBuf;

use starter_ext_spi::{Error, ExtensionId, LifecycleState, Manifest};

/// One extension's full state inside the registry.
#[derive(Debug, Clone)]
pub struct ExtensionRecord {
    /// Validated reverse-DNS id. Present for both successful and failed
    /// records when the manifest at least parsed far enough to surface
    /// an id; `None` when the manifest itself failed at deserialise
    /// time (in which case `id_hint` carries the directory name so an
    /// operator can still locate the bundle).
    pub id: Option<ExtensionId>,

    /// The bundle directory name. Always populated — even when the
    /// manifest failed to parse — so admin UIs can point at a path.
    pub id_hint: String,

    /// Absolute path to the bundle directory.
    pub bundle_dir: PathBuf,

    /// Lifecycle state after `Loader::commit`. `Validated` on success,
    /// `Failed` when the manifest or a semantic check rejected the
    /// extension.
    pub state: LifecycleState,

    /// Parsed manifest. `None` when parsing failed.
    pub manifest: Option<Manifest>,

    /// The error that put the record in `Failed`, if any.
    pub failure: Option<Error>,
}

impl ExtensionRecord {
    /// `true` when the record's manifest passed every check.
    pub fn is_validated(&self) -> bool {
        matches!(self.state, LifecycleState::Validated)
    }

    /// `true` when the record is in the terminal `Failed` state.
    pub fn is_failed(&self) -> bool {
        matches!(self.state, LifecycleState::Failed)
    }
}
