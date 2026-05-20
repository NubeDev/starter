// `<IntlProvider>` exercises:
// - fetches manifest + catalog on mount and renders the catalog
// - remounts react-intl when the active language changes (verified
//   by observing the rendered text switch)

import { afterEach, describe, expect, it } from "vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { useState } from "react";
import { StarterClient } from "@nube/starter-client-ts";

import {
  IntlProvider,
  _resetManifestCacheForTesting,
  useIntlContext,
} from "./provider.js";
import { _resetCatalogCacheForTesting } from "./fetcher.js";
import { useTranslate } from "./use-translate.js";

afterEach(() => {
  cleanup();
  _resetManifestCacheForTesting();
  _resetCatalogCacheForTesting();
});

function makeServer(catalogs: Record<string, Record<string, string>>) {
  const manifest: Record<string, string> = {};
  for (const lang of Object.keys(catalogs)) manifest[lang] = `${lang.padEnd(16, "0").slice(0, 16)}`;
  return (async (input: RequestInfo | URL) => {
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
}

function Greet() {
  const t = useTranslate();
  const ctx = useIntlContext();
  return (
    <>
      <span data-testid="greet">{t("starter.greet")}</span>
      <span data-testid="lang">{ctx.language}</span>
    </>
  );
}

describe("IntlProvider", () => {
  it("fetches the manifest and catalog on mount", async () => {
    const fetchImpl = makeServer({
      en: { "starter.greet": "Hello" },
      es: { "starter.greet": "Hola" },
    });
    const client = new StarterClient({ baseUrl: "http://t", fetch: fetchImpl });
    render(
      <IntlProvider client={client} language="en">
        <Greet />
      </IntlProvider>,
    );
    await waitFor(() => expect(screen.getByTestId("greet").textContent).toBe("Hello"));
    expect(screen.getByTestId("lang").textContent).toBe("en");
  });

  it("remounts the catalog when the active language switches", async () => {
    const fetchImpl = makeServer({
      en: { "starter.greet": "Hello" },
      es: { "starter.greet": "Hola" },
    });
    const client = new StarterClient({ baseUrl: "http://t", fetch: fetchImpl });

    function Harness() {
      const [lang, setLang] = useState("en");
      return (
        <>
          <button data-testid="switch" onClick={() => setLang("es")}>
            switch
          </button>
          <IntlProvider client={client} language={lang}>
            <Greet />
          </IntlProvider>
        </>
      );
    }
    render(<Harness />);

    await waitFor(() => expect(screen.getByTestId("greet").textContent).toBe("Hello"));
    screen.getByTestId("switch").click();
    await waitFor(() => expect(screen.getByTestId("greet").textContent).toBe("Hola"));
    expect(screen.getByTestId("lang").textContent).toBe("es");
  });
});
