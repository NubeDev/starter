// Process-wide telemetry sink for preferences runtime events. Names
// match `examples/notes/user-pref.md` § Telemetry.
//
// Currently emits one event:
//
//   * `prefs.broadcast_dropped` (warn) — `BroadcastChannel.postMessage`
//     threw or the channel was not constructible (older browsers /
//     hardened environments where the API is gated). The mutation
//     itself still went through; only the cross-tab fan-out failed.

export type PreferencesTelemetryEvent = {
  kind: "prefs.broadcast_dropped";
  severity: "warn";
  /** The patch that did not fan out, JSON-safe. */
  patch: Readonly<Record<string, unknown>>;
  /** The error captured from `BroadcastChannel.postMessage` (or the
   *  reason the channel could not be constructed). */
  reason: string;
};

export type PreferencesTelemetrySink = (event: PreferencesTelemetryEvent) => void;

let sink: PreferencesTelemetrySink | null = null;

/** Install (or remove) the process-wide preferences telemetry sink.
 *  Returns a `dispose` that restores the previous sink. */
export function setPreferencesTelemetry(
  next: PreferencesTelemetrySink | null,
): () => void {
  const prev = sink;
  sink = next;
  return () => {
    sink = prev;
  };
}

/** Emit one event. Swallows sink-side exceptions so a misbehaving
 *  observer cannot break a render. */
export function emitPreferencesTelemetry(event: PreferencesTelemetryEvent): void {
  if (!sink) return;
  try {
    sink(event);
  } catch (err) {
    // eslint-disable-next-line no-console
    console.warn("[starter-ui-core/preferences] telemetry sink threw:", err);
  }
}

/** Test helper — wipe the sink. */
export function _resetPreferencesTelemetryForTesting(): void {
  sink = null;
}
