// @nube/starter-ext-ui
//
// Host-side Module Federation runtime. Forked from
// `rubix-workspace/extension-ui-sdk`'s `./mf` entry, with
// rubix-specific concepts (graph nodes, kind ids, slot paths tied to
// the graph store) stripped — they belong in rubix-agent, not in a
// generic starter (SCOPE.md §"UI package source").
//
// What this package owns:
//
// - Singleton negotiation. The host registers React, react-dom,
//   `@tanstack/react-query`, and `zustand` as shared singletons.
//   Every extension declares the same packages with a version; the
//   host enforces a matching-majors check at load time. Mismatch is
//   a hard refusal (SCOPE.md §"Decisions made" / singleton-mismatch).
// - `registerExtensionRemote(id, factory)` — called once per
//   enabled extension at host bootstrap. Looks up the manifest's
//   `contributes.ui.exposes`, runs `factory.init(handle)`, and
//   records the resulting components keyed by their name.
// - `<ExtensionSlot id="sidebar"/>` — at render time, looks up every
//   exposure whose `slot` matches `id`, wraps each in a
//   `SlotContextProvider`, and mounts them in source order.
// - `useExtensionHost()` — hook to read the host's view of installed
//   extensions and their lifecycle state. Reads via the injected
//   `StarterClient` (SCOPE R11 — never raw fetch).
//
// SCOPE.md says `starter-ext-ui` is separate from `starter-ui-kit`:
// a consumer that renders shadcn primitives without extensions does
// not pay for the federation runtime. This package therefore has no
// design-system deps; it only provides plumbing.

export {
  ExtensionHostManager,
  type ExtensionHostManagerOptions,
  type ExtensionHostTelemetryEvent,
  type ExtensionHostTelemetrySink,
  type ExtensionRemoteFactory,
  type ManifestUi,
  type ManifestUiExpose,
  type RegisteredRemote,
  type SingletonProvision,
  type SlotResolution,
} from "./host-manager.js";

export {
  ExtensionHostProvider,
  type ExtensionHostProviderProps,
} from "./host-provider.js";

export { useExtensionHostManager } from "./host-context.js";

export {
  ExtensionSlot,
  type ExtensionSlotProps,
} from "./extension-slot.js";

export {
  useExtensionHost,
  type ExtensionHostView,
  type ExtensionHostExtensionView,
} from "./use-extension-host.js";

export {
  bootstrapExtensions,
  type BootstrapOptions,
  type BootstrapResult,
  type BootstrapExtensionSummary,
  type BootstrapExtensionDetail,
} from "./bootstrap.js";

export {
  matchingMajor,
  parseMajor,
  parseMinor,
  SINGLETON_REACT,
  SINGLETON_REACT_DOM,
  SINGLETON_UI_CORE_I18N,
  SINGLETON_UI_CORE_PREFERENCES,
  SingletonMismatchError,
  type SingletonMinorDrift,
  type SingletonMismatchReason,
} from "./singletons.js";

export type {
  ExtensionRemoteHandle,
  ExtensionContributions,
  ResolvedSingletons,
  HostThemeMode,
  HostThemeTokens,
  HostTheme,
} from "@nube/starter-ext-sdk-ts";
