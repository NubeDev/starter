# Application data root + safe uninstall

> **Tier:** plan, not system-as-it-is. Lives in `docs/scope/` per
> [HOW-TO-CODE.md §0a](../../HOW-TO-CODE.md). Source code must not
> reference this file — once landed, its design moves into
> `docs/design/extensions/` and code links there.

**Status:** proposal
**Author:** rubix-agent
**Date:** 2026-05-29

## Problem

`DELETE /api/v1/extensions/<id>?purge=true` deleted a developer's
extension *source tree* at
[`rubix/extensions/com.nubeio.rubixos/`](../../../extensions/) — code
that is checked into git, edited by hand, and loaded in-place by the
agent.

The root cause is a single conflated path. Today `extensions_dir`
serves two completely different roles:

1. **Dev source trees.** `rubix/extensions/com.nubeio.rubixos/` is the
   user's working copy. It is scanned in-place at boot
   ([`boot/extensions.rs:187-198`](../../../crates/rubix-agent/src/boot/extensions.rs#L187-L198))
   and is not produced by the install handler.
2. **Installed bundles.** `POST /api/v1/extensions/install` unpacks
   uploaded tarballs into the same directory
   ([`lifecycle.rs:188-232`](../../../../starter-extensions/crates/starter-ext-server/src/lifecycle.rs#L188-L232)).

The uninstall handler does not distinguish between them. It computes
`bundle_dir = extensions_dir.join(sanitize_dirname(&id))` and calls
`std::fs::remove_dir_all(&bundle_dir)`
([`lifecycle.rs:298`](../../../../starter-extensions/crates/starter-ext-server/src/lifecycle.rs#L298),
[`:339`](../../../../starter-extensions/crates/starter-ext-server/src/lifecycle.rs#L339)) —
no symlink check, no origin check, no marker file check. Whatever lives
at that path is destroyed.

There is also a deeper symptom: across the whole starter + rubix tree
there is no shared notion of *where application data lives*. Different
subsystems each invent their own answer
([`starter-secrets-file/src/store.rs:65`](../../../../crates/starter-secrets-file/src/store.rs#L65)
defaults to `$XDG_DATA_HOME/<binary>/`,
[`rubix-agent/src/boot/config.rs:181`](../../../crates/rubix-agent/src/boot/config.rs#L181)
defaults to `rubix/extensions`, etc.). The fix for uninstall is also
the fix for a missing piece of the platform: a single, layered data
root that every component reads from.

## Principles

1. **Starter owns the mechanism; rubix owns the policy.** Path
   resolution and the dev-vs-installed distinction live in a new
   `starter-paths` crate. Rubix only chooses the binary name and
   composes consumers.
2. **Source code is never deleted by the runtime.** Dev-mounted
   extensions are loaded read-only. Uninstall on a dev bundle never
   touches files — it disables and purges *data*, never source.
3. **One data root, many subdirs.** Every component that needs to
   write durable state asks `Paths` for its subdir. No more ad-hoc
   `dirs::data_dir()` calls scattered across crates.
4. **The wire format remains stable.** This is an internal refactor;
   no breaking changes to `/api/v1/extensions/*` request/response
   shapes. Uninstall on a dev bundle changes its *behavior* (now safe),
   not its endpoint.

## Layout

```
$DATA_ROOT/                       ← single root, resolved once at boot
├── config/                       ← writable config snapshots (future)
├── extensions/                   ← INSTALLED bundles (writable, deletable)
│   └── com.example.foo/          ← unpacked tarballs land here
├── rubix/                        ← rubix-specific durable state
│   ├── warehouse/                ← SQLite/Postgres if file-backed
│   ├── ui-cache/                 ← UI bundle ETag + bytes cache
│   ├── secrets/                  ← what starter-secrets-file owns
│   └── enablement.db             ← extension enablement store
└── logs/                         ← rotating log files (future)
```

Defaults, in order of precedence:

1. `--data-root <path>` CLI flag (or equivalent on each consumer).
2. `$RUBIX_DATA_ROOT` env var (or `$<APP>_DATA_ROOT` for other
   consumers).
3. `$XDG_DATA_HOME/<binary>/` — the standard Linux state path,
   falling back to `~/.local/share/<binary>/`.
4. On macOS: `~/Library/Application Support/<binary>/`.
5. On Windows: `%LOCALAPPDATA%/<binary>/`.

**Dev source trees are separate.** `extensions.dev_dirs` (a list of
absolute or repo-relative paths) is scanned but never written to.
The default for `rubix-agent` is `["rubix/extensions"]` — exactly what
ships with the repo today, only now marked dev so uninstall can refuse
to touch it.

## Changes

### 1. New crate: `starter-paths`

Owns OS-aware data-root resolution and the per-subdir API.

```rust
// crates/starter-paths/src/lib.rs (sketch)
pub struct Paths {
    root: PathBuf,
}

impl Paths {
    /// Resolve from env + XDG conventions. `app` is the binary
    /// name used as the leaf segment (e.g. `"rubix"`).
    pub fn resolve(app: &str, override_root: Option<PathBuf>)
        -> Result<Self, PathsError>;

    pub fn root(&self) -> &Path;
    pub fn config_dir(&self) -> PathBuf      { self.root.join("config") }
    pub fn extensions_dir(&self) -> PathBuf  { self.root.join("extensions") }
    pub fn logs_dir(&self) -> PathBuf        { self.root.join("logs") }
    /// Caller-named subdir under the root. For consumer-specific
    /// state — e.g. `paths.subdir("rubix/warehouse")`.
    pub fn subdir(&self, name: &str) -> PathBuf;

    /// Create the root + standard subdirs if missing. Idempotent.
    pub fn ensure(&self) -> Result<(), PathsError>;
}
```

No domain types. No HTTP. No DB. Same posture as `starter-config`.
Consumers depend on `Paths` and ask for what they need.

Where existing crates currently resolve their own paths — see
[`starter-secrets-file/src/store.rs:65`](../../../../crates/starter-secrets-file/src/store.rs#L65)
— they migrate to take a `&Paths` (or the specific subdir `PathBuf`)
in their builder. Migration is incremental; legacy `data_dir(...)`
overrides keep working.

### 2. Extension records carry an `origin`

`starter-ext-host::ExtensionRecord` gains:

```rust
pub enum BundleOrigin {
    /// Loaded in-place from a dev source tree.
    /// Uninstall will NOT delete the bundle directory.
    Dev { source_dir: PathBuf },
    /// Unpacked from an uploaded tarball into the installs dir.
    /// Uninstall removes the bundle directory.
    Installed { installs_dir: PathBuf },
}

pub struct ExtensionRecord {
    // ... existing fields
    pub origin: BundleOrigin,
}
```

The loader knows which it is at scan time:

- `Loader::scan_dev(path)` → marks records `BundleOrigin::Dev`.
- `Loader::scan_installs(path)` → marks records `BundleOrigin::Installed`.

The install handler always writes into `Paths::extensions_dir()` and
the resulting record is `Installed`. The boot scanner walks both
`installs_dir` and every entry in `dev_dirs`, with `Installed` taking
precedence on id collision (and a warning logged so the dev knows
their working tree is shadowed).

### 3. Safe uninstall

`lifecycle.rs::uninstall` branches on `origin`:

```rust
match rec.origin {
    BundleOrigin::Installed { .. } => {
        // existing behavior: remove_dir_all + run cleanup providers
    }
    BundleOrigin::Dev { source_dir } => {
        // 1. Set EnablementState::Disabled.
        // 2. Run cleanup providers (warehouse, skills, UI cache,
        //    enablement-row delete) — same as installed.
        // 3. SKIP remove_dir_all. The source tree is the user's.
        // 4. Return 200 with a manifest noting the bundle dir was
        //    preserved at `<source_dir>`.
    }
}
```

The dry-run preview (`GET /extensions/<id>/cleanup`) already exposes a
manifest of what will be removed. Add a `bundle: { path, will_delete }`
entry so the dialog can render *"Source files at `<path>` will be
preserved"* for dev mounts.

### 4. Rubix config — explicit two-path model

```rust
// rubix/crates/rubix-agent/src/boot/config.rs
pub struct ExtensionsConfig {
    pub enabled: bool,

    /// Dev source trees, scanned read-only. Never written to,
    /// never deleted from. Default: `["rubix/extensions"]`.
    pub dev_dirs: Vec<PathBuf>,

    /// Installed (uploaded) bundles live here. Writable. Resolved
    /// from `Paths::extensions_dir()` unless overridden.
    pub installs_dir: Option<PathBuf>,

    pub autostart_enabled_records: bool,
}
```

The single `dir` field is gone. A short migration note in CHANGELOG:
`extensions.dir` is interpreted as `extensions.dev_dirs = [<value>]`
for one release with a deprecation warning, then removed.

### 5. Frontend — surface the distinction

`UninstallDialog` already fetches the cleanup manifest. Two small
additions:

- A "Source location" line showing the bundle path.
- If dev-mounted: a badge "Dev bundle — source files are safe" and a
  confirm button reading **"Purge data & disable"** rather than
  **"Uninstall & purge"**.
- If installed: existing copy ("Uninstall & purge").

The kebab-menu **Admin** entry remains identical; only the dialog copy
diverges.

## Migration plan

1. Land `starter-paths` (no consumers yet). Tests cover XDG + Windows
   + macOS resolution and `subdir` collision/sanitization.
2. Add `BundleOrigin` to `starter-ext-host::ExtensionRecord` defaulting
   to `Installed { installs_dir: extensions_dir.clone() }` — existing
   behavior preserved.
3. Add `Loader::scan_dev` / `Loader::scan_installs` variants. The
   single-path `scan` keeps working, marked `#[deprecated]`.
4. Add the `origin` branch in `lifecycle.rs::uninstall`. Now safe
   even though nothing emits `Dev` records yet.
5. Switch `rubix-agent` boot to scan `dev_dirs` and `installs_dir`
   separately. The default `dev_dirs = ["rubix/extensions"]` instantly
   protects the in-repo source tree.
6. Wire `starter-paths` through rubix-agent and migrate existing
   consumers (`starter-secrets-file`, the enablement store, the UI
   cache) one at a time.
7. Update `UninstallDialog` copy.

Each step ships independently. Step 5 is the one that fixes the
reported bug; the earlier steps unblock it without changing behavior.

## Non-goals

- **No symlink farms.** We could install bundles outside the data
  root and symlink them in. Rejected: it complicates rm-rf safety
  reasoning and adds nothing dev-mode doesn't already.
- **No new config format.** Layered TOML via `starter-config` stays
  as-is; this proposal only adds fields.
- **No remote bundle URLs.** Install is still local upload + unpack.
  A future scope can add registry fetch.

## Open questions

1. Should `Paths::extensions_dir()` be called `installs_dir` to make
   the dev/installed split visible at the API surface? Probably yes —
   easier to grep for, harder to confuse.
2. Where does `$XDG_CONFIG_HOME` fit? Today `starter-config` reads
   from there for the layered loader. Suggest: config files stay in
   `$XDG_CONFIG_HOME/<app>/`; writable state lives in
   `Paths::root()` (= `$XDG_DATA_HOME/<app>/`). Don't conflate them.
3. Is a per-extension data dir worth adding now
   (`Paths::extensions_dir().join(id).join("data/")`)? Today the
   warehouse + skill stores carry their own per-extension state and
   the cleanup providers know how to drop it; a filesystem dir would
   duplicate that. Leave it out until a real consumer needs it.
