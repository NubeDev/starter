// token/expo-secure-store.ts — production `SecureTokenStore` impl.
//
// Keyed by `rubix.token.<connectionId>` per LOCAL-DB.md §Secret handling
// — the platform Keychain (iOS) / Keystore (Android) row. SQLite holds
// the connection list; this holds the bearer secret. Same trust boundary,
// different store.

import * as SecureStore from 'expo-secure-store';

import type { SecureTokenStore } from './contract';

const KEY_PREFIX = 'rubix.token.';

function keyFor(connectionId: string): string {
  return `${KEY_PREFIX}${connectionId}`;
}

export const expoSecureTokenStore: SecureTokenStore = {
  async get(connectionId) {
    return SecureStore.getItemAsync(keyFor(connectionId));
  },
  async put(connectionId, token) {
    await SecureStore.setItemAsync(keyFor(connectionId), token);
  },
  async clear(connectionId) {
    await SecureStore.deleteItemAsync(keyFor(connectionId));
  },
};
