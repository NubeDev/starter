// Bridge between a stored absolute bound (RFC-3339 / ISO instant) and the
// value an `<input type="datetime-local">` wants (local wall-clock, no zone,
// minute precision). Relative tokens (`now-6h`) have no datetime-local form,
// so the absolute tab seeds from the *resolved* instant instead.

import { resolveBound } from "@/store/time/resolve";

/** ISO instant -> `datetime-local` string (`YYYY-MM-DDTHH:mm`) in local time.
 *  A relative token is first resolved against `now` so the field shows the
 *  concrete instant the user is currently looking at. */
export function toDatetimeLocal(bound: string, now: Date): string {
  let d: Date;
  try {
    d = resolveBound(bound, now);
  } catch {
    d = now;
  }
  const pad = (n: number) => String(n).padStart(2, "0");
  return (
    `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}` +
    `T${pad(d.getHours())}:${pad(d.getMinutes())}`
  );
}

/** `datetime-local` string -> stored absolute bound (UTC ISO). The input is
 *  local wall-clock; `new Date(local)` interprets it in the local zone, which
 *  is what the user typed. Returns the canonical ISO instant. */
export function fromDatetimeLocal(local: string): string {
  return new Date(local).toISOString();
}
