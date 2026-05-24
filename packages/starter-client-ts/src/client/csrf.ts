// CSRF helper. Mutating endpoints must echo the value of the
// `starter_csrf` cookie back as the `X-CSRF-Token` header so the
// server can verify the request originated from the same browser
// session. The cookie is set by the server on login; we read it from
// `document.cookie` (the only place a browser exposes it to JS).
//
// In non-browser environments (SSR, tests without `document`) this is
// a no-op returning an empty header map — callers spread it into their
// request headers unconditionally.

/** Read the named cookie's value from `document.cookie`, or undefined
 * when the cookie is absent or `document` is unavailable. */
export function readCookie(name: string): string | undefined {
  if (typeof document === "undefined") return undefined;
  for (const part of document.cookie.split(";")) {
    const [k, v] = part.trim().split("=");
    if (k === name) return v;
  }
  return undefined;
}

/** Build the CSRF header map for a mutating request. Returns an empty
 * object when the cookie is missing so the caller can spread it
 * unconditionally. */
export function readCsrfHeader(cookieName: string = "starter_csrf"): Record<string, string> {
  const csrf = readCookie(cookieName);
  return csrf ? { "X-CSRF-Token": csrf } : {};
}
