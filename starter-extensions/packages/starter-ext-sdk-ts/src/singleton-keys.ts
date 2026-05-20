// Singleton id constants — kept here (in the SDK) so extension
// authors and tests do not have to depend on `@nube/starter-ext-ui`
// just to reference a string. Mirrors the constants in
// `@nube/starter-ext-ui/src/singletons.ts`. Drift between the two
// would break the handshake silently, so the host-side test in
// `starter-ext-ui/host-manager.test.ts` imports from this file too;
// any rename surfaces as a compile error.

export const SINGLETON_REACT = "react" as const;
export const SINGLETON_REACT_DOM = "react-dom" as const;
export const SINGLETON_UI_CORE_PREFERENCES = "@nube/starter-ui-core/preferences" as const;
export const SINGLETON_UI_CORE_I18N = "@nube/starter-ui-core/i18n" as const;
