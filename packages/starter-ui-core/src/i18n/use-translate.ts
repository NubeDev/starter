// `useTranslate()` — typed wrapper around react-intl's
// `useIntl().formatMessage`. Returns a function with a `MessageKey`
// overload so callers get autocomplete on the dotted key shape and
// can plug in their own registry of keys via TS module augmentation.
//
// Fallback chain:
// 1. The catalog at `prefs.language` (loaded by `<IntlProvider>`).
// 2. The `en` catalog (react-intl's `defaultLocale`).
// 3. The literal `MessageKey` string — same as react-intl's default
//    behaviour when no translation is found.

import { useIntl } from "react-intl";

/** The reverse-DNS-style key shape. Apps can declare their own keys
 * by augmenting this module — e.g.
 *
 * ```ts
 * declare module "@nube/starter-ui-core" {
 *   interface AppMessageKeys {
 *     "myapp.dashboard.title": never;
 *   }
 * }
 * ```
 *
 * The augmented keys are then accepted by `useTranslate()` without
 * a cast. */
export interface AppMessageKeys {}

export type MessageKey = keyof AppMessageKeys | (string & {});

/** Variable bag for ICU MessageFormat placeholders. */
export type MessageValues = Record<string, string | number | boolean | Date | null | undefined>;

export interface TranslateFn {
  (id: MessageKey): string;
  (id: MessageKey, values: MessageValues): string;
}

/** Returns a translate function bound to the active catalog. The
 * function is stable for the lifetime of the surrounding
 * `<IntlProvider>` instance (react-intl re-creates `intl` on locale
 * change, which is why we re-key the provider). */
export function useTranslate(): TranslateFn {
  const intl = useIntl();
  return ((id: MessageKey, values?: MessageValues) => {
    // react-intl handles the en-fallback via `defaultLocale`; when
    // the key is absent from both, it returns the id verbatim. We
    // explicitly cast `values` because react-intl's signature is
    // wider than we expose.
    // react-intl's `formatMessage` overload set wants a narrower
    // value type than our `MessageValues`; cast at the boundary so
    // callers don't need to import react-intl's types.
    return intl.formatMessage(
      { id: id as string },
      values as Parameters<typeof intl.formatMessage>[1],
    ) as unknown as string;
  }) as TranslateFn;
}
