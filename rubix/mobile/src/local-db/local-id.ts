// local-id.ts — generate a local row ID without needing a Web Crypto
// polyfill.
//
// We previously used `ulid()` for SQLite primary keys, but the `ulid`
// package needs `globalThis.crypto.getRandomValues`, and the obvious
// polyfill (`react-native-get-random-values`) requires a native module
// that Expo Go does not bundle. Trying to import it on Expo Go cascades
// into "Invalid hook call / useLinkPreviewContext / null RCTView" after
// the root layout module fails to evaluate cleanly.
//
// These IDs never leave the device — they live in SQLite, identify
// connections/dashboards locally, and are never compared cryptographically.
// So a 48-bit timestamp + a 12-character base36 random suffix from
// Math.random is more than enough for our scale (a few hundred rows per
// device, no adversary to collide against).

const ALPHABET = '0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ';

/**
 * Returns a 21-character lexicographically-sortable local ID:
 *  - 9 chars of base36-encoded timestamp (ms since epoch — sorts by
 *    creation order until year 5188)
 *  - 12 chars of base36 random suffix (~62 bits of entropy from two
 *    Math.random calls — collision-safe for on-device row counts)
 *
 * The string is uppercase base36 to mimic the look of ULIDs in logs so
 * older log lines remain visually consistent.
 */
export function localId(): string {
  const ts = Date.now().toString(36).toUpperCase().padStart(9, '0');
  return ts + randomBase36(12);
}

function randomBase36(length: number): string {
  let out = '';
  for (let i = 0; i < length; i++) {
    out += ALPHABET[Math.floor(Math.random() * ALPHABET.length)];
  }
  return out;
}
