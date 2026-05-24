// User-facing locale preference store factory.
//
// Returns a zustand store hook whose state holds the active locale and a
// setter. Persists to `localStorage` under a per-app key so multiple apps
// in the same browser origin don't collide.
//
// Usage:
//
//     import { createLocaleStore } from "@nube/starter-ui-core/i18n";
//
//     export const useAppLocale = createLocaleStore({
//       persistKey: "myapp:locale",
//       locales: ["en", "es", "fr"] as const,
//       defaultLocale: "en",
//     });
//
//     // in a component
//     const locale = useAppLocale((s) => s.locale);
//     const setLocale = useAppLocale((s) => s.setLocale);
//
// Why a factory and not a singleton? Two reasons:
//   1. App-scoped `localStorage` keys prevent cross-app bleed.
//   2. The `locales` tuple's literal types flow through to the hook's
//      `locale` value type — consumers get an exhaustive union without
//      casting.

import { create, type StoreApi, type UseBoundStore } from "zustand";
import { persist, createJSONStorage } from "zustand/middleware";

export interface LocaleStoreState<L extends string> {
  locale: L;
  setLocale: (l: L) => void;
}

export interface CreateLocaleStoreOptions<L extends string> {
  /** localStorage key. Prefix with the app name to avoid collisions. */
  persistKey: string;
  /** Supported locales, in priority order. The first is used if no valid
   *  persisted locale is found and no `defaultLocale` is provided. */
  locales: readonly L[];
  /** Defaults to `locales[0]`. */
  defaultLocale?: L;
}

export function createLocaleStore<L extends string>(
  options: CreateLocaleStoreOptions<L>,
): UseBoundStore<StoreApi<LocaleStoreState<L>>> {
  const { persistKey, locales, defaultLocale } = options;
  const fallback = defaultLocale ?? locales[0];
  if (fallback === undefined) {
    throw new Error("createLocaleStore: `locales` must contain at least one entry");
  }

  return create<LocaleStoreState<L>>()(
    persist(
      (set) => ({
        locale: fallback,
        setLocale: (locale) => set({ locale }),
      }),
      {
        name: persistKey,
        storage: createJSONStorage(() => localStorage),
        // Guard against an unknown locale being read from storage (e.g.
        // after dropping a language). Falls back instead of crashing.
        merge: (persisted, current) => {
          const p = persisted as Partial<LocaleStoreState<L>> | undefined;
          if (p?.locale && (locales as readonly string[]).includes(p.locale)) {
            return { ...current, locale: p.locale };
          }
          return current;
        },
      },
    ),
  );
}
