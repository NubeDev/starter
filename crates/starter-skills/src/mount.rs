//! On-mount resource verification (Phase 4b).
//!
//! The `ai-agent` node body calls into this module **at mount time** —
//! immediately before it would hand a skill resource to the model — so
//! the byte-stream the model sees is the same byte-stream the
//! [`crate::SkillRegistry`] hashed when the [`SkillSelection`] was
//! frozen at `FlowRunner::start`.
//!
//! ## Why this exists
//!
//! [`crate::SkillRegistry`] freezes a per-resource
//! [`ResourceRef::content_hash`] at selection time. The bytes
//! themselves stay on disk — they are not copied into the
//! [`SkillSelection`], because copying them would defeat the
//! "parse-once, mount-from-disk" memory posture.
//!
//! A racing operator-driven [`crate::SkillRegistry::reload`] can
//! therefore swap the on-disk bytes underneath an in-flight run. The
//! selection itself is immutable, but without an on-mount hash check
//! the model would silently read the new bytes. R-skills-7 says that
//! must not happen: drifted bytes are quarantined bytes, and an
//! in-flight run keeps the bytes it selected with or aborts. This
//! module implements the abort half.
//!
//! ## How it works
//!
//! [`read_and_verify`] takes a [`ResourceRef`] (the
//! [`SkillSelection::Selected`] entries the node received), resolves
//! the `file://` URI to an absolute path, reads the bytes, applies
//! the **same** text/binary classification + line-ending
//! normalisation as [`crate::approval::hash_bundle`], hashes with
//! `blake3`, and compares against the frozen
//! [`ResourceRef::content_hash`]. On mismatch the function returns
//! [`ResourceMountError::HashMismatch`] and the bytes are **not**
//! returned — the node has nothing to mount, the run aborts.
//!
//! [`SkillSelection`]: starter_flow_spi::skill::SkillSelection
//! [`SkillSelection::Selected`]: starter_flow_spi::skill::SkillSelection::Selected
//! [`ResourceRef`]: starter_flow_spi::skill::ResourceRef
//! [`ResourceRef::content_hash`]: starter_flow_spi::skill::ResourceRef::content_hash

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use starter_flow_spi::skill::ResourceRef;
use thiserror::Error;

use crate::approval::{is_text_path_pub, normalise_line_endings_pub};

/// Failure modes the on-mount verification can surface.
///
/// Surfaced verbatim by the `ai-agent` body, which then maps each
/// variant to a typed `NodeError::Domain` so the run-telemetry
/// surface shows the failure as a structured node error rather than
/// a generic backend message.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ResourceMountError {
    /// The [`ResourceRef::uri`] did not start with `file://`. V1
    /// supports only the `file://` scheme; any other shape is
    /// rejected at parse time, but the mount path double-checks for
    /// defence in depth.
    #[error("unsupported resource scheme in `{uri}` (v1 supports file:// only)")]
    UnsupportedScheme {
        /// Offending URI verbatim.
        uri: String,
    },
    /// Reading the on-disk bytes failed.
    #[error("io error reading `{path}`: {source}")]
    Io {
        /// Absolute path the URI resolved to.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: io::Error,
    },
    /// The on-disk bytes hashed to a value that does not match the
    /// frozen [`ResourceRef::content_hash`]. The run **must** abort:
    /// the bytes the model would have read are not the bytes the
    /// registry approved.
    ///
    /// This is the load-bearing arm of R-skills-7. Subsequent runs
    /// against the edited bundle see the registry's new selection
    /// (after `reload`) and proceed normally.
    #[error("skill resource hash mismatch for `{uri}`: expected {expected}, got {actual}")]
    HashMismatch {
        /// URI of the offending resource.
        uri: String,
        /// Hash frozen into the [`ResourceRef`] at selection time.
        expected: String,
        /// Hash computed from the bytes currently on disk.
        actual: String,
    },
}

/// Read a [`ResourceRef`] off disk, verify its hash against the
/// frozen [`ResourceRef::content_hash`], and return the bytes the
/// caller should mount.
///
/// On success the returned `Vec<u8>` is the **post-normalisation**
/// byte sequence (line-ending-normalised for text resources, raw for
/// binary). That is the byte sequence the hash was computed over, so
/// it is the canonical form the model should see.
pub fn read_and_verify(resource: &ResourceRef) -> Result<Vec<u8>, ResourceMountError> {
    let path = uri_to_path(&resource.uri)?;
    let raw = fs::read(&path).map_err(|source| ResourceMountError::Io {
        path: path.clone(),
        source,
    })?;
    let normalised = if is_text_path_pub(&resource.uri) {
        normalise_line_endings_pub(&raw)
    } else {
        raw
    };
    let actual = blake3::hash(&normalised).to_hex().to_string();
    if actual != resource.content_hash {
        return Err(ResourceMountError::HashMismatch {
            uri: resource.uri.clone(),
            expected: resource.content_hash.clone(),
            actual,
        });
    }
    Ok(normalised)
}

/// Resolve a `file://` URI to an absolute [`PathBuf`].
///
/// V1 accepts only the `file://` scheme; the post-prefix portion is
/// taken verbatim as a filesystem path. Empty / non-`file://` URIs
/// surface [`ResourceMountError::UnsupportedScheme`].
fn uri_to_path(uri: &str) -> Result<PathBuf, ResourceMountError> {
    let rest =
        uri.strip_prefix("file://")
            .ok_or_else(|| ResourceMountError::UnsupportedScheme {
                uri: uri.to_owned(),
            })?;
    Ok(Path::new(rest).to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    struct TempDir(PathBuf);
    impl TempDir {
        fn new(tag: &str) -> Self {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            let base = std::env::temp_dir().join(format!(
                "starter-skills-mount-{tag}-{}-{nanos}",
                std::process::id()
            ));
            let _ = fs::remove_dir_all(&base);
            fs::create_dir_all(&base).unwrap();
            Self(base)
        }
        fn path(&self) -> &Path {
            &self.0
        }
    }
    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn write_bytes(p: &Path, bytes: &[u8]) {
        let mut f = fs::File::create(p).unwrap();
        f.write_all(bytes).unwrap();
    }

    #[test]
    fn round_trip_text_resource_hash_matches() {
        let tmp = TempDir::new("ok");
        let p = tmp.path().join("greeting.md");
        write_bytes(&p, b"hello\r\nworld\r\n");

        // The hash is computed over the *normalised* bytes — that is
        // how `resource_refs` in the registry produces it, so mirror
        // the same algorithm here.
        let normalised = normalise_line_endings_pub(b"hello\r\nworld\r\n");
        let expected = blake3::hash(&normalised).to_hex().to_string();

        let uri = format!("file://{}", p.display());
        let resource = ResourceRef::new(uri.clone(), expected);
        let bytes = read_and_verify(&resource).expect("hash matches");
        assert_eq!(bytes, normalised);
    }

    #[test]
    fn hash_mismatch_surfaces_typed_error_after_edit() {
        let tmp = TempDir::new("mismatch");
        let p = tmp.path().join("style.md");
        write_bytes(&p, b"H1 body\n");
        let normalised_h1 = normalise_line_endings_pub(b"H1 body\n");
        let h1 = blake3::hash(&normalised_h1).to_hex().to_string();

        // Edit the bytes on disk between selection-time and mount.
        write_bytes(&p, b"H2 body -- drifted\n");

        let uri = format!("file://{}", p.display());
        let resource = ResourceRef::new(uri.clone(), h1.clone());
        let err = read_and_verify(&resource).expect_err("expected mismatch");
        match err {
            ResourceMountError::HashMismatch {
                expected, actual, ..
            } => {
                assert_eq!(expected, h1);
                assert_ne!(actual, h1);
            }
            other => panic!("expected HashMismatch, got {other:?}"),
        }
    }

    #[test]
    fn non_file_scheme_is_rejected() {
        let resource = ResourceRef::new("s3://bucket/obj", "deadbeef");
        let err = read_and_verify(&resource).expect_err("expected unsupported scheme");
        assert!(matches!(err, ResourceMountError::UnsupportedScheme { .. }));
    }
}
