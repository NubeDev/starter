// Pure resolution of a relative-or-absolute time range into a concrete
// `{from, to}` instant pair. Kept free of React and the store so it is
// trivially unit-testable and reusable by the query layer (the binder
// expects absolute instants — it never interprets `now`).
//
// A bound is either an absolute RFC-3339 string or a relative token:
//   `now`            — the reference instant
//   `now-<n><unit>`  — that many units before `now` (s|m|h|d|w|M|y)
//   `now/<unit>`     — `now` rounded down to the start of the unit
//   `now-<n><unit>/<unit>` — shift then round (Grafana semantics)
// Resolution takes the reference `now` as an argument so every panel in
// one refresh resolves against a single frozen instant (no fan-out skew).

/** A time-range bound: an absolute ISO instant or a relative `now…` token. */
export type TimeBound = string;

/** A dashboard time range, stored as the user expressed it (relative tokens
 *  survive a reload and keep tracking `now`). */
export interface TimeRange {
  from: TimeBound;
  to: TimeBound;
}

/** A resolved absolute window: both bounds are concrete instants. */
export interface ResolvedRange {
  from: Date;
  to: Date;
}

const UNIT_MS: Record<string, number> = {
  s: 1_000,
  m: 60_000,
  h: 3_600_000,
  d: 86_400_000,
  w: 604_800_000,
};

// `now-3h`, `now-15m`, with an optional `/unit` rounding suffix; or a bare
// `now`, or `now/d`. Calendar units (M, y) are handled out of band because
// they aren't fixed-width.
const RELATIVE = /^now(?:-(\d+)([smhdwMy]))?(?:\/([smhdwMy]))?$/;

/** Resolve one bound against a frozen `now`. Throws on an unparseable token
 *  so a malformed URL/state surfaces rather than silently snapping to `now`. */
export function resolveBound(bound: TimeBound, now: Date): Date {
  const m = RELATIVE.exec(bound.trim());
  if (!m) {
    const abs = new Date(bound);
    if (Number.isNaN(abs.getTime())) {
      throw new Error(`unparseable time bound: ${bound}`);
    }
    return abs;
  }

  const [, amount, shiftUnit, roundUnit] = m;
  let t = now.getTime();

  if (amount && shiftUnit) {
    t = subtract(new Date(t), Number(amount), shiftUnit).getTime();
  }
  if (roundUnit) {
    t = floorToUnit(new Date(t), roundUnit).getTime();
  }
  return new Date(t);
}

/** Resolve a whole range against one frozen `now`. */
export function resolveTimeRange(range: TimeRange, now: Date): ResolvedRange {
  return {
    from: resolveBound(range.from, now),
    to: resolveBound(range.to, now),
  };
}

// Subtract `n` of a unit. Months and years are calendar-aware (variable
// width); the rest are fixed-width millisecond arithmetic.
function subtract(d: Date, n: number, unit: string): Date {
  if (unit === "M") {
    const r = new Date(d);
    r.setMonth(r.getMonth() - n);
    return r;
  }
  if (unit === "y") {
    const r = new Date(d);
    r.setFullYear(r.getFullYear() - n);
    return r;
  }
  return new Date(d.getTime() - n * UNIT_MS[unit]);
}

// Round down to the start of the unit (local-time calendar boundaries for
// d/w/M/y so "Today" means the user's midnight, fixed-width for s/m/h).
function floorToUnit(d: Date, unit: string): Date {
  const r = new Date(d);
  switch (unit) {
    case "s":
      r.setMilliseconds(0);
      return r;
    case "m":
      r.setSeconds(0, 0);
      return r;
    case "h":
      r.setMinutes(0, 0, 0);
      return r;
    case "d":
      r.setHours(0, 0, 0, 0);
      return r;
    case "w": {
      r.setHours(0, 0, 0, 0);
      // ISO-ish: week starts Monday. getDay() is 0=Sun..6=Sat.
      const day = (r.getDay() + 6) % 7;
      r.setDate(r.getDate() - day);
      return r;
    }
    case "M":
      r.setHours(0, 0, 0, 0);
      r.setDate(1);
      return r;
    case "y":
      r.setHours(0, 0, 0, 0);
      r.setMonth(0, 1);
      return r;
    default:
      return r;
  }
}
