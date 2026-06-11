import { test, expect } from "@playwright/test";

// Loads the `chart-settings` dashboard — one panel per chart setting, each
// named after the setting — and classifies every panel as rendered / no-data /
// error, then asserts none failed. The per-panel report prints so a human (or
// the next agent) sees exactly which setting misbehaves.
//
// Prereqs: stack up (UI :4790, API :4780), the `chart-settings` dashboard
// seeded, and datapump publishing recent rows (panels query "now-ish" data).

const SLUG = "chart-settings";
// Wide window so "now-ish" datapump data is in range regardless of when run.
const URL = `/d/${SLUG}?from=now-6h&to=now`;

type Verdict = "rendered" | "no-data" | "error";

// Panels whose correct output is specific TEXT (not a chart/table): a stat
// showing a value-mapping result or a no-value placeholder. The spec asserts
// the text is present rather than looking for an svg/table.
const EXPECT_TEXT: Record<string, RegExp> = {
  "field: value mapping": /On/, // numeric 1 → "On ⚡"
  "field: no-value text": /n\/a/, // null value → configured noValue "n/a"
};

test("every chart-setting panel renders without error or empty state", async ({ page }) => {
  await page.goto(URL);

  // Each panel is a card with an <h3> title. Wait for the dashboard to mount.
  const titles = page.locator("h3");
  await expect(titles.first()).toBeVisible({ timeout: 15_000 });

  // Let the panel queries resolve (they fire on mount + time-range resolve).
  await page.waitForLoadState("networkidle");
  await page.waitForTimeout(1500);

  // Collect each panel card by walking up from its <h3> to the card container.
  const count = await titles.count();
  const results: { name: string; verdict: Verdict; detail?: string }[] = [];

  for (let i = 0; i < count; i++) {
    const h3 = titles.nth(i);
    const name = (await h3.textContent())?.trim() ?? `panel ${i}`;
    // DOM: card div.glass > header > h3. The card is the h3's grandparent;
    // scope all checks to that card so we read this panel's body only.
    const region = h3.locator("xpath=../..");

    let verdict: Verdict = "rendered";
    let detail: string | undefined;

    const expectText = EXPECT_TEXT[name];

    // Error state: role=alert with the destructive title.
    if (await region.getByRole("alert").count()) {
      verdict = "error";
      detail = (await region.getByRole("alert").innerText()).replace(/\s+/g, " ").slice(0, 120);
    } else if (expectText) {
      // Text-output panel: correct iff the expected text is present.
      const txt = (await region.innerText()).replace(/\s+/g, " ");
      if (expectText.test(txt)) {
        verdict = "rendered";
      } else {
        verdict = "no-data";
        detail = `expected ${expectText} — got "${txt.replace(name, "").trim().slice(0, 60)}"`;
      }
    } else if (
      // Empty state: "No data" / "No rows" text.
      (await region.getByText(/No data|No rows/i).count()) > 0
    ) {
      verdict = "no-data";
    } else {
      // Rendered: a chart (svg/canvas) or a stat value or a table.
      const hasChart = (await region.locator("svg, canvas").count()) > 0;
      const hasTable = (await region.locator("table").count()) > 0;
      if (!hasChart && !hasTable) {
        // Could be a stat number — accept any non-empty numeric/text body that
        // isn't the empty/error state. Fall back to "rendered" only if there's
        // visible body text beyond the title.
        const txt = (await region.innerText()).replace(name, "").trim();
        if (!txt) {
          verdict = "no-data";
          detail = "no body content";
        }
      }
    }
    results.push({ name, verdict, detail });
  }

  // Print a readable report (shows in the test output).
  const line = (r: (typeof results)[number]) =>
    `  ${r.verdict === "rendered" ? "✓" : "✗"} [${r.verdict}] ${r.name}${r.detail ? ` — ${r.detail}` : ""}`;
  console.log(`\nchart-settings panels (${results.length}):\n${results.map(line).join("\n")}\n`);

  const bad = results.filter((r) => r.verdict !== "rendered");
  // Soft-list the failures in the assertion message so they're visible on fail.
  expect(
    bad,
    `panels not rendering:\n${bad.map(line).join("\n")}`,
  ).toEqual([]);
});
