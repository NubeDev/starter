// `useHostTranslate()` — typed translator bound to the host's
// `IntlShape` (singleton `@nube/starter-ui-core/i18n`).
//
// The returned function:
//
// - Accepts a `MessageKey` — the union of platform catalog keys
//   (generated from `crates/starter-i18n/catalogs/starter/en.json`
//   by `scripts/gen-message-keys.mjs`) plus any extension-declared
//   keys (TypeScript module augmentation; see `./message-keys.ts`).
//   A typo against a platform key is a TS error, not a silent
//   "translation missing".
// - Auto-prefixes keys without a dot with the calling extension's
//   id (D-NP.3) — so `t("greeting")` from `com.nube.hello` resolves
//   to `com.nube.hello.greeting`. Fully-qualified keys pass through
//   verbatim.
// - Routes through `intl.formatMessage` on the host's IntlShape, so
//   the extension shares one catalog + one language with the host
//   chrome. Even if the extension bundles its own `react-intl`, the
//   shape is the host's — methods + AST cache come from the host.

import * as React from "react";

import { useHostBindings } from "./host-bindings.js";
import { SINGLETON_UI_CORE_I18N } from "./singleton-keys.js";
import type { MessageKey } from "./message-keys.js";
import type { HostIntlContextValue, HostIntlShape } from "./prefs-types.js";

/** Variable bag for ICU MessageFormat placeholders. */
export type MessageValues = Record<
  string,
  string | number | boolean | Date | null | undefined
>;

export interface TranslateFn {
  (id: MessageKey): string;
  (id: MessageKey, values: MessageValues): string;
}

/**
 * Return a translator bound to the host's IntlShape + the calling
 * extension's id. Throws on the same wiring-bug paths as
 * `useHostPrefs` so the surface is uniformly loud.
 */
export function useHostTranslate(): TranslateFn {
  const { singletons, extensionId } = useHostBindings();
  const IntlCtx = singletons[SINGLETON_UI_CORE_I18N] as
    | React.Context<HostIntlContextValue | undefined>
    | undefined;
  if (!IntlCtx) {
    throw new Error(
      "useHostTranslate(): host did not provide the " +
        "@nube/starter-ui-core/i18n singleton. Declare it in your " +
        "remoteEntry factory's `singletons` block.",
    );
  }
  const value = React.useContext(IntlCtx);
  if (!value) {
    throw new Error(
      "useHostTranslate(): host has not mounted <IntlProvider>. " +
        "The notes host's app.tsx must wrap extensions in IntlProvider.",
    );
  }
  const intl: HostIntlShape = value.intl;

  return React.useMemo<TranslateFn>(() => {
    const translate = (id: MessageKey, values?: MessageValues): string => {
      // Auto-prefix bare keys with the extension id (D-NP.3). A
      // fully-qualified key carries at least one dot; bare keys are
      // single-segment identifiers like `greeting`.
      const fullId = typeof id === "string" && id.includes(".") ? id : `${extensionId}.${id}`;
      return intl.formatMessage({ id: fullId }, values);
    };
    return translate as TranslateFn;
  }, [intl, extensionId]);
}
