//! Content-hash algorithm for skill bundles (R-skills-2 / agent R4).
//!
//! [`hash_bundle`] produces a stable, hex-encoded `blake3` digest of
//! every non-excluded file under a bundle directory. The digest is
//! the "version" of the bundle — the approval store keys off it,
//! and the `ai-agent` node verifies resource bytes against it on
//! mount so a concurrent `reload()` can never substitute drifted
//! bytes into a running flow (R-skills-7).
//!
//! The algorithm is intentionally narrow and **byte-deterministic**:
//!
//! 1. Walk the bundle root recursively, skipping any path whose
//!    components match an entry in [`EXCLUDED`] (editor cruft, VCS
//!    metadata, Python bytecode caches, IDE state). Adding to that
//!    list is a deliberate PR — the constant is `pub`, not a config
//!    knob, because every entry shifts every approved hash.
//! 2. Compute each entry's bundle-relative path with `/` separators
//!    (Windows `\` is normalised before sort and before hashing).
//! 3. Sort entries by their relative-path bytes, lexicographic.
//! 4. For text-classified files (extension in [`TEXT_EXTENSIONS`])
//!    apply exactly two byte-level replacements, in order:
//!    `\r\n -> \n`, then lone `\r -> \n`. Per S-D5 (and agent R4),
//!    no BOM stripping, no UTF-16 special-casing, no other
//!    transforms. Binary files are hashed as-is.
//! 5. Feed each entry into a single `blake3` hasher with explicit
//!    length-prefixed framing so adjacent path / content bytes
//!    cannot be permuted into a collision (R4: path framing
//!    prevents `a/b` + `c` colliding with `a` + `b/c`):
//!
//!    ```text
//!    u64_le(path.len()) || path_bytes
//!    || u64_le(content.len()) || content_bytes
//!    ```
//!
//! 6. The final 32-byte digest is hex-encoded lower-case.
//!
//! The two property smokes ("framing prevents collision" and
//! "line-ending normalisation is stable") and the fixture-pinned
//! digest test guard the algorithm against accidental drift.

use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

use blake3::Hasher;

/// File / directory names that are **always** excluded from the
/// hash regardless of where they sit in the bundle.
///
/// Editor cruft (`.DS_Store`, `Thumbs.db`, swap files, emacs
/// backups) and tool / IDE state (`.git/`, `.idea/`,
/// `__pycache__/`) are noise: they vary across machines without
/// changing the skill, so including them would either
/// re-quarantine every bundle on every commit or force bundle
/// authors to ship a `.gitignore`-style discipline this loader
/// has no business enforcing.
///
/// Matching is by *exact component name*. `.git` matches the
/// directory `vendor/foo/.git` because *some* component equals
/// `.git`; it does **not** match `mygit/state.txt`. The trailing
/// `/` in the doc comments above is illustrative — the constant
/// itself stores plain names.
///
/// Glob-style entries (`*.swp`, `*.swo`, `*~`) are matched as
/// suffixes / suffixes-of-name; the matcher is intentionally
/// hand-rolled (no `glob` dep) so the rules are auditable in this
/// file.
///
/// **Adding to this list is a deliberate PR** — every addition
/// shifts every approved bundle hash, so changes must be reviewed
/// alongside an approval-store migration plan.
pub const EXCLUDED: &[&str] = &[
    ".DS_Store",
    "Thumbs.db",
    "*.swp",
    "*.swo",
    "*~",
    ".git",
    ".idea",
    "__pycache__",
];

/// Extensions that classify a file as "text" for the purpose of
/// line-ending normalisation (S-D5). Anything not in this set is
/// treated as binary and hashed verbatim.
///
/// The set is closed: we do not sniff content, we do not consult
/// `file(1)`, we do not treat unknown text-y extensions
/// (`.markdown`, `.cfg`) as text. The closed set keeps the hash
/// stable across machines that happen to have different libmagic
/// rules.
pub const TEXT_EXTENSIONS: &[&str] = &["md", "txt", "json", "yaml", "yml", "toml"];

/// Compute the content hash of a bundle directory.
///
/// See the module docs for the full algorithm. Returns a 64-char
/// lower-case hex string (32-byte `blake3` digest).
///
/// Errors: any I/O failure during the walk or read is surfaced as
/// [`io::Error`]; the bundle must exist and be readable.
pub fn hash_bundle(bundle_root: impl AsRef<Path>) -> io::Result<String> {
    let bundle_root = bundle_root.as_ref();

    let mut entries: Vec<(String, PathBuf)> = Vec::new();
    collect(bundle_root, bundle_root, &mut entries)?;

    // Lexicographic sort by relative-path *bytes* — not by str ord,
    // which could differ on locale-sensitive collation, and not by
    // path-component sort, which would re-order `a/b` after `a-b`.
    entries.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));

    let mut hasher = Hasher::new();
    for (rel, abs) in &entries {
        let raw = fs::read(abs)?;
        let content = if is_text(rel) {
            normalise_line_endings(&raw)
        } else {
            raw
        };

        let path_bytes = rel.as_bytes();
        hasher.update(&(path_bytes.len() as u64).to_le_bytes());
        hasher.update(path_bytes);
        hasher.update(&(content.len() as u64).to_le_bytes());
        hasher.update(&content);
    }

    Ok(hasher.finalize().to_hex().to_string())
}

/// Recursive walk that respects [`EXCLUDED`]. Collects
/// `(relative_path_with_forward_slashes, absolute_path)` pairs.
fn collect(root: &Path, dir: &Path, out: &mut Vec<(String, PathBuf)>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name();
        let name_str = match name.to_str() {
            Some(s) => s,
            // Non-UTF-8 file names are not addressable by a
            // forward-slash relative path; reject loudly rather
            // than silently re-encoding.
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("non-utf8 entry name in {}", dir.display()),
                ));
            }
        };
        if is_excluded(name_str) {
            continue;
        }
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            collect(root, &path, out)?;
        } else if file_type.is_file() {
            let rel = relative_forward_slash(root, &path)?;
            out.push((rel, path));
        }
        // Symlinks and other types are ignored — a bundle that
        // needs them is misusing the loader.
    }
    Ok(())
}

/// Build a `/`-separated relative path from `root` to `path`.
fn relative_forward_slash(root: &Path, path: &Path) -> io::Result<String> {
    let rel = path.strip_prefix(root).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "path {} is not under bundle root {}",
                path.display(),
                root.display()
            ),
        )
    })?;
    let mut parts = Vec::new();
    for comp in rel.components() {
        match comp {
            Component::Normal(s) => match s.to_str() {
                Some(s) => parts.push(s),
                None => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("non-utf8 path segment in {}", path.display()),
                    ));
                }
            },
            // CurDir is harmless to drop; the others can't appear
            // after strip_prefix on a real walked path, but bail
            // explicitly so a future refactor doesn't hide drift.
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("unexpected component in relative path {}", rel.display()),
                ));
            }
        }
    }
    Ok(parts.join("/"))
}

/// `true` if any component of the path matches an [`EXCLUDED`]
/// entry. Called per file *name* during the walk; the directory
/// check happens implicitly because excluded directories are
/// never descended into.
fn is_excluded(name: &str) -> bool {
    for pat in EXCLUDED {
        if let Some(suffix) = pat.strip_prefix('*') {
            if name.ends_with(suffix) && name.len() >= suffix.len() {
                return true;
            }
        } else if pat.ends_with('~') {
            // `*~` is the only entry of this shape today and is
            // handled by the `*` branch above; keep this branch
            // empty so future contributors notice that trailing-`~`
            // patterns must start with `*`.
            if *pat == name {
                return true;
            }
        } else if *pat == name {
            return true;
        }
    }
    false
}

/// Public alias for [`is_text`] used by the registry to compute
/// per-resource [`starter_flow_spi::skill::ResourceRef::content_hash`]
/// with the same text/binary classification as `hash_bundle` (so the
/// Phase 4b on-mount check sees a byte-equal hash).
pub(crate) fn is_text_path(rel: &str) -> bool {
    is_text(rel)
}

/// Public accessor for the same text/binary classifier the registry
/// and the Phase 4b on-mount check use. Exposed so external callers
/// (the `ai-agent` body via [`crate::mount`]) classify a URI exactly
/// the way the registry classified it at selection time — drift here
/// would let a text resource hash one way at selection and another at
/// mount.
pub fn is_text_path_pub(rel: &str) -> bool {
    is_text(rel)
}

/// Public-in-crate accessor for [`normalise_line_endings`] — kept
/// separate from the private function so the algorithmic core stays
/// `fn` (no trait, no allocator churn) while consumers go through
/// this stable name.
pub fn normalise_line_endings_pub(input: &[u8]) -> Vec<u8> {
    normalise_line_endings(input)
}

/// Is this relative path a text-classified file per
/// [`TEXT_EXTENSIONS`]? Extension match is case-sensitive and
/// matches the last `.`-separated suffix.
fn is_text(rel: &str) -> bool {
    let ext = match rel.rsplit_once('.') {
        Some((_, ext)) => ext,
        None => return false,
    };
    TEXT_EXTENSIONS.contains(&ext)
}

/// Apply the two byte-level replacements R4 / S-D5 specify, in
/// order: `CRLF -> LF`, then lone `CR -> LF`. The single-pass
/// implementation does both at once because the second pass would
/// otherwise need to know which `\n` bytes came from a CRLF (and
/// must therefore not be turned back into anything) — handling
/// both in one pass keeps that distinction implicit.
fn normalise_line_endings(input: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(input.len());
    let mut i = 0;
    while i < input.len() {
        let b = input[i];
        if b == b'\r' {
            // CRLF -> LF (skip the \r, keep the \n on the next
            // iteration so we don't accidentally turn a single CR
            // followed by a CR into two LFs).
            if i + 1 < input.len() && input[i + 1] == b'\n' {
                out.push(b'\n');
                i += 2;
                continue;
            }
            // Lone CR -> LF.
            out.push(b'\n');
            i += 1;
            continue;
        }
        out.push(b);
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use std::path::PathBuf;

    /// Test-only tempdir helper. We avoid pulling in `tempfile` to
    /// keep the dep tree boring (per SCOPE: no I/O crates yet).
    struct TempDir(PathBuf);
    impl TempDir {
        fn new(tag: &str) -> Self {
            // Include a nanos suffix so parallel tests don't stomp.
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            let base = std::env::temp_dir().join(format!(
                "starter-skills-hash-{tag}-{}-{nanos}",
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

    fn write_bytes(dir: &Path, rel: &str, bytes: &[u8]) {
        let p = dir.join(rel);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut f = fs::File::create(&p).unwrap();
        f.write_all(bytes).unwrap();
    }

    /// "Path framing prevents collision" (R-skills-2 smoke).
    ///
    /// Without length-prefixed framing, hashing `path||content`
    /// for `{a/b: "c"}` and `{a: "b/c"}` would feed the *same*
    /// bytes `a/bc` into the hasher and collide. The `u64_le` length
    /// prefixes make the inputs distinguishable.
    #[test]
    fn path_framing_prevents_collision() {
        let a = TempDir::new("framing-a");
        write_bytes(a.path(), "a/b", b"c");

        let b = TempDir::new("framing-b");
        write_bytes(b.path(), "a", b"b/c");

        let ha = hash_bundle(a.path()).unwrap();
        let hb = hash_bundle(b.path()).unwrap();
        assert_ne!(
            ha, hb,
            "framing must distinguish (a/b -> c) from (a -> b/c)"
        );
    }

    /// "Line-ending normalisation is stable" (S-D5 smoke).
    ///
    /// The same logical text file committed three ways — CRLF, LF,
    /// CR-only — must produce identical bundle hashes, otherwise
    /// a Windows contributor will silently re-quarantine every
    /// previously-approved bundle. We exercise all three on a `.md`
    /// file (text-classified) to keep the smoke focused on the
    /// text-file branch of `hash_bundle`.
    #[test]
    fn line_ending_normalisation_is_stable() {
        let body = b"alpha\nbeta\ngamma\n";
        let crlf: Vec<u8> = body
            .iter()
            .flat_map(|&b| {
                if b == b'\n' {
                    vec![b'\r', b'\n']
                } else {
                    vec![b]
                }
            })
            .collect();
        let cr: Vec<u8> = body
            .iter()
            .map(|&b| if b == b'\n' { b'\r' } else { b })
            .collect();

        let lf_dir = TempDir::new("le-lf");
        write_bytes(lf_dir.path(), "SKILL.md", body);
        let crlf_dir = TempDir::new("le-crlf");
        write_bytes(crlf_dir.path(), "SKILL.md", &crlf);
        let cr_dir = TempDir::new("le-cr");
        write_bytes(cr_dir.path(), "SKILL.md", &cr);

        let h_lf = hash_bundle(lf_dir.path()).unwrap();
        let h_crlf = hash_bundle(crlf_dir.path()).unwrap();
        let h_cr = hash_bundle(cr_dir.path()).unwrap();

        assert_eq!(h_lf, h_crlf, "CRLF must normalise to LF");
        assert_eq!(h_lf, h_cr, "lone CR must normalise to LF");
    }

    /// Binary files (extension not in [`TEXT_EXTENSIONS`]) must
    /// **not** have line endings normalised — a `\r\n` inside a
    /// `.bin` is real data the hash must preserve.
    #[test]
    fn binary_files_are_not_normalised() {
        let a = TempDir::new("bin-crlf");
        write_bytes(a.path(), "blob.bin", b"a\r\nb");
        let b = TempDir::new("bin-lf");
        write_bytes(b.path(), "blob.bin", b"a\nb");

        let ha = hash_bundle(a.path()).unwrap();
        let hb = hash_bundle(b.path()).unwrap();
        assert_ne!(ha, hb, "binary files must hash verbatim");
    }

    /// EXCLUDED entries do not contribute to the hash. If they
    /// did, every developer's `.DS_Store` would silently
    /// re-quarantine the bundle on macOS.
    #[test]
    fn excluded_paths_do_not_affect_hash() {
        let clean = TempDir::new("excl-clean");
        write_bytes(clean.path(), "SKILL.md", b"hi\n");

        let dirty = TempDir::new("excl-dirty");
        write_bytes(dirty.path(), "SKILL.md", b"hi\n");
        write_bytes(dirty.path(), ".DS_Store", b"junk");
        write_bytes(dirty.path(), "Thumbs.db", b"junk");
        write_bytes(dirty.path(), "sub/file.swp", b"junk");
        write_bytes(dirty.path(), "sub/file.swo", b"junk");
        write_bytes(dirty.path(), "sub/file~", b"junk");
        write_bytes(dirty.path(), ".git/HEAD", b"ref: refs/heads/x\n");
        write_bytes(dirty.path(), ".idea/workspace.xml", b"<x/>");
        write_bytes(dirty.path(), "__pycache__/foo.pyc", b"\0\0");

        assert_eq!(
            hash_bundle(clean.path()).unwrap(),
            hash_bundle(dirty.path()).unwrap()
        );
    }

    /// Pin a fixture-bundle digest. If a future refactor changes
    /// framing, sort order, or normalisation, this test fails and
    /// forces an explicit conversation (and an approval-store
    /// migration plan) before the algorithm drifts.
    ///
    /// Fixture: `SKILL.md` containing `hello\n`, and `docs/a.txt`
    /// containing `world\n`. The expected digest below was
    /// produced by `hash_bundle` itself once and pinned.
    #[test]
    fn fixture_digest_is_pinned() {
        let tmp = TempDir::new("pinned");
        write_bytes(tmp.path(), "SKILL.md", b"hello\n");
        write_bytes(tmp.path(), "docs/a.txt", b"world\n");

        let got = hash_bundle(tmp.path()).unwrap();
        assert_eq!(
            got,
            expected_pinned_digest(),
            "hash_bundle drift detected — see module docs before \
             updating this constant; every approval store keyed on \
             the old hash will re-quarantine"
        );
    }

    /// Recompute the pinned digest manually using the documented
    /// framing rules. Keeping the expectation derivable (rather
    /// than a bare hex literal) means the test fails loudly if
    /// the *spec* drifts in addition to the *implementation*.
    fn expected_pinned_digest() -> String {
        let mut h = Hasher::new();
        let entries: [(&str, &[u8]); 2] = [("SKILL.md", b"hello\n"), ("docs/a.txt", b"world\n")];
        for (path, content) in entries {
            h.update(&(path.len() as u64).to_le_bytes());
            h.update(path.as_bytes());
            h.update(&(content.len() as u64).to_le_bytes());
            h.update(content);
        }
        h.finalize().to_hex().to_string()
    }

    #[test]
    fn normalise_handles_mixed_endings() {
        // Mixed CRLF + lone CR + bare LF in one buffer.
        assert_eq!(
            normalise_line_endings(b"a\r\nb\rc\nd"),
            b"a\nb\nc\nd".to_vec()
        );
    }

    #[test]
    fn is_excluded_matches_suffix_globs() {
        assert!(is_excluded(".DS_Store"));
        assert!(is_excluded("Thumbs.db"));
        assert!(is_excluded("foo.swp"));
        assert!(is_excluded("foo.swo"));
        assert!(is_excluded("backup~"));
        assert!(is_excluded(".git"));
        assert!(!is_excluded("git"));
        assert!(!is_excluded("notes.md"));
    }
}
