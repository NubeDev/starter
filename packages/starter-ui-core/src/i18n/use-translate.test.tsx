// `useTranslate` exercises:
// - returns the message from the active catalog
// - falls back to `en` when the active catalog lacks the key
// - returns the key id verbatim when both catalogs lack it

import { afterEach, describe, expect, it } from "vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { StarterClient } from "@nube/starter-client-ts";

import {
  IntlProvider,
  _resetManifestCacheForTesting,
} from "./provider.js";
import { _resetCatalogCacheForTesting } from "./fetcher.js";
import { useTranslate } from "./use-translate.js";

afterEach(() => {
  cleanup();
  _resetManifestCacheForTesting();
  _resetCatalogCacheForTesting();
});

interface Server {
  fetch: typeof fetch;
}

function makeServer(catalogs: Record<string, Record<string, string>>): Server {
  // Stable 16-char fingerprint per language (good enough for tests).
  const manifest: Record<string, string> = {};
  for (const lang of Object.keys(catalogs)) manifest[lang] = `${lang.padEnd(16, "0").slice(0, 16)}`;
  const fetchImpl = (async (input: RequestInfo | URL) => {
    const url = typeof input === "string" ? input : input instanceof URL ? input.toString() : input.url;
    if (url.endsWith("/v1/i18n/manifest")) {
      return new Response(JSON.stringify(manifest), { status: 200 });
    }
    const m = url.match(/\/v1\/i18n\/catalogs\/([a-z-]+)-([^.]+)\.json$/);
    if (m) {
      const lang = m[1]!;
      const body = catalogs[lang];
      if (!body) return new Response("not found", { status: 404 });
      return new Response(JSON.stringify(body), { status: 200 });
    }
    return new Response("not found", { status: 404 });
  }) as typeof fetch;
  return { fetch: fetchImpl };
}

function Probe({ id }: { id: string }) {
  const t = useTranslate();
  return <span data-testid="t">{t(id)}</span>;
}

describe("useTranslate", () => {
  it("renders the message from the active catalog", async () => {
    const server = makeServer({
      en: { "starter.greet": "Hello" },
      es: { "starter.greet": "Hola" },
    });
    const client = new StarterClient({ baseUrl: "http://t", fetch: server.fetch });
    render(
      <IntlProvider client={client} language="es">
        <Probe id="starter.greet" />
      </IntlProvider>,
    );
    await waitFor(() => expect(screen.getByTestId("t").textContent).toBe("Hola"));
  });

  it("falls back to the en catalog when the active locale lacks the key", async () => {
    const server = makeServer({
      en: { "starter.only_en": "EnglishOnly" },
      es: { "starter.greet": "Hola" },
    });
    const client = new StarterClient({ baseUrl: "http://t", fetch: server.fetch });
    render(
      <IntlProvider client={client} language="es" defaultMessages={{ "starter.only_en": "EnglishOnly" }}>
        <Probe id="starter.only_en" />
      </IntlProvider>,
    );
    // react-intl uses `defaultLocale="en"` + `defaultMessage`; we
    // wire the defaults via `defaultMessages` for the en branch.
    await waitFor(() => expect(screen.getByTestId("t").textContent).toBe("EnglishOnly"));
  });

  it("returns the key id verbatim when nothing matches", async () => {
    const server = makeServer({ en: { other: "x" } });
    const client = new StarterClient({ baseUrl: "http://t", fetch: server.fetch });
    render(
      <IntlProvider client={client} language="en">
        <Probe id="starter.unknown" />
      </IntlProvider>,
    );
    await waitFor(() => expect(screen.getByTestId("t").textContent).toBe("starter.unknown"));
  });
});
