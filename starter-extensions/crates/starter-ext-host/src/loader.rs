//! Bundle discovery and two-phase commit.
//!
//! [`Loader::scan`] walks the configured extensions root one directory
//! deep — each immediate child whose name matches an extension id is a
//! candidate. Per SCOPE.md "Extension bundle on-disk convention" the
//! default location is `$XDG_DATA_HOME/<binary>/extensions/<id>/`, but
//! the loader is agnostic to that: it takes whatever path the consumer
//! configured.
//!
//! Per-candidate errors do **not** short-circuit. A typo in one
//! `block.yaml` produces a `Failed` record for that bundle and leaves
//! every other bundle untouched — this is the SCOPE "Bad manifest is
//! isolated to its own extension" smoke test made executable.
//!
//! [`Loader::validate_all`] runs the namespace + capability checks (see
//! [`crate::validate`]) on every candidate that parsed, then
//! [`Loader::commit`] performs the *two-phase* commit: every record
//! lands in the registry in one shot, success or failure, never a half-
//! built state.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use starter_ext_spi::{Error, ExtensionId, LifecycleState, Manifest};

use crate::{record::ExtensionRecord, registry::ExtensionRegistry, validate::validate_manifest};

/// Per-candidate result of the discovery walk.
struct ScanCandidate {
    /// Directory name (used as `id_hint` even when parsing fails).
    dir_name: String,
    /// Absolute path to the bundle directory.
    bundle_dir: PathBuf,
    /// Result of reading + deserialising `block.yaml`. The parse error is
    /// kept as-is so the failed record surfaces the precise reason an
    /// operator can act on.
    parsed: Result<Manifest, Error>,
}

/// Walks the extensions root, parses each `block.yaml`, validates the
/// passing ones, and returns an [`ExtensionRegistry`] containing every
/// candidate (good and bad) at `Validated` / `Failed` respectively.
pub struct Loader {
    candidates: Vec<ScanCandidate>,
}

/// Aggregate counts surfaced after `commit` so a consumer can log a
/// "loaded N extensions (M failed)" line without iterating records itself.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LoaderOutcome {
    /// Number of records that landed in [`LifecycleState::Validated`].
    pub validated: usize,
    /// Number of records that landed in [`LifecycleState::Failed`].
    pub failed: usize,
}

impl Loader {
    /// Walk `root` one level deep. Every immediate child directory becomes
    /// a candidate. Children that are not directories, do not contain a
    /// `block.yaml`, or whose `block.yaml` fails to read/parse all surface
    /// as failed candidates — never as host-level errors.
    ///
    /// `scan` itself does not validate the manifest beyond `serde`'s
    /// deserialisation: the namespace + capability checks happen in
    /// [`Self::validate_all`].
    pub fn scan(root: &Path) -> Self {
        let mut candidates = Vec::new();
        let entries = match fs::read_dir(root) {
            Ok(it) => it,
            Err(_) => {
                // A missing extensions root is a valid empty load — not an
                // error. SCOPE explicitly treats the root as optional; a
                // consumer that does not ship extensions still boots.
                return Self { candidates };
            }
        };
        for entry in entries.flatten() {
            let file_type = match entry.file_type() {
                Ok(t) => t,
                Err(_) => continue,
            };
            if !file_type.is_dir() {
                continue;
            }
            let bundle_dir = entry.path();
            let dir_name = entry.file_name().to_string_lossy().into_owned();
            let manifest_path = bundle_dir.join("block.yaml");
            // A directory without a `block.yaml` is **not a bundle**
            // — silently skip it. This filters out incidental siblings
            // (a stray `target/` cargo cache, `node_modules/`, `.git/`,
            // a dev workspace's `Cargo.toml` toolbox, etc.) that would
            // otherwise surface as bogus `Failed` extension records in
            // `GET /extensions`. A *present* but unreadable/malformed
            // manifest still produces a Failed record — that signals a
            // real broken bundle the operator wants to see.
            let parsed = match fs::read_to_string(&manifest_path) {
                Ok(s) => serde_yaml::from_str::<Manifest>(&s)
                    .map_err(|e| Error::manifest(format!("{}: {}", manifest_path.display(), e))),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                Err(e) => Err(Error::manifest(format!(
                    "{}: {}",
                    manifest_path.display(),
                    e
                ))),
            };
            candidates.push(ScanCandidate {
                dir_name,
                bundle_dir,
                parsed,
            });
        }
        Self { candidates }
    }

    /// Consume the loader and run validation across every candidate.
    /// Returns a `Vec<ExtensionRecord>` — each entry is either
    /// `Validated` (manifest passed every check) or `Failed` (with a
    /// human-readable `failure` reason).
    ///
    /// Validation is *complete* before any commit happens — this is the
    /// "validate every candidate, then register atomically" half of the
    /// two-phase commit (SCOPE.md "Decisions made: two-phase manifest
    /// commit").
    pub fn validate_all(self) -> Vec<ExtensionRecord> {
        let mut records: Vec<ExtensionRecord> = Vec::with_capacity(self.candidates.len());
        let mut seen_ids: HashSet<ExtensionId> = HashSet::new();

        for candidate in self.candidates {
            let ScanCandidate {
                dir_name,
                bundle_dir,
                parsed,
            } = candidate;

            match parsed {
                Err(e) => records.push(ExtensionRecord {
                    id: None,
                    id_hint: dir_name,
                    bundle_dir,
                    state: LifecycleState::Failed,
                    manifest: None,
                    failure: Some(e),
                }),
                Ok(manifest) => {
                    let id = manifest.id.clone();
                    // Schema version compatibility. v0.1 only understands v=1;
                    // newer manifests must be refused with a clear reason so
                    // operators see "unsupported manifest schema", not a
                    // missing-field surprise.
                    if let Some(err) = check_schema_version(&manifest) {
                        records.push(ExtensionRecord {
                            id: Some(id.clone()),
                            id_hint: dir_name,
                            bundle_dir,
                            state: LifecycleState::Failed,
                            manifest: Some(manifest),
                            failure: Some(err),
                        });
                        continue;
                    }
                    // R4 + R6 semantic checks.
                    if let Err(err) = validate_manifest(&manifest) {
                        records.push(ExtensionRecord {
                            id: Some(id.clone()),
                            id_hint: dir_name,
                            bundle_dir,
                            state: LifecycleState::Failed,
                            manifest: Some(manifest),
                            failure: Some(err),
                        });
                        continue;
                    }
                    // Id uniqueness across candidates.
                    if !seen_ids.insert(id.clone()) {
                        records.push(ExtensionRecord {
                            id: Some(id.clone()),
                            id_hint: dir_name,
                            bundle_dir,
                            state: LifecycleState::Failed,
                            manifest: Some(manifest),
                            failure: Some(Error::validation(format!(
                                "duplicate extension id {:?}: another bundle in the same root \
                                 already claimed it",
                                id.as_str()
                            ))),
                        });
                        continue;
                    }
                    records.push(ExtensionRecord {
                        id: Some(id),
                        id_hint: dir_name,
                        bundle_dir,
                        state: LifecycleState::Validated,
                        manifest: Some(manifest),
                        failure: None,
                    });
                }
            }
        }
        records
    }

    /// All-or-nothing registration. Every record from [`Self::validate_all`]
    /// is inserted into `registry` in one pass; the registry never lands
    /// in a partial state.
    ///
    /// The registry is mutated in place (per SCOPE.md, the consumer hands
    /// a `&mut ExtensionRegistry` to the loader before sealing it with
    /// [`ExtensionRegistry::seal`]).
    pub fn commit(
        records: Vec<ExtensionRecord>,
        registry: &mut ExtensionRegistry,
    ) -> LoaderOutcome {
        let mut outcome = LoaderOutcome::default();
        let mut by_id: HashMap<String, ExtensionRecord> = HashMap::new();

        // Build the new state in a local map first so a panic inside the
        // loop cannot leave `registry` half-populated. The registry only
        // sees the final, complete picture.
        for record in records {
            match record.state {
                LifecycleState::Validated => outcome.validated += 1,
                LifecycleState::Failed => outcome.failed += 1,
                // `scan` + `validate_all` only emit Validated / Failed.
                // Any other state here is a programming error worth
                // surfacing loudly rather than silently coercing.
                other => panic!(
                    "starter-ext-host: Loader::commit received unexpected state {:?}; \
                     scan/validate must only produce Validated or Failed",
                    other
                ),
            }
            let key = record
                .id
                .as_ref()
                .map(|i| i.as_str().to_owned())
                .unwrap_or_else(|| format!("<unparsed:{}>", record.id_hint));
            by_id.insert(key, record);
        }

        registry.install(by_id);
        outcome
    }
}

/// `v: 1` is the only manifest schema the v0.1 host understands. Newer
/// majors are refused with a typed reason (SCOPE.md "SDK ↔ host version
/// compatibility").
fn check_schema_version(m: &Manifest) -> Option<Error> {
    if m.v != starter_ext_spi::manifest::MANIFEST_VERSION {
        return Some(Error::manifest(format!(
            "unsupported manifest schema: this host understands v={} but the bundle declares v={}",
            starter_ext_spi::manifest::MANIFEST_VERSION,
            m.v
        )));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn write_bundle(root: &Path, name: &str, manifest: &str) {
        let dir = root.join(name);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("block.yaml"), manifest).unwrap();
    }

    const GOOD: &str = r#"
v: 1
id: com.acme.good
version: 0.1.0
display_name: "G"
runtime: { kind: builtin, crate_name: good }
contributes:
  tools:
    - id: com.acme.good.echo
      input_schema: a.json
      output_schema: b.json
      description_file: c.md
"#;

    #[test]
    fn empty_root_is_legal() {
        let tmp = tempdir().unwrap();
        let recs = Loader::scan(tmp.path()).validate_all();
        assert!(recs.is_empty());
    }

    /// Sibling directories without a `block.yaml` (a stray cargo
    /// `target/`, `node_modules/`, `.git/`, etc.) must not surface as
    /// `Failed` extension records — they are not bundles at all.
    #[test]
    fn directories_without_block_yaml_are_skipped() {
        let tmp = tempdir().unwrap();
        write_bundle(tmp.path(), "com.acme.good", GOOD);
        // Cargo build-cache lookalike: a directory with arbitrary
        // contents but no `block.yaml`.
        let target = tmp.path().join("target");
        std::fs::create_dir(&target).unwrap();
        std::fs::write(
            target.join("CACHEDIR.TAG"),
            b"Signature: 8a477f597d28d172789f06886806bc55",
        )
        .unwrap();
        std::fs::create_dir(target.join("release")).unwrap();
        // `.git`-style hidden dir.
        std::fs::create_dir(tmp.path().join(".dotdir")).unwrap();

        let recs = Loader::scan(tmp.path()).validate_all();
        assert_eq!(recs.len(), 1, "only the bundle with block.yaml is recorded");
        assert!(recs[0].is_validated());
        assert_eq!(recs[0].id_hint, "com.acme.good");
    }

    #[test]
    fn missing_root_is_legal() {
        let tmp = tempdir().unwrap();
        let nonexistent = tmp.path().join("does-not-exist");
        let recs = Loader::scan(&nonexistent).validate_all();
        assert!(recs.is_empty());
    }

    #[test]
    fn scans_validates_and_commits_one_good_bundle() {
        let tmp = tempdir().unwrap();
        write_bundle(tmp.path(), "com.acme.good", GOOD);
        let recs = Loader::scan(tmp.path()).validate_all();
        assert_eq!(recs.len(), 1);
        assert!(recs[0].is_validated());

        let mut reg = ExtensionRegistry::new();
        let out = Loader::commit(recs, &mut reg);
        assert_eq!(out.validated, 1);
        assert_eq!(out.failed, 0);
    }

    /// The SCOPE "Bad manifest is isolated to its own extension" smoke test.
    /// One broken manifest must fail *itself* and leave every sibling bundle
    /// loadable.
    #[test]
    fn bad_manifest_is_isolated_to_its_own_extension() {
        let tmp = tempdir().unwrap();
        write_bundle(tmp.path(), "com.acme.good", GOOD);
        // Top-level typo inside `deny_unknown_fields`.
        write_bundle(
            tmp.path(),
            "com.acme.broken",
            r#"
v: 1
id: com.acme.broken
version: 0.0.1
display_name: "B"
runtime: { kind: builtin, crate_name: b }
nope_unknown_top_level: true
"#,
        );

        let recs = Loader::scan(tmp.path()).validate_all();
        assert_eq!(recs.len(), 2, "both bundles must be recorded");

        let good = recs
            .iter()
            .find(|r| r.id_hint == "com.acme.good")
            .expect("good bundle present");
        let bad = recs
            .iter()
            .find(|r| r.id_hint == "com.acme.broken")
            .expect("broken bundle present");

        assert!(good.is_validated(), "good bundle must validate cleanly");
        assert!(bad.is_failed(), "broken bundle must land in Failed");
        assert!(
            bad.failure.is_some(),
            "broken bundle must carry a parseable failure reason"
        );

        let mut reg = ExtensionRegistry::new();
        let out = Loader::commit(recs, &mut reg);
        assert_eq!(out.validated, 1);
        assert_eq!(out.failed, 1);
        // Registry contains both records; the good one is queryable and
        // serves requests, the bad one is informational.
        assert!(reg.get_by_id_str("com.acme.good").is_some());
        let bad_rec = reg.list().iter().find(|r| r.id_hint == "com.acme.broken");
        assert!(bad_rec.is_some());
    }

    #[test]
    fn namespace_violation_is_isolated_not_fatal() {
        let tmp = tempdir().unwrap();
        write_bundle(tmp.path(), "com.acme.good", GOOD);
        write_bundle(
            tmp.path(),
            "com.acme.bad-ns",
            r#"
v: 1
id: com.acme.badns
version: 0.0.1
display_name: "BN"
runtime: { kind: builtin, crate_name: bn }
contributes:
  tools:
    - id: com.other.escape.tool
      input_schema: a.json
      output_schema: b.json
      description_file: c.md
"#,
        );
        let recs = Loader::scan(tmp.path()).validate_all();
        let bad = recs
            .iter()
            .find(|r| r.id_hint == "com.acme.bad-ns")
            .unwrap();
        assert!(bad.is_failed());
        let good = recs.iter().find(|r| r.id_hint == "com.acme.good").unwrap();
        assert!(good.is_validated());
    }

    #[test]
    fn duplicate_id_across_bundles_fails_second_load() {
        let tmp = tempdir().unwrap();
        write_bundle(tmp.path(), "first", GOOD);
        write_bundle(tmp.path(), "second", GOOD);
        let recs = Loader::scan(tmp.path()).validate_all();
        let validated: Vec<_> = recs.iter().filter(|r| r.is_validated()).collect();
        let failed: Vec<_> = recs.iter().filter(|r| r.is_failed()).collect();
        assert_eq!(validated.len(), 1);
        assert_eq!(failed.len(), 1);
        assert!(failed[0]
            .failure
            .as_ref()
            .unwrap()
            .to_string()
            .contains("duplicate"));
    }

    #[test]
    fn unsupported_schema_version_is_refused_with_a_typed_reason() {
        let tmp = tempdir().unwrap();
        write_bundle(
            tmp.path(),
            "com.acme.future",
            r#"
v: 2
id: com.acme.future
version: 0.0.1
display_name: "F"
runtime: { kind: builtin, crate_name: f }
"#,
        );
        let recs = Loader::scan(tmp.path()).validate_all();
        assert_eq!(recs.len(), 1);
        assert!(recs[0].is_failed());
        let msg = recs[0].failure.as_ref().unwrap().to_string();
        assert!(msg.contains("unsupported manifest schema"));
    }
}
