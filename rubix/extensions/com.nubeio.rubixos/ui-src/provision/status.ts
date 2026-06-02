// `status.ts` — shared device-status → tone mapping used by the devices
// table and the device page, so a status renders identically everywhere.

export interface StatusTone {
  /** Tailwind bg-* class for the status dot. */
  dot: string;
  /** Tailwind text-* class for the status label. */
  text: string;
}

// Map a backend status to a chip tone + dot color. Unknown statuses
// fall back to a neutral slate dot so nothing renders un-styled.
export function statusTone(status: string): StatusTone {
  const s = status.toLowerCase();
  if (s.includes("fail") || s === "error" || s === "decommissioned")
    return { dot: "bg-rose-500", text: "text-rose-400" };
  if (s.includes("pend") || s.includes("sync"))
    return { dot: "bg-amber-500", text: "text-amber-400" };
  if (s === "active" || s.includes("commission") || s.includes("provision") || s.includes("connect"))
    return { dot: "bg-emerald-500", text: "text-emerald-400" };
  return { dot: "bg-slate-400", text: "text-muted-foreground" };
}
