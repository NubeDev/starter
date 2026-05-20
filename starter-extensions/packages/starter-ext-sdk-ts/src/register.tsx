// `registerExtensionContributions` — the single registration entry
// point an extension's `remoteEntry.init` calls.
//
// Wire shape (this is the contract `@nube/starter-ext-ui` consumes,
// see its `register-remote.ts`):
//
// ```ts
// // extension's remoteEntry.ts
// export default {
//   singletons: { react: { version: "18.3.1" }, "react-dom": { ... } },
//   async init(handle: ExtensionRemoteHandle) {
//     const Panel = (await import("./Panel.js")).default;
//     registerExtensionContributions(handle, {
//       components: { Panel },
//     });
//   },
// };
// ```
//
// The handle is passed in by the host's federation runtime after
// singleton-major negotiation succeeds. The extension never reaches
// into host internals directly; the SDK call delegates to the
// handle's `register` method, which the host owns.

import * as React from "react";

import { HostBindingsProvider } from "./host-bindings.js";

/**
 * Resolved singleton instances the host gives the extension. Keyed
 * by the package name the extension declared in its
 * `RemoteFactory.singletons`. Concretely each value is the
 * package's default module export (the host's `react`, `react-dom`,
 * `@tanstack/react-query`, `zustand` instances).
 *
 * `unknown` rather than typed entries because the SDK is loaded by
 * an extension that knows which singletons it asked for — typing the
 * resolved value to the requested set is the extension author's
 * job (a cast at the use site is fine; the host cannot prove the
 * shape).
 */
export type ResolvedSingletons = Readonly<Record<string, unknown>>;

/**
 * Contributions an extension publishes back to the host.
 *
 * The kernel keeps this minimal in v0.1: components keyed by the
 * `contributes.ui.exposes[*].name` declared in `block.yaml`. The
 * host matches names to slots via the manifest, never the extension.
 *
 * Adapter phases (REST/CLI/workers/gRPC) do not extend this struct
 * — those contributions live entirely on the Rust side. A UI
 * extension that *also* contributes tools declares the tools in
 * `block.yaml`; the UI side has no separate registration for them.
 */
export interface ExtensionContributions {
  /**
   * React components keyed by the `name` field of the matching
   * `contributes.ui.exposes` entry. The host looks each one up by
   * name when mounting an `<ExtensionSlot/>`.
   */
  components: Readonly<Record<string, React.ComponentType<unknown>>>;
}

/**
 * Opaque handle the host's federation runtime passes to the
 * remote's `init`. The extension reads `singletons` to bind its
 * shared deps (React etc.) and calls `register` exactly once with
 * its contributions.
 *
 * The handle is *not* re-usable across remotes. The host creates a
 * fresh one per `registerExtensionRemote` call.
 */
export interface ExtensionRemoteHandle {
  /** The remote's extension id (reverse-DNS, matches `block.yaml`). */
  readonly id: string;
  /** Singletons the host negotiated for this remote. */
  readonly singletons: ResolvedSingletons;
  /** Called by `registerExtensionContributions`. Implementation lives in the host. */
  register(contributions: ExtensionContributions): void;
}

/**
 * Publish the extension's contributions to the host.
 *
 * Implementation is a one-liner — the function exists so extension
 * authors have a single named call site, and so the SDK has a place
 * to add validation (component-name lints, dev-mode warnings) later
 * without a breaking API change.
 *
 * Calling twice from the same `init` is allowed but discouraged: the
 * second call replaces the first. The host's `registerExtensionRemote`
 * does not re-invoke `init`, so duplicate calls only happen if the
 * extension does something unusual; we keep the behaviour predictable
 * rather than refusing it.
 */
export function registerExtensionContributions(
  handle: ExtensionRemoteHandle,
  contributions: ExtensionContributions,
): void {
  // Wrap every contributed component in a `<HostBindingsProvider>`
  // seeded from the handle (extension id + resolved singletons). The
  // wrapping happens once at registration time, not per render, so
  // it does not add a useState/useEffect cost; the wrapper is a
  // closure over a stable `bindings` object. Hooks
  // (`useHostPrefs`/`useHostTranslate`/`useHostFormatters`) read
  // through this provider — extension authors get the host's prefs
  // and IntlShape without seeing the plumbing.
  const bindings = { extensionId: handle.id, singletons: handle.singletons };
  const wrapped: Record<string, React.ComponentType<unknown>> = {};
  for (const [name, Component] of Object.entries(contributions.components)) {
    wrapped[name] = wrapWithBindings(name, Component, bindings);
  }
  handle.register({ components: wrapped });
}

function wrapWithBindings(
  displayName: string,
  Component: React.ComponentType<unknown>,
  bindings: { extensionId: string; singletons: ResolvedSingletons },
): React.ComponentType<unknown> {
  const Wrapped = (props: unknown): React.ReactElement => (
    <HostBindingsProvider bindings={bindings}>
      <Component {...(props as Record<string, unknown>)} />
    </HostBindingsProvider>
  );
  Wrapped.displayName = `HostBindings(${bindings.extensionId}:${displayName})`;
  return Wrapped;
}
