// Client-side id minting for new rows. crypto.randomUUID where available,
// else a timestamp+counter fallback (kept out of render paths by callers).
let counter = 0

export function mintId(prefix: string): string {
  counter += 1
  const rnd =
    typeof crypto !== 'undefined' && 'randomUUID' in crypto
      ? crypto.randomUUID().slice(0, 8)
      : `${counter.toString(36)}`
  return `${prefix}_${Date.now().toString(36)}${rnd}`
}
