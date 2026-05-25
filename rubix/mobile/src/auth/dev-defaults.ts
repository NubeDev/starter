// auth/dev-defaults.ts — credential prefills for development builds only.
//
// These mirror the operator that `rubix-admin bootstrap-user` creates by
// default (see `rubix/Makefile` `bootstrap` target). They are convenience
// for adding a fresh local server during development, NOT a fallback —
// the prefill is gated by `__DEV__`, which Metro / Expo dev client set to
// `true` and EAS production builds set to `false`. The string literals
// never reach a shipped binary's default UI state.

export const DEV_LOGIN_DEFAULTS = {
  email: 'op@example.com',
  password: 'rubix-dev-passwd',
} as const;

/**
 * `true` in Metro / Expo dev client; `false` in EAS production builds.
 * `typeof` guard so the module is safe to import in non-RN contexts
 * (unit tests under vitest/node, where `__DEV__` is undefined).
 */
export const PREFILL_LOGIN_IN_DEV: boolean =
  typeof __DEV__ !== 'undefined' && __DEV__ === true;
