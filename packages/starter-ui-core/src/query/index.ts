// Query-key namespacing. Every react-query key owned by a starter
// crate or by `@nube/starter-ui-core` must be built via
// `starterQueryKey(...)` so consumer app code never collides with
// starter-owned cache entries.
//
// SCOPE 117–134: starter owns the `['starter', ...]` prefix; consumer
// keys must start with anything else.

export const STARTER_QUERY_PREFIX = "starter" as const;

/** Build a react-query key under the `['starter', ...]` namespace.
 *
 * @example
 *   useQuery({ queryKey: starterQueryKey('auth', 'me'), queryFn: ... })
 *   // → ['starter', 'auth', 'me']
 */
export function starterQueryKey(...parts: ReadonlyArray<string | number>): readonly unknown[] {
  return [STARTER_QUERY_PREFIX, ...parts] as const;
}

/** Type guard: is this query key starter-owned? Useful for cache
 * invalidation that should leave consumer keys alone. */
export function isStarterQueryKey(key: readonly unknown[]): boolean {
  return key[0] === STARTER_QUERY_PREFIX;
}
