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
export {
  BlockShell,
  DEFAULT_BLOCK_SHELL_MESSAGES,
  mergeBlockShellMessages,
  type BlockShellProps,
  type BlockShellMessages,
} from "./block-shell.js";
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

// Prefs + i18n hook surface (Stage 3, examples/notes/user-pref.md).
// These three hooks are the one stable read API an extension author
// uses to consume the host's resolved preferences + IntlShape via
// the singleton handshake.

export {
  HostBindingsProvider,
  useHostBindings,
  type HostBindings,
  type HostBindingsProviderProps,
} from "./host-bindings.js";

export {
  SINGLETON_REACT,
  SINGLETON_REACT_DOM,
  SINGLETON_UI_CORE_I18N,
  SINGLETON_UI_CORE_PREFERENCES,
} from "./singleton-keys.js";

export { useHostPrefs } from "./use-host-prefs.js";
export {
  useHostTranslate,
  type MessageValues,
  type TranslateFn,
} from "./use-host-translate.js";
export {
  useHostFormatters,
  type HostFormatters,
} from "./use-host-formatters.js";

export type {
  ExtensionMessageKey,
  MessageKey,
  PlatformMessageKey,
} from "./message-keys.js";

// Catalog-merge surface (Stage 5, `examples/notes/user-pref.md`). The
// helpers live in `@nube/starter-ui-core/i18n` because the registry has
// to share module state with `<IntlProvider>`; the SDK does not pull
// `@nube/starter-ui-core` into its dep graph (SCOPE: TS dep ban). The
// notes host's `extension-host.ts` imports them directly from ui-core.
//
// Documented here only — see
// `packages/starter-ui-core/src/i18n/extension-messages.ts`.

export type {
  CurrencyCode,
  DateFormat,
  HostIntlContextValue,
  HostIntlShape,
  HostPreferencesContextValue,
  NumberFormat,
  PreferencesPatch,
  Quantity,
  ResolvedPreferences,
  Theme,
  TimeFormat,
  Unit,
  UnitSystem,
  WeekStart,
} from "./prefs-types.js";
