// Unit tests for the CSRF double-submit helpers. `csrfHeaderForMethod`
// is the method-aware gate `fetchJson`/`fetchVoid` use to auto-attach
// the token on mutations only.

import { afterEach, describe, expect, it, vi } from "vitest";

import { csrfHeaderForMethod, readCsrfHeader } from "./csrf.js";

// `document` is undefined in the node test env by default; stub a cookie
// jar when a test needs the token to be readable.
function withCookie(value: string | undefined, fn: () => void): void {
  if (value === undefined) {
    vi.stubGlobal("document", undefined);
  } else {
    vi.stubGlobal("document", { cookie: `starter_csrf=${value}` });
  }
  try {
    fn();
  } finally {
    vi.unstubAllGlobals();
  }
}

afterEach(() => vi.unstubAllGlobals());

describe("csrfHeaderForMethod", () => {
  it("attaches the token on mutating methods", () => {
    withCookie("tok123", () => {
      for (const m of ["POST", "PUT", "PATCH", "DELETE", "post", "Delete"]) {
        expect(csrfHeaderForMethod(m)).toEqual({ "X-CSRF-Token": "tok123" });
      }
    });
  });

  it("omits the token on safe methods", () => {
    withCookie("tok123", () => {
      for (const m of ["GET", "HEAD", "OPTIONS", undefined]) {
        expect(csrfHeaderForMethod(m)).toEqual({});
      }
    });
  });

  it("is a no-op when the cookie is absent (non-browser / logged out)", () => {
    withCookie(undefined, () => {
      expect(csrfHeaderForMethod("POST")).toEqual({});
    });
  });
});

describe("readCsrfHeader", () => {
  it("reads the named cookie", () => {
    withCookie("abc", () => {
      expect(readCsrfHeader()).toEqual({ "X-CSRF-Token": "abc" });
    });
  });

  it("returns empty when no document", () => {
    withCookie(undefined, () => {
      expect(readCsrfHeader()).toEqual({});
    });
  });
});
