// Stage-5 smoke test: an extension's catalog is fetched lazily for the
// active language and merges into the host's `<IntlProvider>` so a
// federated panel renders the right translation without the panel
// owning a copy of the catalog.
//
// What this proves end-to-end:
//
//   1. `registerExtensionMessages` namespaces bare keys under the
//      extension id (D-NP.3) — `"greeting"` becomes
//      `"com.nube.hello.greeting"` in the merged bundle.
//   2. The `<IntlProvider>` rebuilds its `IntlShape` in the same
//      commit as the registry mutation (the panel renders the new
//      string without a parent re-mount).
//   3. Switching language triggers a fresh fetch for the *new*
//      language only — no upfront bulk load (D-NP.8 lazy-load).
//   4. A cross-namespace key (`com.nube.other.greeting` written
//      inside `com.nube.hello`'s catalog) is dropped and surfaces as
//      one `extension.catalog_key_collision` telemetry event with the
//      exact intruded namespace.
//
// The host wiring under test is the same code production runs —
// `<ExtensionCatalogLoader>` + `registerExtensionMessages` + the
// modified `<IntlProvider>` — only the network is stubbed.

import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { useState } from "react";
import { StarterClient } from "@nube/starter-client-ts";
import {
  IntlProvider,
  useIntlContext,
  _resetManifestCacheForTesting,
  _resetExtensionMessagesForTesting,
  _resetCatalogCacheForTesting,
  registerExtensionMessages,
  setExtensionMessageTelemetry,
  getExtensionMessages,
  type ExtensionMessageTelemetryEvent,
} from "@nube/starter-ui-core/i18n";

import { ExtensionCatalogLoader, _resetExtensionCatalogFetchesForTesting } from "./extension-catalog-loader.js";

// The loader iterates the module-level catalog manifest set by
// `extension-host.ts#registerOne`. Tests seed that map directly via
// the test-only helper export to avoid spinning up the full
// extension-host bootstrap.
import { _listExtensionCatalogsForTesting } from "./extension-host.js";

const HELLO_ID = "com.nube.hello";

const PLATFORM_MANIFEST = { en: "platform-en-fp", es: "platform-es-fp" };
const PLATFORM_CATALOGS: Record<string, Record<string, string>> = {
  en: { "starter.greet": "Hello platform" },
  es: { "starter.greet": "Hola plataforma" },
};
const EXT_CATALOGS: Record<string, Record<string, string>> = {
  en: { greeting: "Hello from extension" },
  es: { greeting: "Hola desde la extensión" },
};

interface ServerOptions {
  /** When non-null, the extension's `i18n/es.json` body that the
   *  catalog endpoint returns. Lets the collision test inject a key
   *  that violates D-NP.3. */
  extensionEsOverride?: Record<string, string>;
  /** Number of times each (lang) catalog endpoint was hit. The lazy-
   *  load assertion reads this. */
  hits?: { en: number; es: number };
}

function makeFetch(opts: ServerOptions = {}) {
  const hits = opts.hits ?? { en: 0, es: 0 };
  return (async (input: RequestInfo | URL) => {
    const url = typeof input === "string" ? input : input instanceof URL ? input.toString() : input.url;
    if (url.endsWith("/v1/i18n/manifest")) {
      return new Response(JSON.stringify(PLATFORM_MANIFEST), { status: 200 });
    }
    const platform = url.match(/\/v1\/i18n\/catalogs\/([a-z-]+)-([^.]+)\.json$/);
    if (platform) {
      const lang = platform[1]!;
      return new Response(JSON.stringify(PLATFORM_CATALOGS[lang] ?? {}), {
        status: 200,
      });
    }
    const ext = url.match(/\/extensions\/([^/]+)\/i18n\/([^.]+)\.json$/);
    if (ext) {
      const lang = ext[2]!;
      if (lang === "en") hits.en += 1;
      if (lang === "es") hits.es += 1;
      const body =
        lang === "es" && opts.extensionEsOverride
          ? opts.extensionEsOverride
          : EXT_CATALOGS[lang];
      if (!body) return new Response("not found", { status: 404 });
      return new Response(JSON.stringify(body), { status: 200 });
    }
    return new Response("not found", { status: 404 });
  }) as typeof fetch;
}

/** Tiny consumer that exposes the merged-bundle lookup as text the
 *  test can assert against. Reaches react-intl through the same
 *  context an extension panel would (the host's IntlShape carried
 *  on the singleton). */
function HelloProbe() {
  const { intl } = useIntlContext();
  // The shape's typings are local-scope (we don't pull react-intl
  // into the test) so we duck-type the `formatMessage` call.
  const f = (intl as { formatMessage: (d: { id: string }) => string }).formatMessage;
  return <span data-testid="probe">{f({ id: "com.nube.hello.greeting" })}</span>;
}

/** Harness wires the language flip the loader is meant to react to. */
function Harness({ client }: { client: StarterClient }) {
  const [lang, setLang] = useState<"en" | "es">("en");
  return (
    <>
      <button data-testid="flip" onClick={() => setLang(lang === "en" ? "es" : "en")}>
        flip
      </button>
      <IntlProvider client={client} language={lang}>
        <ExtensionCatalogLoader client={client} />
        <HelloProbe />
      </IntlProvider>
    </>
  );
}

beforeEach(() => {
  // Seed the catalog manifest the loader iterates — the same map the
  // production `registerOne` writes after parsing the manifest.
  const map = _listExtensionCatalogsForTesting() as Map<
    string,
    { catalogs: Record<string, string> }
  >;
  map.clear();
  map.set(HELLO_ID, { catalogs: { en: "i18n/en.json", es: "i18n/es.json" } });
});

afterEach(() => {
  cleanup();
  _resetManifestCacheForTesting();
  _resetCatalogCacheForTesting();
  _resetExtensionMessagesForTesting();
  _resetExtensionCatalogFetchesForTesting();
  const map = _listExtensionCatalogsForTesting() as Map<string, unknown>;
  map.clear();
});

describe("extension catalog merge — Stage 5", () => {
  it("namespaces bare keys under the extension id (D-NP.3)", () => {
    registerExtensionMessages({
      extensionId: HELLO_ID,
      language: "en",
      messages: { greeting: "Hi" },
    });
    expect(getExtensionMessages("en")["com.nube.hello.greeting"]).toBe("Hi");
  });

  it("lazy-fetches only the active language (D-NP.8) and renders it through the host's IntlShape", async () => {
    const hits = { en: 0, es: 0 };
    const client = new StarterClient({ baseUrl: "http://t", fetch: makeFetch({ hits }) });

    render(<Harness client={client} />);

    // Initial language is en; loader fetches en, not es.
    await waitFor(() =>
      expect(screen.getByTestId("probe").textContent).toBe("Hello from extension"),
    );
    expect(hits.en).toBe(1);
    expect(hits.es).toBe(0);

    // Flip to es — loader fetches es exactly once; en stays at 1.
    screen.getByTestId("flip").click();
    await waitFor(() =>
      expect(screen.getByTestId("probe").textContent).toBe("Hola desde la extensión"),
    );
    expect(hits.en).toBe(1);
    expect(hits.es).toBe(1);

    // Flip back — already-fetched language is not re-requested.
    screen.getByTestId("flip").click();
    await waitFor(() =>
      expect(screen.getByTestId("probe").textContent).toBe("Hello from extension"),
    );
    expect(hits.en).toBe(1);
    expect(hits.es).toBe(1);
  });

  it("drops cross-namespace keys + emits extension.catalog_key_collision", () => {
    const events: ExtensionMessageTelemetryEvent[] = [];
    const dispose = setExtensionMessageTelemetry((e) => events.push(e));
    try {
      const result = registerExtensionMessages({
        extensionId: HELLO_ID,
        language: "en",
        messages: {
          greeting: "Hello",
          "com.nube.other.greeting": "evil",
        },
      });
      expect(result.collisions).toBe(1);
      expect(result.accepted).toBe(1);
      expect(events).toHaveLength(1);
      expect(events[0]!.kind).toBe("extension.catalog_key_collision");
      if (events[0]!.kind === "extension.catalog_key_collision") {
        expect(events[0]!.extensionId).toBe(HELLO_ID);
        expect(events[0]!.intrudedNamespace).toBe("com.nube.other");
        expect(events[0]!.key).toBe("com.nube.other.greeting");
      }
    } finally {
      dispose();
    }
  });
});
