// @nube/starter-ext-sdk-ts
//
// Forked from `rubix-workspace/extension-ui-sdk` main entry. The
// rubix-specific graph hooks (`useNode`, `useSlot`, `useKinds`) were
// stripped — they live in rubix-agent's domain and have no analogue
// in a generic starter (SCOPE.md §"UI package source").
//
// This is what a UI extension author imports. The four exports are
// the v0.1 surface:
//
// - `useHostClient` — typed wrapper over `@nube/starter-client-ts`.
//   UI extensions never issue raw `fetch` calls (SCOPE.md R11);
//   every call to the host goes through this hook so auth, tracing,
//   and retry are uniform across the kit.
// - `BlockShell` — the standard panel wrapper. Wires the slot
//   context provider, an error boundary, and a loading-state
//   skeleton around an extension's panel root.
// - `useSlotContext` — read the slot id, host theme, and feature
//   flags relevant to the place the extension is mounted in.
// - `registerExtensionContributions` — single registration entry
//   point called from the extension's `remoteEntry` `init`. Returns
//   the contributions to the host via the `ExtensionRemoteHandle`
//   so the host's runtime never has to scan the extension's exports.

export {
  ExtensionHostClientProvider,
  useHostClient,
  type ExtensionHostClient,
  type ExtensionHostClientProviderProps,
} from "./host-client.js";
export { BlockShell, type BlockShellProps } from "./block-shell.js";
export {
  SlotContextProvider,
  useSlotContext,
  type SlotContext,
  type SlotContextProviderProps,
  type HostThemeMode,
  type HostThemeTokens,
} from "./slot-context.js";
export {
  useHostTheme,
  type HostTheme,
} from "./use-host-theme.js";
export {
  registerExtensionContributions,
  type ExtensionContributions,
  type ExtensionRemoteHandle,
  type ResolvedSingletons,
} from "./register.js";
