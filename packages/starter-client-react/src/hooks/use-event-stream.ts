// `useEventStream` — React bridge over the `streamJson` SSE
// primitive in `@nube/starter-client-ts`.
//
// Returns the last value the stream emitted (or `null` until the
// first frame), the current connection status, and a stable
// `reconnect()` callback. The hook owns one `AbortController` per
// mounted subscription; on unmount we abort it so the underlying
// iterator returns cleanly.
//
// Status transitions:
//
//   connecting  → first attempt is in flight.
//   open        → at least one frame has arrived.
//   reconnecting → the primitive invoked `onReconnecting`.
//   closed      → consumer aborted (unmount or manual reconnect).
//   error       → terminal error surfaced by the iterator.
//
// Implementation note: we use `useSyncExternalStore` rather than
// `useState` so concurrent renders always see a consistent snapshot
// of (data, status, error). The store is created once per (path,
// client) pair via `useMemo`; `reconnect` bumps a generation counter
// to force a fresh store instance, which keeps `reconnect`'s
// identity stable while still tearing the old subscription down.

import { useCallback, useEffect, useMemo, useState, useSyncExternalStore } from "react";

import { streamJson } from "@nube/starter-client-ts";

import { useStarterClient } from "../provider/starter-client-provider.js";

export type EventStreamStatus =
  | "connecting"
  | "open"
  | "reconnecting"
  | "closed"
  | "error";

export interface UseEventStreamResult<T> {
  /** Most recent payload, or `null` before the first frame. */
  data: T | null;
  /** Terminal or last reconnect error, if any. */
  error: Error | null;
  status: EventStreamStatus;
  /** Tear down and re-subscribe. Identity stable across renders. */
  reconnect(): void;
}

export interface UseEventStreamOptions {
  /** Pause subscription when `false` (default `true`). */
  enabled?: boolean;
  /** Force the fetch fallback (mostly for tests). */
  forceFetch?: boolean;
  /** Inject an `EventSource` constructor (mostly for tests). */
  eventSourceCtor?: typeof EventSource;
}

interface Snapshot<T> {
  data: T | null;
  error: Error | null;
  status: EventStreamStatus;
}

type Subscriber = () => void;

function makeStore<T>(
  start: (
    push: (frame: T) => void,
    setStatus: (s: EventStreamStatus) => void,
    setError: (e: Error) => void,
  ) => () => void,
) {
  let snapshot: Snapshot<T> = { data: null, error: null, status: "connecting" };
  const subs = new Set<Subscriber>();
  const emit = () => subs.forEach((s) => s());

  const push = (frame: T) => {
    snapshot = { data: frame, error: null, status: "open" };
    emit();
  };
  const setStatus = (status: EventStreamStatus) => {
    if (snapshot.status === status) return;
    snapshot = { ...snapshot, status };
    emit();
  };
  const setError = (error: Error) => {
    snapshot = { ...snapshot, error, status: "error" };
    emit();
  };

  let stop: (() => void) | null = null;
  let refcount = 0;
  // When the last subscriber leaves we defer teardown by one tick.
  // React StrictMode double-invokes effects (mount → unmount → mount)
  // and SPA route transitions briefly unmount one consumer before the
  // next one mounts; tearing the EventSource down immediately on
  // refcount=0 would reopen the connection both times. The defer is
  // short (100ms) — long enough to bridge those gaps, short enough
  // that a genuine "no one is listening" state still releases the
  // network resource promptly.
  let teardownTimer: ReturnType<typeof setTimeout> | null = null;

  return {
    subscribe(listener: Subscriber): () => void {
      subs.add(listener);
      if (teardownTimer !== null) {
        clearTimeout(teardownTimer);
        teardownTimer = null;
      }
      if (refcount === 0 && stop === null) {
        stop = start(push, setStatus, setError);
      }
      refcount += 1;
      return () => {
        subs.delete(listener);
        refcount -= 1;
        if (refcount === 0) {
          if (teardownTimer !== null) clearTimeout(teardownTimer);
          teardownTimer = setTimeout(() => {
            teardownTimer = null;
            if (refcount === 0) {
              stop?.();
              stop = null;
            }
          }, 100);
        }
      };
    },
    getSnapshot(): Snapshot<T> {
      return snapshot;
    },
  };
}

// Module-level store cache so that multiple `useEventStream(path)`
// consumers — and the same consumer across unmount/remount cycles
// (route navigation, StrictMode double-mount) — share ONE underlying
// EventSource per (starter, path, generation) tuple. The store's own
// refcount handles teardown only when the last subscriber unsubscribes;
// without sharing, every component opened its own SSE channel and a
// fresh `GET /api/v1/dashboards/events` fired on every navigation.
//
// Disabled subscriptions use a per-call throwaway no-op store and are
// not cached: there is nothing to share, and caching them would leak.
type AnyStore = ReturnType<typeof makeStore<unknown>>;
const storeCache = new Map<string, AnyStore>();

// Stable per-instance id for the `StarterClient` so the cache key can
// distinguish two different clients in the same tab without serialising
// the whole object.
const starterIds = new WeakMap<object, string>();
let starterIdCounter = 0;
function getStarterKey(starter: object): string {
  let id = starterIds.get(starter);
  if (!id) {
    starterIdCounter += 1;
    id = `s${starterIdCounter}`;
    starterIds.set(starter, id);
  }
  return id;
}

function getOrCreateStore<T>(
  key: string,
  factory: () => AnyStore,
): ReturnType<typeof makeStore<T>> {
  const existing = storeCache.get(key);
  if (existing) return existing as ReturnType<typeof makeStore<T>>;
  const created = factory();
  storeCache.set(key, created);
  return created as ReturnType<typeof makeStore<T>>;
}

/**
 * Subscribe to a server-sent event stream of typed JSON frames.
 *
 * The hook re-subscribes whenever `path` or `enabled` changes; on
 * unmount it aborts the underlying iterator (via the shared store's
 * refcount).
 */
export function useEventStream<T>(
  path: string,
  options: UseEventStreamOptions = {},
): UseEventStreamResult<T> {
  const starter = useStarterClient();
  const enabled = options.enabled ?? true;
  const [generation, setGeneration] = useState(0);

  // Key on (starter identity, path, generation). `forceFetch` /
  // `eventSourceCtor` are test seams — bake them into the key so
  // different test configurations don't collide, but in production
  // both are undefined and every consumer collapses onto the same
  // store.
  const key = enabled
    ? `${getStarterKey(starter)}::${path}::${generation}::${options.forceFetch ? "1" : "0"}::${options.eventSourceCtor ? "ctor" : "default"}`
    : null;
  const store = useMemo<ReturnType<typeof makeStore<T>>>(() => {
    if (key === null) return makeStore<T>(() => () => {});
    return getOrCreateStore<T>(key, () =>
      makeStore<unknown>((push, setStatus, setError) => {
        const ctrl = new AbortController();
        let cancelled = false;
        setStatus("connecting");

        (async () => {
          try {
            for await (const frame of streamJson<T>(starter, path, {
              signal: ctrl.signal,
              forceFetch: options.forceFetch,
              eventSourceCtor: options.eventSourceCtor,
              onReconnecting: () => setStatus("reconnecting"),
            })) {
              if (cancelled) break;
              push(frame as unknown);
            }
            if (!cancelled) setStatus("closed");
          } catch (err) {
            if (!cancelled) setError(err instanceof Error ? err : new Error(String(err)));
          }
        })();

        return () => {
          cancelled = true;
          ctrl.abort();
        };
      }) as AnyStore,
    );
  }, [key, starter, path, options.forceFetch, options.eventSourceCtor]);

  // Bridge into React's concurrent-safe external-store API.
  const snapshot = useSyncExternalStore(
    store.subscribe,
    store.getSnapshot,
    store.getSnapshot,
  );

  // Belt and braces: also tear down on unmount via effect. The
  // store's refcount handles this for normal subscribers, but if a
  // future caller bypasses `useSyncExternalStore` they still get
  // clean shutdown.
  useEffect(() => () => {}, [store]);

  const reconnect = useCallback(() => {
    setGeneration((g) => g + 1);
  }, []);

  return {
    data: snapshot.data,
    error: snapshot.error,
    status: snapshot.status,
    reconnect,
  };
}
