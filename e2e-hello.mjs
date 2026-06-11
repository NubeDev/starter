import { chromium } from "playwright";

const UI = "http://127.0.0.1:4790";
const EMAIL = "admin@nexus.local";
const PASS = "change-me-admin";
const CHROME = "/home/user/.cache/ms-playwright/chromium-1223/chrome-linux64/chrome";

const browser = await chromium.launch({ executablePath: CHROME });
const ctx = await browser.newContext({ viewport: { width: 1400, height: 900 } });
const page = await ctx.newPage();

const errors = [];
page.on("console", (m) => { if (m.type() === "error") errors.push(m.text()); });
page.on("pageerror", (e) => errors.push("PAGEERROR: " + e.message));

// --- login ---
await page.goto(UI, { waitUntil: "domcontentloaded" });
await page.fill("#email", EMAIL);
await page.fill("#password", PASS);
const [loginResp] = await Promise.all([
  page.waitForResponse((r) => r.url().includes("/auth/login")),
  page.click('button[type="submit"]'),
]);
console.log("login response status:", loginResp.status());
// wait until we leave the login screen
await page.waitForFunction(() => !document.querySelector("#email"), null, { timeout: 8000 })
  .catch(() => console.log("WARN: still showing login form after submit"));
await page.waitForLoadState("networkidle");
console.log("after login URL:", page.url());

// --- go to the extension page ---
await page.goto(`${UI}/x/com.nexus.hello`, { waitUntil: "networkidle" });
await page.waitForTimeout(2000);
console.log("ext page URL:", page.url());

const extCount = await page.locator('[data-ext-id="com.nexus.hello"]').count();
console.log("data-ext-id roots:", extCount);

const card = page.locator('[data-ext-id="com.nexus.hello"] [data-slot="card"]');
const cardCount = await card.count();
console.log("shadcn Card found:", cardCount);

if (cardCount > 0) {
  const styles = await card.first().evaluate((el) => {
    const cs = getComputedStyle(el);
    return {
      backgroundColor: cs.backgroundColor,
      borderWidth: cs.borderTopWidth,
      borderColor: cs.borderTopColor,
      borderRadius: cs.borderTopLeftRadius,
      boxShadow: cs.boxShadow.slice(0, 30),
      padding: cs.paddingTop,
    };
  });
  console.log("Card styles:", JSON.stringify(styles));
  const styled =
    styles.backgroundColor !== "rgba(0, 0, 0, 0)" &&
    parseFloat(styles.borderRadius) > 0 &&
    parseFloat(styles.borderWidth) > 0;
  console.log(styled ? "✅ CARD STYLED" : "❌ CARD UNSTYLED");
}

const btn = page.locator('[data-ext-id="com.nexus.hello"] [data-slot="button"]');
const btnCount = await btn.count();
console.log("shadcn Button found:", btnCount);
if (btnCount > 0) {
  const bs = await btn.first().evaluate((el) => {
    const cs = getComputedStyle(el);
    return { backgroundColor: cs.backgroundColor, color: cs.color, height: cs.height, borderRadius: cs.borderTopLeftRadius };
  });
  console.log("Button styles:", JSON.stringify(bs));
}

const bodyText = await page.locator("body").innerText();
console.log("--- visible text ---");
console.log(bodyText.slice(0, 400).replace(/\n+/g, " | "));

console.log("--- console errors ---");
console.log(errors.length ? errors.slice(0, 10).join("\n") : "(none)");

await page.screenshot({ path: "/tmp/hello-ext.png" });
console.log("screenshot: /tmp/hello-ext.png");
await browser.close();
