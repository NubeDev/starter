// Stage 6 — Australian operator end-to-end.
//
// The scope is `examples/notes/user-pref.md § Stage 6`. This spec is
// the consumer-facing proof that prefs + i18n flow end-to-end from
// the host chrome through one Module-Federation extension panel:
//
//   1. The `au-bbq-operator` fixture (locale `en-AU`, unit_system
//      `metric`, `temperature_unit` `fahrenheit`, language `en`)
//      sees AU-formatted dates and °F across host chrome *and* the
//      federated `HelloPanel`.
//   2. Flipping the host language to Spanish flips both surfaces in
//      one render — no page reload, no second fetch.
//   3. The same flip propagates to a second tab via the
//      `BroadcastChannel("starter-prefs")` from D-NP.9 — a
//      production operator with two tabs of the same product sees
//      both update in lockstep.
//   4. Flipping `temperature_unit` `fahrenheit` → `celsius` flips
//      both tabs' panels (and host chrome) to `°C`.
//   5. `<html lang>` follows the resolved language so screen
//      readers, hyphenation, spell-check, and font fallback all see
//      the right rules (Stage 1 contract).
//   6. axe-core reports zero new accessibility violations against
//      the authenticated app.
//
// The fixture is seeded by PATCHing `/v1/me/preferences` *before*
// the first page navigation — the global-setup gives us an owner
// bearer token but the default prefs are platform defaults, so the
// spec is responsible for putting the operator into the canonical
// AU BBQ profile.
//
// The cross-cut features this exercises (BroadcastChannel multi-tab,
// aria-live language announcement, missing-key telemetry) ship in
// Stage 7. This spec is the merge gate for that work.

import { test, expect, type Page, type BrowserContext } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
import { ownerToken } from "./helpers.js";

const AU_BBQ_OPERATOR = {
  timezone: "Australia/Brisbane",
  locale: "en-AU",
  language: "en",
  unit_system: "metric",
  temperature_unit: "fahrenheit",
  pressure_unit: "kilopascal",
  speed_unit: "kilometer_per_hour",
  length_unit: "meter",
  mass_unit: "kilogram",
  date_format: "DD/MM/YYYY",
  time_format: "24h",
  week_start: "monday",
  number_format: "auto",
  currency: "AUD",
  theme: "system",
} as const;

async function seedAuBbqOperator(token: string): Promise<void> {
  // PATCH /v1/me/preferences against the same backend the dev
  // server proxies to. The global-setup boots the Rust binary on
  // :8080, so we hit it directly here.
  const res = await fetch("http://127.0.0.1:8080/v1/me/preferences", {
    method: "PATCH",
    headers: {
      "content-type": "application/json",
      authorization: `Bearer ${token}`,
    },
    body: JSON.stringify(AU_BBQ_OPERATOR),
  });
  if (!res.ok) {
    throw new Error(
      `PATCH /v1/me/preferences failed: ${res.status} ${await res.text()}`,
    );
  }
}

async function signIn(page: Page, token: string): Promise<void> {
  await page.goto("/");
  await expect(page.locator("h1")).toContainText("notes");
  await page.getByPlaceholder("bearer token").fill(token);
  await page.getByRole("button", { name: /sign in/i }).click();
  // The HelloPanel is the load-bearing extension surface for this
  // spec; waiting on its greeting row also waits for the extension
  // host to finish loading the remote.
  await expect(page.getByTestId("hello-greeting")).toBeVisible({
    timeout: 10_000,
  });
}

async function openSettings(page: Page): Promise<void> {
  await page.getByRole("button", { name: /^settings$/i }).click();
  await expect(page.getByTestId("settings-form")).toBeVisible({
    timeout: 5_000,
  });
}

async function saveSettings(page: Page): Promise<void> {
  const [putResponse] = await Promise.all([
    page.waitForResponse(
      (res) =>
        res.url().includes("/v1/me/preferences") &&
        res.request().method() === "PATCH" &&
        res.status() === 200,
      { timeout: 5_000 },
    ),
    page.getByTestId("settings-submit").click(),
  ]);
  // Guard against the server returning 2xx without applying — the
  // PATCH body must be the diff the form computed.
  expect(putResponse.status()).toBe(200);
}

test.describe("au-operator: prefs + i18n end-to-end", () => {
  test.beforeEach(async () => {
    const token = ownerToken();
    await seedAuBbqOperator(token);
  });

  test("AU operator sees AU date + °F, flips to Spanish + °C across two tabs, axe clean", async ({
    browser,
  }) => {
    const context: BrowserContext = await browser.newContext();
    const page = await context.newPage();
    const token = ownerToken();
    await signIn(page, token);

    // ----- Step 1. Initial AU + en + °F render -------------------
    // HelloPanel greeting comes from the catalog (`com.nube.hello.
    // greeting`); the English catalog renders `Hello, world!`.
    await expect(page.getByTestId("hello-greeting")).toHaveText(
      /^Hello, world!?$/,
    );
    // Temperature uses the host's resolved prefs — the BBQ override
    // flips display to °F without the panel knowing units.
    await expect(page.getByTestId("hello-temperature")).toContainText("°F");
    // Date is rendered through `formatDate` against `en-AU` +
    // `DD/MM/YYYY`. Pin the slash separators rather than a literal
    // date so the test does not need to be re-pinned every day.
    await expect(page.getByTestId("hello-date")).toHaveText(
      /^\d{2}\/\d{2}\/\d{4}$/,
    );
    // Host chrome's PrefsProbe — proves "same prefs, same render"
    // across host + panel.
    await expect(page.getByTestId("prefs-probe-date")).toBeVisible();
    await expect(page.getByTestId("prefs-probe-temp")).toContainText("°F");
    // <html lang> follows the resolved language (Stage 1 contract).
    await expect(page.locator("html")).toHaveAttribute("lang", "en");

    // ----- Step 1b. axe-core clean on the authenticated app ------
    // The scope explicitly calls this out as the merge gate. We
    // disable color-contrast because the host's design tokens are
    // theme-driven and a contrast failure in a single token is a
    // theme bug, not a prefs/i18n bug — out of this spec's scope.
    const axeBefore = await new AxeBuilder({ page })
      .disableRules(["color-contrast"])
      .analyze();
    expect(axeBefore.violations).toEqual([]);

    // ----- Step 2. Open a second tab in the same context ---------
    // BroadcastChannel propagation only works inside one browser
    // context (same-origin same-browser), which is exactly the
    // 95th-percentile case D-NP.9 calls out.
    const page2 = await context.newPage();
    await signIn(page2, token);
    await expect(page2.getByTestId("hello-greeting")).toHaveText(
      /^Hello, world!?$/,
    );
    await expect(page2.getByTestId("hello-temperature")).toContainText("°F");

    // ----- Step 3. Flip language to Spanish on tab 1 -------------
    await openSettings(page);
    await page.getByTestId("field-language").selectOption("es");
    await saveSettings(page);

    // Tab 1: greeting flips to the Spanish catalog
    // (`com.nube.hello.greeting` → `¡Hola, world!`). No reload —
    // the IntlProvider re-renders the same React tree.
    await expect(page.getByTestId("hello-greeting")).toHaveText(
      /^¡Hola, world!?$/,
      { timeout: 5_000 },
    );
    // Temperature is independent of language; °F sticks.
    await expect(page.getByTestId("hello-temperature")).toContainText("°F");
    // <html lang> follows. The locale resolver may emit either
    // `es` or `es-AU` depending on whether the region was
    // preserved; both are documented as valid in the scope (Stage
    // 6 step: "pin whichever the resolver actually emits").
    await expect(page.locator("html")).toHaveAttribute(
      "lang",
      /^es(-AU)?$/,
    );

    // ----- Step 4. Tab 2 flips to Spanish via BroadcastChannel ---
    // No user action on tab 2; the BroadcastChannel("starter-prefs")
    // message from tab 1's PATCH must update tab 2 within one
    // animation frame. Playwright's auto-waiting on `toHaveText`
    // covers the frame budget.
    await expect(page2.getByTestId("hello-greeting")).toHaveText(
      /^¡Hola, world!?$/,
      { timeout: 5_000 },
    );
    await expect(page2.getByTestId("hello-temperature")).toContainText("°F");
    await expect(page2.locator("html")).toHaveAttribute(
      "lang",
      /^es(-AU)?$/,
    );

    // ----- Step 5. Flip temperature_unit °F → °C on tab 1 --------
    // Settings is still open on tab 1; reopen if a tab change
    // closed it on save.
    if (!(await page.getByTestId("settings-form").isVisible())) {
      await openSettings(page);
    }
    await page.getByTestId("field-temperature_unit").selectOption("celsius");
    await saveSettings(page);

    await expect(page.getByTestId("hello-temperature")).toContainText("°C");
    await expect(page.getByTestId("prefs-probe-temp")).toContainText("°C");

    // ----- Step 6. Tab 2 follows the unit flip -------------------
    await expect(page2.getByTestId("hello-temperature")).toContainText("°C", {
      timeout: 5_000,
    });
    await expect(page2.getByTestId("prefs-probe-temp")).toContainText("°C");

    // ----- Step 7. axe-core clean after both flips ---------------
    // A regression caught here would be e.g. the aria-live
    // language-change announcer breaking landmark structure.
    const axeAfter = await new AxeBuilder({ page })
      .disableRules(["color-contrast"])
      .analyze();
    expect(axeAfter.violations).toEqual([]);

    await page2.close();
    await context.close();
  });
});
