// Process-wide telemetry sink for i18n runtime events. Names match
// `examples/notes/user-pref.md` § Telemetry — production dashboards
// key off these strings, do not rename without bumping the SCOPE.
//
// The two events:
//
//   * `i18n.locale_fallback`  (info)  — first time the resolver falls
//     back to a shorter tag (or to the `en` floor) for a given locale
//     in a session. The provider de-dupes per session, this module
//     only forwards.
//   * `i18n.message_missing`  (warn)  — `useTranslate` /
//     `useHostTranslate` resolved a key the catalog had no entry for.
//     The runtime returns the id verbatim (react-intl default) so the
//     UI doesn't crash; the counter is what tells the platform team
//     a catalog has a gap.
//
// Same shape as `setExtensionMessageTelemetry`: a process-wide setter
// that returns a `dispose`, plus a runtime emit helper. The host
// installs the sink during bootstrap; tests inject + restore.

import type { LanguageTag } from "./types.js";

export type I18nTelemetryEvent =
  | {
      kind: "i18n.locale_fallback";
      severity: "info";
      /** What the consumer asked for. */
      requested: LanguageTag;
      /** What the resolver picked (the active catalog). */
      picked: LanguageTag;
      /** The walk that got us there, requested-first, picked-last. */
      chain: ReadonlyArray<LanguageTag>;
    }
  | {
      kind: "i18n.message_missing";
      severity: "warn";
      /** The fully-qualified key the caller asked for. */
      key: string;
      /** The locale the catalog was loaded for. */
      language: LanguageTag;
      /** The extension id (when called via `useHostTranslate`), or
       *  `null` for platform `useTranslate` calls. */
      extensionId: string | null;
    };

export type I18nTelemetrySink = (event: I18nTelemetryEvent) => void;

let sink: I18nTelemetrySink | null = null;

/** Install (or remove) the process-wide i18n telemetry sink. Returns
 *  a `dispose` that restores the previous sink — pairs cleanly with
 *  React effects in tests. */
export function setI18nTelemetry(next: I18nTelemetrySink | null): () => void {
  const prev = sink;
  sink = next;
  return () => {
    sink = prev;
  };
}

/** Emit one event. Swallows sink-side exceptions so a misbehaving
 *  dashboard observer cannot crash a render. */
export function emitI18nTelemetry(event: I18nTelemetryEvent): void {
  if (!sink) {
    // Dev convenience: surface missing-key warnings to the console so
    // the author sees the gap. In prod (no dev mode flag here), this
    // is still a single console.warn — the cost is one log line per
    // unique key per render path, far cheaper than a Sentry round-
    // trip. Keep behind `process.env.NODE_ENV !== "production"` to
    // match react-intl's own posture.
    if (
      event.kind === "i18n.message_missing" &&
      typeof process !== "undefined" &&
      process.env?.NODE_ENV !== "production"
    ) {
      // eslint-disable-next-line no-console
      console.warn(
        `[starter-ui-core/i18n] missing translation: ${event.key} (${event.language})`,
      );
    }
    return;
  }
  try {
    sink(event);
  } catch (err) {
    // eslint-disable-next-line no-console
    console.warn("[starter-ui-core/i18n] telemetry sink threw:", err);
  }
}

/** Test helper — wipe the sink. */
export function _resetI18nTelemetryForTesting(): void {
  sink = null;
}
