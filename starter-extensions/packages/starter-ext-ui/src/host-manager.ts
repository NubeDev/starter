// `ExtensionHostManager` — the host's runtime state for federation.
//
// Responsibilities:
//
// - Hold the one set of shared singletons the host provides (React,
//   react-dom, the query lib, the store). Every remote binds to
//   *these* references; the runtime enforces matching majors before
//   handing them over.
// - Hold per-extension contributions (components keyed by name) the
//   remote's `init` registers.
// - Hold the manifest snapshot the host fetched from
//   `GET /extensions/<id>` (or from a static seed in tests) so
//   `<ExtensionSlot/>` can look up `contributes.ui.exposes` by slot
//   id without re-fetching.
// - Expose `registerExtensionRemote(id, factory)` — the one entry
//   point the host shell calls per enabled extension.
//
// The manager is intentionally framework-light. It holds plain JS
// state and emits change notifications via subscribe; React layers
// (`ExtensionHostProvider`, `<ExtensionSlot/>`) read it through hooks.

import type {
  ExtensionContributions,
  ExtensionRemoteHandle,
  ResolvedSingletons,
} from "@nube/starter-ext-sdk-ts";
import type { StarterClient } from "@nube/starter-client-ts";

import {
  matchingMajor,
  parseMajor,
  parseMinor,
  SingletonMismatchError,
  type SingletonMinorDrift,
  type SingletonMismatchReason,
} from "./singletons.js";

/**
 * Telemetry events emitted by `ExtensionHostManager` when it
 * negotiates singletons. Names match
 * `examples/notes/user-pref.md` § Telemetry. Production deployments
 * key dashboards off these strings — they are part of the public
 * contract.
 */
export type ExtensionHostTelemetryEvent =
  | {
      kind: "extension.singleton_mismatch";
      severity: "error";
      extensionId: string;
      reasons: ReadonlyArray<SingletonMismatchReason>;
    }
  | {
      kind: "extension.singleton_minor_drift";
      severity: "warn";
      extensionId: string;
      drifts: ReadonlyArray<SingletonMinorDrift>;
    };

/** Sink for telemetry events. Implementations should be cheap and
 * non-throwing — the host swallows any exception so a misbehaving
 * sink can't take down the registration path. */
export type ExtensionHostTelemetrySink = (event: ExtensionHostTelemetryEvent) => void;

/**
 * The host-side declaration of one shared singleton. Bundled as a
 * `{ version, instance }` pair so the matching-majors check and the
 * actual binding both work off the same source.
 */
export interface SingletonProvision {
  version: string;
  instance: unknown;
}

/**
 * What an extension's `remoteEntry` default-exports. The host loads
 * the entry (via dynamic import, `<script>` tag, or — in tests — a
 * direct object) and reads these fields.
 */
export interface ExtensionRemoteFactory {
  /**
   * Singletons this remote consumes. Keyed by package name. The
   * version is what the extension was *built against* — the host
   * compares majors to its own provided versions.
   */
  singletons: Readonly<Record<string, { version: string }>>;
  /**
   * Called by the host once singleton negotiation succeeds. The
   * handle exposes the resolved singletons (the host's own
   * instances) and a `register` callback for the remote to publish
   * its contributions through.
   */
  init(handle: ExtensionRemoteHandle): Promise<void> | void;
}

/**
 * Snapshot of a registered remote. Held by the host manager and
 * surfaced to `<ExtensionSlot/>` / `useExtensionHost()`.
 */
export interface RegisteredRemote {
  id: string;
  /**
   * The manifest's `contributes.ui` block. The slot resolver maps
   * `exposes[i].name` to the component the remote registered.
   */
  ui: ManifestUi;
  /**
   * What the remote's `init` registered. Absent until `init`
   * resolves; absent forever for remotes that don't expose any
   * components.
   */
  contributions: ExtensionContributions | null;
}

/**
 * Minimal manifest UI shape, in TS. Kept structurally compatible
 * with `starter_ext_spi::manifest::ContributeUi` so the host can
 * pass `GET /extensions/<id>`'s JSON straight in.
 */
export interface ManifestUi {
  entry: string;
  exposes: ReadonlyArray<ManifestUiExpose>;
}

export interface ManifestUiExpose {
  name: string;
  module: string;
  slot: string;
}

export interface ExtensionHostManagerOptions {
  /**
   * The host's `StarterClient` instance. Available to extensions
   * through `useHostClient()` (`@nube/starter-ext-sdk-ts`); also
   * used by `useExtensionHost()` to read `/extensions`.
   */
  client: StarterClient;
  /**
   * Shared singletons the host provides. Every key listed here is a
   * package the host has *one* live instance of; extensions that
   * declare any of these keys receive `instance` and must match the
   * host's major.
   *
   * The four well-known keys (`react`, `react-dom`,
   * `@tanstack/react-query`, `zustand`) per SCOPE R11 are the
   * baseline; consumers may add more (e.g. a design-system package).
   */
  singletons: Readonly<Record<string, SingletonProvision>>;
  /**
   * Optional telemetry sink. When provided, the manager emits one
   * `extension.singleton_mismatch` event per refused registration and
   * one `extension.singleton_minor_drift` event per registration with
   * a minor-only drift. Production hosts wire this through the
   * existing observability event bus; tests inject a recording sink.
   */
  telemetry?: ExtensionHostTelemetrySink;
}

/**
 * Listener fired whenever the registered-remote table changes —
 * either a new remote registered or a remote's `init` resolved its
 * contributions. React layers subscribe in `useSyncExternalStore`.
 */
type Listener = () => void;

export class ExtensionHostManager {
  readonly client: StarterClient;
  readonly singletons: Readonly<Record<string, SingletonProvision>>;
  private readonly telemetry: ExtensionHostTelemetrySink | undefined;

  private remotes = new Map<string, RegisteredRemote>();
  private listeners = new Set<Listener>();
  // `useSyncExternalStore` requires the getSnapshot result to be
  // referentially stable across calls when state has not changed.
  // We memoise the per-slot resolution and the remotes-list snapshot,
  // invalidating both inside `notify()`.
  private slotCache = new Map<string, ReadonlyArray<SlotResolution>>();
  private remotesSnapshot: ReadonlyArray<RegisteredRemote> | null = null;

  constructor(opts: ExtensionHostManagerOptions) {
    this.client = opts.client;
    this.singletons = opts.singletons;
    this.telemetry = opts.telemetry;
  }

  /** Emit a telemetry event, swallowing any error from the sink so a
   * misbehaving observer can't take down extension registration. */
  private emit(event: ExtensionHostTelemetryEvent): void {
    if (!this.telemetry) return;
    try {
      this.telemetry(event);
    } catch (err) {
      // eslint-disable-next-line no-console
      console.warn("[starter-ext-ui] telemetry sink threw:", err);
    }
  }

  // --- subscription -----------------------------------------------------

  subscribe(listener: Listener): () => void {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  }

  private notify(): void {
    this.slotCache.clear();
    this.remotesSnapshot = null;
    for (const l of this.listeners) l();
  }

  // --- read-side --------------------------------------------------------

  listRemotes(): ReadonlyArray<RegisteredRemote> {
    if (this.remotesSnapshot === null) {
      this.remotesSnapshot = Array.from(this.remotes.values());
    }
    return this.remotesSnapshot;
  }

  getRemote(id: string): RegisteredRemote | undefined {
    return this.remotes.get(id);
  }

  /**
   * Return every (extensionId, exposeMeta, component) triple whose
   * `slot` matches the requested slot id. `<ExtensionSlot/>` reads
   * this through `useSyncExternalStore`; the result is memoised
   * until the next `notify()` so the subscription's snapshot stays
   * referentially stable.
   *
   * Source order is stable: insertion order in `remotes` ×
   * declaration order in `exposes`.
   */
  resolveSlot(slotId: string): ReadonlyArray<SlotResolution> {
    const cached = this.slotCache.get(slotId);
    if (cached) return cached;
    const out: SlotResolution[] = [];
    for (const rec of this.remotes.values()) {
      for (const exp of rec.ui.exposes) {
        if (exp.slot !== slotId) continue;
        const component = rec.contributions?.components[exp.name];
        out.push({
          extensionId: rec.id,
          expose: exp,
          component: component ?? null,
        });
      }
    }
    const frozen: ReadonlyArray<SlotResolution> = Object.freeze(out);
    this.slotCache.set(slotId, frozen);
    return frozen;
  }

  // --- write-side -------------------------------------------------------

  /**
   * Register one extension's remote. Throws on singleton mismatch
   * (the caller — typically the host shell's bootstrap — marks the
   * extension `Failed` and continues with the rest).
   *
   * Idempotency: calling twice for the same `id` replaces the prior
   * registration. This matches SCOPE.md's enable/disable model where
   * disable→enable re-spawns the remote.
   */
  async registerExtensionRemote(
    id: string,
    ui: ManifestUi,
    factory: ExtensionRemoteFactory,
  ): Promise<RegisteredRemote> {
    const mismatches = this.checkSingletons(factory.singletons);
    if (mismatches.length > 0) {
      this.emit({
        kind: "extension.singleton_mismatch",
        severity: "error",
        extensionId: id,
        reasons: mismatches,
      });
      throw new SingletonMismatchError(id, mismatches);
    }

    const drifts = this.checkMinorDrift(factory.singletons);
    if (drifts.length > 0) {
      this.emit({
        kind: "extension.singleton_minor_drift",
        severity: "warn",
        extensionId: id,
        drifts,
      });
    }

    const resolved: Record<string, unknown> = {};
    for (const pkg of Object.keys(factory.singletons)) {
      // checkSingletons already verified the host provides every
      // declared pkg with a matching major.
      const provision = this.singletons[pkg];
      if (provision) resolved[pkg] = provision.instance;
    }
    const resolvedSingletons: ResolvedSingletons = Object.freeze(resolved);

    let contributions: ExtensionContributions | null = null;
    const handle: ExtensionRemoteHandle = {
      id,
      singletons: resolvedSingletons,
      register: (c) => {
        contributions = c;
      },
    };

    await factory.init(handle);

    const rec: RegisteredRemote = { id, ui, contributions };
    this.remotes.set(id, rec);
    this.notify();
    return rec;
  }

  /**
   * Unregister a remote. Called by the host shell when an extension
   * is disabled; the next `<ExtensionSlot/>` render no longer mounts
   * any of its components.
   */
  unregisterExtensionRemote(id: string): void {
    if (this.remotes.delete(id)) this.notify();
  }

  // --- internals --------------------------------------------------------

  /**
   * Return any singleton declarations whose major does not match the
   * host's provision. Empty means OK. The full diagnostic shape goes
   * into the thrown `SingletonMismatchError`.
   */
  private checkSingletons(
    declared: Readonly<Record<string, { version: string }>>,
  ): SingletonMismatchReason[] {
    const reasons: SingletonMismatchReason[] = [];
    for (const [pkg, decl] of Object.entries(declared)) {
      const provision = this.singletons[pkg];
      if (!provision) {
        reasons.push({
          pkg,
          hostVersion: "",
          extensionVersion: decl.version,
          reason: `${pkg}@${decl.version} requested but host provides no singleton for it`,
        });
        continue;
      }
      if (!matchingMajor(provision.version, decl.version)) {
        reasons.push({
          pkg,
          hostVersion: provision.version,
          extensionVersion: decl.version,
          reason: `${pkg}@${decl.version} vs host ${provision.version} — major mismatch`,
        });
      }
    }
    return reasons;
  }

  /**
   * Return any singletons whose declared minor is *strictly behind*
   * the host's minor (host: 1.3, extension: 1.1 → drift of 2). Used
   * to fire `extension.singleton_minor_drift` after a successful
   * major-match. Same major is a precondition: callers only invoke
   * this once `checkSingletons` has returned no mismatches, so we
   * silently skip any pkg whose major doesn't match (defensive — it
   * shouldn't happen). Extensions declared *ahead* of the host on
   * minor are not flagged; that's a host-needs-updating signal, not
   * an extension issue.
   */
  private checkMinorDrift(
    declared: Readonly<Record<string, { version: string }>>,
  ): SingletonMinorDrift[] {
    const out: SingletonMinorDrift[] = [];
    for (const [pkg, decl] of Object.entries(declared)) {
      const provision = this.singletons[pkg];
      if (!provision) continue;
      const hostMajor = parseMajor(provision.version);
      const extMajor = parseMajor(decl.version);
      if (hostMajor === null || extMajor === null || hostMajor !== extMajor) {
        continue;
      }
      const hostMinor = parseMinor(provision.version);
      const extMinor = parseMinor(decl.version);
      if (hostMinor === null || extMinor === null) continue;
      const drift = hostMinor - extMinor;
      if (drift > 0) {
        out.push({
          pkg,
          hostVersion: provision.version,
          extensionVersion: decl.version,
          driftMinors: drift,
        });
      }
    }
    return out;
  }
}

/** Item produced by `ExtensionHostManager.resolveSlot`. */
export interface SlotResolution {
  extensionId: string;
  expose: ManifestUiExpose;
  /**
   * `null` while the remote's `init` is still resolving, or when an
   * extension declared an `exposes[*].name` its `init` did not
   * register. The slot renders nothing for that entry.
   */
  component: import("react").ComponentType<unknown> | null;
}
