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

import { emitI18nTelemetry } from "./telemetry.js";

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
    const out = intl.formatMessage(
      { id: id as string },
      values as Parameters<typeof intl.formatMessage>[1],
    ) as unknown as string;
    // react-intl returns the id verbatim when neither the active
    // catalog nor the `en` default catalog has the key — that's our
    // "missing translation" signal. Fire `i18n.message_missing` once
    // per call so production dashboards can count gaps; in dev the
    // sink's default `console.warn` path is what surfaces it to the
    // author. `extensionId: null` — platform callers, not extension
    // callers (the SDK wires its own emit with the extension id).
    if (out === id) {
      emitI18nTelemetry({
        kind: "i18n.message_missing",
        severity: "warn",
        key: id as string,
        language: intl.locale,
        extensionId: null,
      });
    }
    return out;
  }) as TranslateFn;
}
