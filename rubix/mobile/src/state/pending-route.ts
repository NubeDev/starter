// state/pending-route.ts — survives 401 mid-session.
//
// Per APP-SHELL.md §401 mid-session: on a 401 we evict the token and
// preserve the route the operator was on, then re-route to login. After
// a successful re-login we restore the route. Module-level state; only
// one pending route can exist at a time (the most recent navigation
// wins).

interface PendingRoute {
  pathname: string;
  params?: Record<string, string>;
}

let pending: PendingRoute | null = null;

export function setPendingRoute(route: PendingRoute | null): void {
  pending = route;
}

export function takePendingRoute(): PendingRoute | null {
  const out = pending;
  pending = null;
  return out;
}
