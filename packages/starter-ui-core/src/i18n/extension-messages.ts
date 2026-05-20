// Extension catalog registry — the host-side merge target for
// `registerExtensionMessages`.
//
// The flow (`examples/notes/user-pref.md` § Stage 5):
//
// 1. The notes host's `extension-host.ts` fetches
//    `GET /extensions/<id>/i18n/<activeLang>.json` for every loaded
//    extension whose manifest declares `contributes.i18n.catalogs`.
// 2. It calls `registerExtensionMessages({extensionId, language,
//    messages})` with the JSON body. Bare keys (no dot) are
//    prefixed with the extension id (`com.nube.hello.greeting`); keys
//    already starting with the extension id pass through; any key
//    inside *another* extension's namespace is **dropped** and
//    `extension.catalog_key_collision` telemetry fires.
// 3. `<IntlProvider>` subscribes to the registry and merges the
//    extension messages into its react-intl bundle every time the
//    registry notifies — extensions appear in the active language
//    without a host re-mount.
//
// Lazy-load is enforced at the *caller* (the host only fetches the
// active language); the registry simply stores whatever it is given.
// Switching language is "load the new lang catalog, register, drop
// the old lang from the registry" — no eviction policy here.

import type { Catalog, LanguageTag } from "./types.js";

/**
 * Telemetry events emitted by [`registerExtensionMessages`]. Names
 * match `examples/notes/user-pref.md` § Telemetry; production
 * dashboards key off these exact strings.
 */
export type ExtensionMessageTelemetryEvent = {
  kind: "extension.catalog_key_collision";
  severity: "warn";
  /** The extension that owned the catalog file the offending key
   * lived in. */
  extensionId: string;
  /** Language tag of the catalog. */
  language: LanguageTag;
  /** The fully-qualified key that targeted a different namespace. */
  key: string;
  /** The extension id the key tried to write into. */
  intrudedNamespace: string;
};

/** Sink for catalog-collision telemetry. Should be cheap +
 *  non-throwing — the registry swallows any exception so a
 *  misbehaving observer cannot break a language flip. */
export type ExtensionMessageTelemetrySink = (
  event: ExtensionMessageTelemetryEvent,
) => void;

let telemetry: ExtensionMessageTelemetrySink | null = null;

/** Install (or remove) the process-wide telemetry sink. The notes
 * host wires this to its `extension.*` event bus so collisions surface
 * on the registry detail page. Returns a `dispose` that restores the
 * previous sink — pairs cleanly with React effects in tests. */
export function setExtensionMessageTelemetry(
  sink: ExtensionMessageTelemetrySink | null,
): () => void {
  const prev = telemetry;
  telemetry = sink;
  return () => {
    telemetry = prev;
  };
}

interface PerLanguage {
  /** Stored messages, keys already namespaced. */
  messages: Record<string, string>;
}

interface PerExtension {
  byLanguage: Map<LanguageTag, PerLanguage>;
}

const registry = new Map<string, PerExtension>();
const listeners = new Set<() => void>();
// Bumps on every successful mutation. Consumers (`useSyncExternalStore`-
// style) read this to spot changes without a structural diff.
let version = 0;

function notify(): void {
  version += 1;
  for (const l of listeners) {
    try {
      l();
    } catch (err) {
      // eslint-disable-next-line no-console
      console.warn("[starter-ui-core/i18n] extension-messages listener threw:", err);
    }
  }
}

/**
 * Subscribe to registry mutations. Returns an unsubscribe function.
 * The IntlProvider wires this through `useSyncExternalStore` so a
 * language flip + a fresh extension catalog rebuild the merged bundle
 * in one commit.
 */
export function subscribeExtensionMessages(listener: () => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

/** Monotone version counter — used as the `getSnapshot` value for
 * external-store integrations. Cheap to compare; the actual merged
 * messages are pulled separately. */
export function extensionMessagesVersion(): number {
  return version;
}

/**
 * Return a flat `{key: message}` map of every registered extension's
 * messages for the given language. Keys are already namespaced
 * (`com.nube.hello.greeting`). Empty when no extension has registered
 * anything for that language.
 *
 * The result is a fresh object on every call; callers that need
 * referential stability should memoise on
 * [`extensionMessagesVersion`].
 */
export function getExtensionMessages(language: LanguageTag): Catalog {
  const out: Record<string, string> = {};
  for (const ext of registry.values()) {
    const per = ext.byLanguage.get(language);
    if (!per) continue;
    Object.assign(out, per.messages);
  }
  return out;
}

export interface RegisterExtensionMessagesInput {
  /** Reverse-DNS extension id (matches `block.yaml`). */
  extensionId: string;
  /** Language tag of the catalog (BCP-47). */
  language: LanguageTag;
  /** Raw catalog as shipped on disk. Bare keys are prefixed with the
   * extension id; fully-qualified keys are kept verbatim except when
   * they target another extension's namespace (then they are dropped
   * and `extension.catalog_key_collision` fires). */
  messages: Catalog;
}

/**
 * Merge one extension's catalog (one language) into the host's
 * registry. Returns the count of accepted keys.
 *
 * `examples/notes/user-pref.md` D-NP.3 — namespacing rules:
 *
 *   * `"greeting"`                  → `"com.nube.hello.greeting"`
 *   * `"com.nube.hello.greeting"`   → kept verbatim
 *   * `"com.nube.other.greeting"`   → dropped + telemetry
 *
 * Calling twice for the same (extensionId, language) replaces the
 * prior catalog — keeps the dev-watcher hot-reload story simple.
 */
export function registerExtensionMessages(
  input: RegisterExtensionMessagesInput,
): { accepted: number; collisions: number } {
  const { extensionId, language, messages } = input;
  const namespacePrefix = `${extensionId}.`;
  const accepted: Record<string, string> = {};
  let collisions = 0;

  for (const [rawKey, value] of Object.entries(messages)) {
    if (typeof value !== "string") continue;
    let fullKey: string;
    if (!rawKey.includes(".")) {
      // Bare key → auto-prefix.
      fullKey = namespacePrefix + rawKey;
    } else if (rawKey === extensionId || rawKey.startsWith(namespacePrefix)) {
      // Already in our namespace.
      fullKey = rawKey;
    } else {
      // Fully-qualified key in a different namespace → reject + emit.
      collisions += 1;
      // The extension id is everything *before* the last dot in a
      // fully-qualified key (`com.nube.other.greeting` →
      // `com.nube.other`). Reverse-DNS ids are variable depth, so the
      // last segment is the only stable "key name" boundary.
      const dot = rawKey.lastIndexOf(".");
      const intruded = dot > 0 ? rawKey.slice(0, dot) : rawKey;
      if (telemetry) {
        try {
          telemetry({
            kind: "extension.catalog_key_collision",
            severity: "warn",
            extensionId,
            language,
            key: rawKey,
            intrudedNamespace: intruded,
          });
        } catch (err) {
          // eslint-disable-next-line no-console
          console.warn("[starter-ui-core/i18n] collision telemetry threw:", err);
        }
      } else {
        // eslint-disable-next-line no-console
        console.warn(
          `[starter-ui-core/i18n] extension ${extensionId} dropped key "${rawKey}" — outside its namespace.`,
        );
      }
      continue;
    }
    accepted[fullKey] = value;
  }

  let perExt = registry.get(extensionId);
  if (!perExt) {
    perExt = { byLanguage: new Map() };
    registry.set(extensionId, perExt);
  }
  perExt.byLanguage.set(language, { messages: accepted });
  notify();
  return { accepted: Object.keys(accepted).length, collisions };
}

/** Drop every catalog for an extension — invoked when the host
 * unregisters the remote (extension disabled). */
export function unregisterExtensionMessages(extensionId: string): void {
  if (registry.delete(extensionId)) notify();
}

/** Test helper — wipe the entire registry. */
export function _resetExtensionMessagesForTesting(): void {
  registry.clear();
  version = 0;
  telemetry = null;
}
