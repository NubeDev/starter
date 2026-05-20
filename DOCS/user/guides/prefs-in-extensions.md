# Preferences in extensions

> **Audience:** operators and team leads using a starter-based product
> that ships third-party extensions. If you are an extension author,
> read [DOCS/extensions/guides/i18n.md](../../extensions/guides/i18n.md)
> instead.

## "I set my language once. Why did it change the widget panel?"

Every starter-based product gives you one place to change your
language, date format, units, and timezone — the **Settings** page.
The setting applies to the whole product:

- The chrome you see (menus, buttons, headers).
- Every extension panel, sidebar tile, or admin widget — even ones
  written by a different team.

This is by design. Operators expect one switch, not "switch the chrome
to Spanish, then switch each panel separately." A starter-based
product enforces that by making every extension read its strings,
date formats, and units from the same place the host reads them.

## What "preferences" covers

The Settings page lets you set:

| Field | Examples |
|---|---|
| **Language** | English, Spanish, German, … (varies by deployment) |
| **Locale** | `en-AU`, `en-US`, `es-MX`, … — drives date and number formatting |
| **Timezone** | `Australia/Brisbane`, `Europe/Berlin`, … |
| **Unit system** | Metric or imperial — drives length, mass, speed defaults |
| **Temperature** | Celsius or Fahrenheit (overrides the unit system) |
| **Date format** | `DD/MM/YYYY`, `MM/DD/YYYY`, `YYYY-MM-DD` |
| **Number format** | `1,234.56`, `1.234,56`, `1 234,56` |
| **Week start** | Monday or Sunday |
| **Currency** | `AUD`, `USD`, `EUR`, … |
| **Theme** | Light, dark, follows system |

All of these flow to every extension. A weather extension that shows
"22 °C" today shows "72 °F" the moment you flip your temperature
preference — without a page reload.

## How it propagates

1. You open Settings, change a value, click Save.
2. The product PATCHes your preferences to the server.
3. The host's `<PreferencesProvider>` refetches and re-renders. Every
   extension panel that was reading the affected field re-renders in
   the same animation frame.
4. If you have a **second tab** open of the same product in the same
   browser, that tab updates within one frame too — a
   `BroadcastChannel("starter-prefs")` carries the patch between tabs.
   No reload needed.
5. The chrome's `<html lang>` attribute updates so your screen reader
   uses the right voice / pronunciation rules. A polite
   `aria-live` announcement says "Language changed to <new language>"
   in the new language.

The fan-out is **same-browser only.** A second device or a different
browser will pick up the change on its next refresh — cross-device
push is out of scope for v1.

## Why it sometimes takes a moment

- A language flip fetches the new catalog over the network. On a slow
  connection you may see English flash before the new language paints
  for a fraction of a second.
- The first time you set a language outside `en` after a deploy, the
  catalog is uncached; subsequent flips are instant.

If the catalog the product needs is not on the server, the host falls
back to a "best match" — `es-MX` → `es`, then `es` → `en` if even
that is missing. The fallback is announced through internal telemetry
so the platform team can ship the missing catalog; from your
perspective, the product simply shows whatever is closest.

## What extensions *cannot* do

- Override your preferences. Extensions read; they cannot write a
  per-extension preference into your account.
- Re-fetch your prefs themselves. Every extension reads the same
  resolved value the chrome reads.
- Skip the format you set. An extension that shows a date must show
  it in your `DD/MM/YYYY` (or whatever you chose) — not its own
  default.

## What to expect after the flip

- Times, dates, and numbers in every panel match your chrome.
- Quantities (temperature, speed, distance) display in your chosen
  units regardless of the panel's author.
- Strings translate when the catalog has them. Strings the catalog
  lacks fall through to English, with the missing entry logged on
  the server side so the team can fix it.
- Currency symbols and digit grouping match your locale + currency.

## What to do if it doesn't

1. Hard-refresh the tab once (the cache may be stale).
2. Confirm the language is set in Settings — the dropdown shows the
   current value.
3. File the bug. Include the extension that did not flip and the
   key you expected to translate; the platform team can use the
   `i18n.message_missing` log to fix the catalog gap.

## See also

- Extension author guide: [DOCS/extensions/guides/i18n.md](../../extensions/guides/i18n.md)
- Platform contract: [DOCS/user/scope/SCOPE.md](../scope/SCOPE.md)
- Reference implementation: [`examples/notes/user-pref.md`](../../../examples/notes/user-pref.md)
