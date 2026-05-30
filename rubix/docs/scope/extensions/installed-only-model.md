# Installed-only extension model (single path for dev + prod)

> **Tier:** plan, not system-as-it-is. Lives in `docs/scope/` per
> [HOW-TO-CODE.md §0a](../../../HOW-TO-CODE.md). Source code must not
> reference this file — once landed, its design moves into
> `docs/design/extensions/` and code links there.
>
> **Supersedes:** [`data-root-and-safe-uninstall.md`](data-root-and-safe-uninstall.md).
> That doc proposed two paths (dev mounts + installed bundles) with a
> safety branch in `uninstall`. This doc deletes the dev-mount path
> entirely. Once landed, the predecessor moves to
> `docs/design/extensions/historical/` for context only — no live
> code paths remain.

**Status:** proposal
**Author:** rubix-agent
**Date:** 2026-05-30

## Problem

After landing the dev/installed split, three rows still show up in the
**Installed extensions** UI on every dev boot:

- `com.nubeio.rubixos`
- `com.rubix.example`
- `com.rubix.geo`

None of these were installed via `POST /extensions/install`. They are
**dev mounts** — bundles in the working tree at `rubix/extensions/`
that the agent scans because `dev_dirs = ["rubix/extensions"]` is set
in [`rubix/dev/agent.toml`](../../../dev/agent.toml). The default in
[`ExtensionsConfig`](../../../crates/rubix-agent/src/boot/config.rs)
is the same hardcoded path, so a production agent started from a
checkout *also* picks them up unless explicitly overridden.

That mismatch is the real problem. Dev and production take **different
code paths**:

| | Dev (today) | Production (today) |
|---|---|---|
| Where bundles live | `rubix/extensions/<id>/` (git working tree) | `$XDG_DATA_HOME/rubix/extensions/installed/<id>/` |
| How they're built | `make build install` writes binary next to `block.yaml` | `tar -czf` then `POST /extensions/install` unpacks |
| Loader call | `Loader::scan_dev` → `BundleOrigin::Dev` | `Loader::scan_installs` → `BundleOrigin::Installed` |
| Uninstall behaviour | Preserve source tree; purge data only | `remove_dir_all` the bundle dir + purge data |
| UI copy | "Dev bundle — source files are safe" | "Uninstall & purge" |

Every one of those differences is a place where dev passes and prod
fails (or vice versa). The safe-uninstall fix we just landed is one
such failure that we papered over; the next one is waiting.

## Principle

**One path for everything.** A bundle reaches the runtime *only* by
being unpacked into `Paths::installs_dir()`. There is no dev-mount
concept. Local development and production deployment use the same
code path, the same loader, the same uninstall semantics, the same
UI copy.

## Design

### Lifecycle (single path)

```
edit source → make build → make pack → POST /extensions/install
                                       → unpacked into installs_dir
                                       → agent restart picks it up
```

For every extension. For dev *and* prod. The agent never reads from
the git working tree.

### What gets deleted

Code:

| File | What goes |
|---|---|
| `starter-ext-host/src/record.rs` | `enum BundleOrigin` |
| `starter-ext-host/src/loader.rs` | `Loader::scan_dev`, dev-vs-installed precedence in `scan` |
| `rubix-agent/src/boot/config.rs` | `ExtensionsConfig::dev_dirs`, `::dir` (deprecated field), default value |
| `rubix-agent/src/boot/extensions.rs` | `effective_dev_dirs`, dev-tree scan loop, shadow-warn logging |
| `starter-ext-server/src/lifecycle.rs` | `BundlePlan::PreserveDev`, dev branch in `apply_bundle_removal`, dev path in `plan_bundle_action` |
| `starter-ext-server/src/lifecycle.rs` | `BundleOutcome::will_delete` collapses to always-true (or field removed) |
| Frontend `UninstallDialog` | dev-bundle badge + dual confirm-button copy |

Tests covering dev-mount preservation go away. Installed-bundle
coverage stays and gains the cases the dev branch used to cover.

Config:

`rubix/dev/agent.toml` loses its `[extensions]` block (or keeps only
`enabled = true`). The hardcoded `rubix/extensions` default disappears
with the field.

Doc/UI surface:

- `bundle.will_delete` in the cleanup-preview response — drop the
  field or keep it always `true` for one release as a compat hint.
- `UninstallDialog` shows a single copy variant: "Uninstall & purge".

### What stays

- `starter-paths` and `Paths::installs_dir()` — the whole reason this
  refactor is safe. One place that resolves where bundles live.
- `Loader::scan_installs` — the only scanner.
- Cleanup providers (warehouse, UI cache, enablement, skills) — the
  data-purge path is orthogonal to where the bundle came from.
- The pack + multipart-install flow already used by the `data-root`
  Makefile verification — it becomes the *only* install flow.

### Makefile contract (per extension)

Every extension Makefile (currently
[`com.nubeio.rubixos/Makefile`](../../../extensions/com.nubeio.rubixos/Makefile),
[`com.rubix.example/Makefile`](../../../extensions/com.rubix.example/Makefile),
[`com.rubix.geo/Makefile`](../../../extensions/com.rubix.geo/Makefile))
gains the same canonical targets:

```
make build              # cargo build --release; copy binary next to block.yaml
make ui-build           # vite build (if present)
make pack               # tar -czf /tmp/<id>.tar.gz <bundle>  (excluding target/, node_modules/)
make install            # POST /extensions/install with the tarball
make reload             # make -C <repo>/rubix restart  (so registry re-scans)
make uninstall          # DELETE /extensions/<id>?purge=true
make all                # build + ui-build + pack + install + reload
```

`make all` is the dev iteration. It is *identical* to what an operator
runs in production except that production fetches the tarball from a
registry rather than building it locally.

### Boot behaviour after the refactor

```
[extensions]
enabled = true
# installs_dir = "/var/lib/rubix/extensions/installed"   # production override
# (no dev_dirs field exists)
```

On boot the agent:

1. Resolves `installs_dir` from `Paths::installs_dir()` unless
   overridden in config.
2. `Loader::scan_installs(installs_dir).validate_all()`.
3. Spawns supervisors for every persisted-enabled record.

A fresh checkout starts with **zero extensions** until the developer
runs `make all` in at least one bundle. This is by design — it forces
the dev loop to match production from minute one.

### Migration

For each extension currently committed to `rubix/extensions/`:

1. Land the new Makefile targets (this is stage 2; reversible).
2. Verify `make all` works against a still-dev_dirs-enabled agent.
3. Then land the code deletions (stage 3).
4. After the deletion lands, document in the repo README that a fresh
   checkout requires `make -C rubix/extensions/<id> all` before any
   extension appears in the UI.

The example bundles (`com.rubix.example`, `com.rubix.geo`) stay
checked into the repo as **source**. They are not auto-loaded; the
developer installs the ones they want. Bonus: this fixes the
phantom-extension noise (currently three rows the user didn't ask for).

## Trade-offs (named explicitly)

**Slower dev iteration.** Today: edit → `make -C rubix restart`. Tomorrow:
edit → `make build pack install reload`. On a hot build that's still
fast (~5–10s) but it's no longer a single command. We can keep `make all`
as the single command so it still feels like one step.

**Fresh-checkout discoverability.** A new contributor cloning the repo
and running `make start` will see no extensions and may not know why.
Mitigate: the rubix root Makefile's `make start` calls
`make -C rubix/extensions/com.rubix.example all` (or all three) as a
post-bootstrap convenience for first-time setup. Easy to add and easy to
disable.

**`safe-uninstall` test becomes vacuous.** That's actually the win — the
attack surface that test was guarding no longer exists.

**Existing operators with `dev_dirs` set will break.** Boot log gets a
hard error "extensions.dev_dirs is no longer supported (see
docs/design/extensions/installed-only-model.md); install via
POST /extensions/install instead." One release of loud-fail beats
silent unexpected behaviour.

## Stage plan

1. **This doc lands** — no code changes yet. Review + sign-off here.
2. **Update three extension Makefiles** to the new contract. `make all`
   becomes build+pack+install+reload. Verify each works against the
   *current* agent (dev_dirs still active, just unused by the new
   flow). Reversible.
3. **Delete `dev_dirs` / `BundleOrigin` / `scan_dev` / dev-uninstall
   branch.** Workspace compiles. Agent boots clean against an empty
   `installs_dir` showing zero extensions. Run `make all` for each
   bundle, confirm they appear, confirm `make uninstall` removes them.
4. **Update `UninstallDialog`** to drop the dev-badge branch and the
   alternate button copy.
5. **Update tests** — remove dev-mount cases; assert the empty-boot +
   install-restart-uninstall path is rock-solid.
6. **Move predecessor doc** to `docs/design/extensions/historical/` and
   add a one-line redirect at the top of this doc once it itself moves
   to `docs/design/extensions/`.

Each stage is one commit. Stage 3 is the only one where boot behaviour
changes for an existing checkout — that's the release-note line.

## Non-goals

- **Registry fetch.** Install is still local multipart upload. A
  signed-registry-URL install path is a later scope.
- **Hot reload.** Still requires `make -C rubix restart`. The supervisor
  registry seals at boot; live re-scan is out of scope.
- **Per-extension data dirs on disk.** Cleanup providers continue to own
  per-extension state in PG/CH/UI-cache. No filesystem `data/` dir per
  extension yet.

## Open questions

1. Should the rubix root `make start` auto-install the example bundles
   on first run so a fresh checkout still demos something? Suggest:
   yes, behind a `RUBIX_DEV_AUTOINSTALL_EXAMPLES=1` env var that
   `rubix/dev/agent.toml`'s comments document. Off by default so
   production never auto-installs anything.
2. Do we keep `bundle.will_delete` in the cleanup-preview response
   (always `true`) or remove it? Suggest: remove. Frontend already
   needs an update for the dialog copy; one release of API breakage
   beats a vestigial field.
3. Should the `Paths::installs_dir()` default leaf change from
   `extensions/installed/` to just `extensions/`? Suggest: no — the
   `installed/` subdir leaves room for `extensions/staging/`,
   `extensions/quarantine/`, etc. without another migration.
