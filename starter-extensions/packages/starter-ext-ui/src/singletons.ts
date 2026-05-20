// Singleton negotiation.
//
// The host provides one instance of each well-known shared package
// (React, react-dom, `@tanstack/react-query`, `zustand`). Every
// extension declares the same packages with the version it was
// built against. The host enforces a matching-majors check
// (SCOPE.md §"Decisions made" / singleton-mismatch); on mismatch
// the extension's lifecycle state goes to `Failed` with reason
// `singleton-mismatch: <pkg>@<expected> vs <actual>`, and the host
// does not register the remote.
//
// The intentionally narrow design choice — *majors only* — comes
// from React's compatibility story: a React 18.x host can run an
// extension built against React 18.x even if minor versions differ;
// a React 18 host running an extension built against React 19 is
// not guaranteed to work, so it is refused at load time. Operators
// who want a more permissive policy can wrap the host manager.

/**
 * Extract the major-version number from a semver string. Returns
 * `null` for inputs the parser cannot interpret.
 *
 * The implementation deliberately tolerates loose inputs (`"^18.3"`,
 * `"18"`, `"18.3.1-rc.1"`) so consumers using slightly non-standard
 * version strings still get a sensible major.
 */
export function parseMajor(version: string): number | null {
  const m = /^[~^=><\s]*(\d+)/.exec(version);
  if (!m) return null;
  const major = m[1];
  if (major === undefined) return null;
  const n = Number.parseInt(major, 10);
  return Number.isFinite(n) ? n : null;
}

/**
 * Compare two semver strings on major only. `null` for either side
 * means "could not parse" — that's treated as a mismatch so a
 * malformed declaration is refused rather than silently waved
 * through.
 */
export function matchingMajor(a: string, b: string): boolean {
  const am = parseMajor(a);
  const bm = parseMajor(b);
  if (am === null || bm === null) return false;
  return am === bm;
}

/**
 * Extract the minor-version number from a semver string. Returns
 * `null` for inputs the parser cannot interpret. Used by the
 * host's drift detector — if the host runs a higher minor than the
 * extension declared, the load is still compatible but worth
 * surfacing as a `extension.singleton_minor_drift` telemetry event
 * so the platform team has a tripwire when adoption lags.
 */
export function parseMinor(version: string): number | null {
  const m = /^[~^=><\s]*\d+\.(\d+)/.exec(version);
  if (!m) return null;
  const minor = m[1];
  if (minor === undefined) return null;
  const n = Number.parseInt(minor, 10);
  return Number.isFinite(n) ? n : null;
}

/**
 * Well-known singleton ids. Singleton keys are the package name +
 * subpath an extension would `import` (per D-NP.1) — using the same
 * convention as React/ReactDOM keeps the table consistent.
 */
export const SINGLETON_REACT = "react" as const;
export const SINGLETON_REACT_DOM = "react-dom" as const;
export const SINGLETON_UI_CORE_PREFERENCES = "@nube/starter-ui-core/preferences" as const;
export const SINGLETON_UI_CORE_I18N = "@nube/starter-ui-core/i18n" as const;

/**
 * Diagnostic carried by a minor-drift telemetry event.
 */
export interface SingletonMinorDrift {
  pkg: string;
  hostVersion: string;
  extensionVersion: string;
  /** Host minor minus extension minor (always positive when reported). */
  driftMinors: number;
}

/**
 * Diagnostic carried by `SingletonMismatchError`. Adapter code
 * surfaces these on `GET /extensions/<id>` so an operator sees
 * exactly why a remote refused to load.
 */
export interface SingletonMismatchReason {
  /** Package name the extension declared. */
  pkg: string;
  /** Version string the host provides. */
  hostVersion: string;
  /** Version string the extension was built against. */
  extensionVersion: string;
  /** Human-readable reason (used as the error message). */
  reason: string;
}

/**
 * Thrown by `ExtensionHostManager.registerExtensionRemote` when a
 * remote's singleton declaration is incompatible with the host's
 * provided singletons. The host does *not* swallow this — it lets
 * the error propagate so the caller (the host shell's bootstrap)
 * can mark the extension's lifecycle state as `Failed` with the
 * carried reason.
 */
export class SingletonMismatchError extends Error {
  readonly extensionId: string;
  readonly reasons: readonly SingletonMismatchReason[];

  constructor(extensionId: string, reasons: SingletonMismatchReason[]) {
    super(
      `singleton-mismatch for ${extensionId}: ` +
        reasons.map((r) => r.reason).join("; "),
    );
    this.name = "SingletonMismatchError";
    this.extensionId = extensionId;
    this.reasons = reasons;
  }
}
