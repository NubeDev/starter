// Tiny document.cookie helpers for UI-preference persistence (sidebar
// open state, layout variant). These hold user *preferences*, never app
// data (F0) — the same category as the zustand UI store.
const DEFAULT_MAX_AGE = 60 * 60 * 24 * 7; // 7 days

export function getCookie(name: string): string | undefined {
  if (typeof document === "undefined") return undefined;
  const parts = `; ${document.cookie}`.split(`; ${name}=`);
  if (parts.length === 2) return parts.pop()?.split(";").shift();
  return undefined;
}

export function setCookie(
  name: string,
  value: string,
  maxAge: number = DEFAULT_MAX_AGE,
): void {
  if (typeof document === "undefined") return;
  document.cookie = `${name}=${value}; path=/; max-age=${maxAge}`;
}
