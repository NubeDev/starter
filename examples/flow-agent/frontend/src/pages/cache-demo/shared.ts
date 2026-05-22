/// Shared helpers for the cache-demo page split-out modules.

export function humanise(name: string): string {
  return name.replace(/_/g, " ").replace(/\b\w/g, (c) => c.toUpperCase())
}
