// The `MessageKey` extension-author-facing type.
//
// The union is `PlatformMessageKey | ExtensionMessageKey | (string &
// {})`:
//
// - `PlatformMessageKey` — generated from
//   `crates/starter-i18n/catalogs/starter/en.json` by
//   `scripts/gen-message-keys.mjs`. Platform-supplied keys get
//   autocomplete; a typo against one of them surfaces as a type
//   error rather than a silent runtime "translation missing".
// - `ExtensionMessageKey` — empty in this package; extension authors
//   augment it with their own catalog keys via TypeScript module
//   augmentation (see the example at the bottom of this file).
//   The augmented keys flow through the same checking path.
// - `(string & {})` — escape hatch so an extension that has not yet
//   wired the augmentation still compiles. Reads as "any string" to
//   the structural matcher but `IntelliSense` still suggests the
//   typed members first.
//
// The intent — narrowest correct type the extension author can opt
// into — matches `@nube/starter-ui-core/i18n`'s `AppMessageKeys`
// pattern. Both surfaces accept the same input; the SDK's hook is
// the one extension authors will use.

import type { PlatformMessageKey } from "./message-keys.generated.js";

export type { PlatformMessageKey };

/**
 * Empty by default; an extension augments it to declare its own
 * catalog keys.
 *
 * ```ts
 * declare module "@nube/starter-ext-sdk-ts" {
 *   interface ExtensionMessageKey {
 *     "com.nube.hello.greeting": never;
 *     "com.nube.hello.unread": never;
 *   }
 * }
 * ```
 *
 * `never` is used as the value type because we only care about the
 * keys — the values come from the runtime catalog file, not from
 * TypeScript.
 */
export interface ExtensionMessageKey {}

/** Union of platform keys + extension-declared keys. The trailing
 * `string & {}` keeps untyped call sites compiling while still
 * making typed members rank first in autocomplete. */
export type MessageKey =
  | PlatformMessageKey
  | keyof ExtensionMessageKey
  | (string & Record<never, never>);
