# i18n + user preferences

How rubix consumes `starter-i18n` and `starter-prefs` to keep
domain code locale-agnostic and to translate at the right edge for
each transport.

> **Status:** design + starter primitives landed; rubix-side
> catalogue + tool wiring lands in Phase 1. The starter
> capabilities below all exist in-tree (see
> [docs/design/starter-changes/](../starter-changes/README.md) for
> the additive items that were upstreamed).

## Three independent axes

Per starter user SCOPE, **locale**, **language**, and **timezone**
are three separate things, often conflated.

| Axis | Type | Drives | Source per request |
|---|---|---|---|
| Language | `starter_spi::i18n::LanguageTag` | Catalogue selection (`"en"`, `"es"`) | `Accept-Language` header, MCP session locale, `sys.identity.preferences.language` |
| Locale | `String` (BCP-47) | Number / date / currency formatting | `sys.identity.preferences.locale` (defaults to language) |
| Timezone | `String` (IANA) | Timestamp rendering | `sys.identity.preferences.timezone` (defaults to UTC) |

Rubix never assumes "language implies timezone" or "locale implies
units". Each axis is read independently. The bridge between
`ResolvedPreferences.language: String` and `LanguageTag` is
`ResolvedPreferences::language_tag()` (one call, infallible).

## Where translation happens — by transport

Four wire surfaces, four rules.

### REST + gRPC — translate at the edge

Every error or human-facing reply on REST/gRPC carries a
`starter_spi::i18n::Diagnostic` (key + typed params) on the wire:

```json
{
  "code": "rubix.system.disk.warn",
  "params": {
    "percent": { "i64": 89 },
    "free":    { "quantity": { "canonical": 12500000000.0,
                                "quantity": "length" } }
  }
}
```

The client picks the catalogue and renders. For clients without a
catalogue (curl, log aggregators), the response body also carries
a pre-resolved `message_en` field — the server already holds the
EN catalogue, so it costs nothing to fill in.

The HTTP `AcceptLanguageLayer` (from `starter-i18n`) populates a
`LocaleCtx` extension; the error converter consults it. **Domain
code never holds a localised string.**

### SSE events — `MessageKey` only, client renders

The typed events in
[`rubix-spi::events`](../../../crates/rubix-spi/src/events.rs)
carry `message_key` + `params` with **no server-side resolution**.
The SSE consumer (the same client that just authenticated) resolves
through its own bundle.

This is what keeps event streams cache-friendly: one stream serves
every locale. The server does not fork on `Accept-Language` for
SSE.

### MCP — the documented exception (server-side render)

MCP clients (Claude Desktop, another agent) do not fetch
translation catalogues. The rubix MCP transport therefore
**resolves keys before serialising the tool result**, using
[`MessageBundle::render_diagnostic`](../../../crates/starter-i18n/src/bundle.rs):

```text
domain produces:   Diagnostic {
                     code:   "rubix.system.disk.warn",
                     params: { percent: I64(89),
                               free:    Quantity { canonical: 12.5e9,
                                                   quantity: Length } }
                   }
                              │
                              ▼  transport reads caller locale + prefs
MCP wire payload:  "El disco está casi lleno (89%, 12.5 GB libres)"
```

`render_diagnostic` converts `Quantity` params from canonical SI
to the caller's preferred unit before substitution — a length goes
out as GB for a metric user, GiB for a binary-prefix user, etc.
The LLM in the MCP client reads already-localised, already-
converted text.

The locale is sourced in this order, first hit wins:

1. The authenticated principal's
   `sys.identity.preferences.language` (post-Phase-2a).
2. The MCP request's `params._meta.acceptLanguage` field. The
   transport (`starter-mcp`) parses the BCP-47 tag, binds it on a
   tokio task-local for the lifetime of one `tools/call`, and any
   code that needs the caller's locale reads
   `starter_mcp::current_locale()`. The HTTP transport binds the
   same task-local per request from the `Accept-Language` header;
   the stdio loop and the in-memory test pair both bind it per
   session from the `initialize` frame's
   `params._meta.acceptLanguage`. Rubix code never re-parses
   `Accept-Language` and never threads a `LanguageTag` through
   call sites by hand — that's the U1 contract from
   `docs/design/starter-changes/`.
3. The agent's fallback (`"en"`).

#### Worked example — `es-AR`

An MCP client invoking `com.rubix.scheduled-system-check` with
`params._meta.acceptLanguage: "es-AR"` resolves a [`LanguageTag`]
of `es-AR`. The transport binds it; the bundled flow's seed
adapter reads `starter_mcp::current_locale()` and snapshots the
matching `ResolvedPreferences`:

```text
language        = "es"
locale          = "es-AR"
timezone        = "America/Argentina/Buenos_Aires"   // UTC-3 year-round
date_format     = DmySlash                            // DD/MM/YYYY
time_format     = H24
```

A `rubix.system.disk.warn` diagnostic with
`at = 2024-01-15T12:00:00Z` renders as:

> `El disco está casi lleno (89% usado, 12500000000 libre, sondeado el 15/01/2024, 09:00).`

— Spanish catalogue, EU date pattern, Buenos Aires wall-clock.

### CLI — `LANG` / `LC_*` env

The CLI reads `std::env::var("LANG")`, picks the matching
catalogue, and calls the same `MessageBundle::render_diagnostic`
the MCP transport uses. No new mechanism — same renderer, same
output shape.

### MCP over stdio — third path, three-step cascade

The `rubix-admin mcp` subcommand drives the same `FlowAsTool`
registry as the HTTP MCP router, but the locale source list is
different: there is no `Accept-Language` header on stdin. The
binary reads three sources at startup / per call, first hit wins:

1. **Per-call `params._meta.acceptLanguage`** — the U1 contract.
   `starter-mcp`'s stdio loop captures this from the `initialize`
   frame and re-binds it on `tools/call`.
2. **Process-startup `LANG`** — POSIX-style `es_AR.UTF-8`
   parsed into a BCP-47 tag (`es-AR`). The serve verb wraps the
   whole stdio loop in `starter_mcp::with_locale(...)` so this
   fallback is live whenever the host did not negotiate a locale
   on `initialize`. `C` / `POSIX` / empty / unparseable values
   fall through.
3. **`"en"`** — final fallback. Matches the HTTP cascade.

Same renderer (`MessageBundle::render_diagnostic`), same output
shape; the only difference from the HTTP path is the source of
the BCP-47 tag.

## Date / time / timezone

The same client-renders-by-default / MCP+CLI-render-server-side
split applies to timestamps as to MessageKeys — but the canonical
wire form is **`i64` epoch milliseconds, UTC** (per starter user
SCOPE R1), not a localised string.

### Three independent inputs

A rendered timestamp combines three resolved preferences:

| Preference | Drives | Examples |
|---|---|---|
| `timezone` (IANA) | The wall-clock shift | `Europe/Paris`, `America/New_York`, `UTC` |
| `date_format` | Date pattern | `YYYY-MM-DD` (ISO), `DD/MM/YYYY` (EU/UK), `MM/DD/YYYY` (US) |
| `time_format` | Clock pattern | `H24` (`13:42`), `H12` (`1:42 PM`) |

Same canonical wire value, four different outputs:

| Locale | Timezone | Date | Time | Output |
|---|---|---|---|---|
| en-US | `America/New_York` | `MM/DD/YYYY` | `12h` | `01/15/2024, 7:00 AM` |
| en-GB | `Europe/London` | `DD/MM/YYYY` | `24h` | `15/01/2024, 12:00` |
| fr-FR | `Europe/Paris` | `DD/MM/YYYY` | `24h` | `15/01/2024, 13:00` |
| (no prefs) | `UTC` | `YYYY-MM-DD` | `24h` | `2024-01-15, 12:00` |

### Where conversion happens

This is the industry-normal split for Rust backend + React (or
Flutter / Swift / Kotlin) frontend:

| Surface | Convert where | Why |
|---|---|---|
| REST + gRPC | **Client** (raw `i64` on the wire) | Cache-friendly; client picks `date-fns-tz` / `Intl.DateTimeFormat` / `java.time`. Every modern client platform has battle-tested timezone libs. |
| SSE events | **Client** (raw `i64` in events) | Same — one stream serves every locale. |
| MCP | **Server** | MCP clients render strings directly; the LLM would either pass raw ms through or hallucinate a format. Documented exception, same as i18n strings. |
| CLI | **Server** (at call site) | A Rust CLI printing `1764892800000` is broken. `chrono`-format at the call site. |

### Server-side rendering uses `render_diagnostic`

The MCP transport and the CLI both call
`MessageBundle::render_diagnostic` (from `starter-i18n`). When a
`DiagnosticParam::Timestamp(ms)` is interpolated, the renderer:

1. Converts `ms` → `DateTime<Utc>` via
   `DateTime::from_timestamp_millis`.
2. Shifts into `prefs.timezone` (IANA) via `chrono_tz`.
3. Formats date pattern from `prefs.date_format` (ISO / DMY / MDY).
4. Formats time pattern from `prefs.time_format` (24h / 12h).
5. Writes `"<date>, <time>"`.

Conversion failures (unknown timezone, out-of-range ms) fall
through to canonical UTC RFC 3339 so the operator always sees a
parseable timestamp.

### Relative time ("5 minutes ago")

**Client renders.** Every target platform has native support:

- React / browser: `Intl.RelativeTimeFormat`, `date-fns/formatDistanceToNow`.
- Flutter: `timeago` package.
- iOS / Swift: `RelativeDateTimeFormatter`.
- Android / Kotlin: `DateUtils.getRelativeTimeSpanString`.

Tools emit absolute `Timestamp` values; clients compute relative
spans if they want. The server doesn't ship a parallel
`rubix.time.relative.minutes_ago` key namespace.

### Anti-patterns

- A REST handler that returns `"2024-01-15T13:00:00+01:00"`
  as a string. Wire is `i64` ms; let the client format.
- A domain function taking a `&Tz` parameter. Domain code is
  timezone-agnostic. The renderer holds the timezone.
- A tool that pre-formats a timestamp before passing it through a
  `Diagnostic`. Use `DiagnosticParam::Timestamp(ms)`; the
  renderer converts at the edge.

## What tools return

A rubix tool's output is **structured + keyed**, never a
pre-formatted string. The canonical shape:

```rust
pub struct ToolOutput {
    /// MessageKey naming the high-level outcome shape.
    pub summary: Diagnostic,
    /// Structured data for the LLM or another tool to read.
    pub data: serde_json::Value,
}
```

`summary` is a `Diagnostic` from `starter-spi`. `Quantity`-typed
params pass through `starter-prefs` for unit conversion at the
transport edge. A tool that returns a raw `f64` or `String` for a
human-facing value is a bug.

## Catalogue home

EN + ES catalogues live in
[`crates/rubix-spi/catalogues/`](../../../crates/rubix-spi/catalogues/)
as flat JSON keyed by `MessageKey`, matching `starter-i18n`'s
format. Two files at launch: `en.json` (canonical) and `es.json`
(initial Spanish coverage).

The [`rubix-spi::i18n`](../../../crates/rubix-spi/src/i18n/) module
exposes `bundled_catalogue(lang) -> Catalog` and a
`rubix_bundle() -> MessageBundle` helper that returns both
languages. `rubix-agent` calls this at boot, merges with starter's
own catalogues, and installs the combined bundle on the
`AcceptLanguageLayer`.

### Key namespace

`rubix.<goal>.<verb>.<concern>` — flat, no nesting:

```
rubix.system.disk.ok
rubix.system.disk.warn
rubix.system.disk.full
rubix.skill.denied
rubix.flow.canceled
rubix.user.disabled.confirmation
```

The catalogue files are the source of truth. Adding a key without
an entry in **both** `en.json` and `es.json` fails review.
Untranslated `es.json` entries are permitted with the EN string
verbatim — flagged for translation but not blocking.

## Skills and tool descriptors stay EN

Skills and descriptors are **LLM-facing prompts**, not human text.
They stay EN canonical:

- Every LLM provider's instruction-following is strongest in EN.
- Translating a skill body risks subtle model regression
  (literal-translation idiom shifts, lost emphasis on negation).
- The skill *output* — what the agent produces with the skill
  active — gets localised via the tool-result path above. That is
  the human-visible surface.

This includes the `description:` frontmatter field used by
`SkillSelector` (the selector is itself an LLM call) and the five
`ToolDescriptor` fields. Both remain `&'static str` in English; no
`MessageKey` here.

## Quantity rendering

`MessageBundle::render_diagnostic` consults `ResolvedPreferences`
to pick the target unit:

- Australian operator (metric, default temp °C) → `25 °C`.
- American operator (imperial, temp °F) → `77 °F`.
- Australian operator with per-quantity override (BBQ temp °F) →
  `77 °F` for the BBQ slot, `25 °C` for everything else.

Domain code stores **canonical SI** (per starter user SCOPE R1).
The transport calls the renderer, which calls
`convert_for_display` from `starter-spi::units`.

## Anti-patterns

- A tool that builds an output string with `format!` for a human
  reader. The output is `Diagnostic` with `code` + `params`.
- A domain function that takes `&LanguageTag` as a parameter.
  Domain code is locale-agnostic. The transport layer holds the
  locale.
- A `MessageKey` defined in a Rust constant *without* a matching
  entry in `en.json` + `es.json`. The catalogue files are the
  source of truth.
- A skill or descriptor field containing non-EN text.
- A REST error body that has only `message_en` and no `code`.
  Clients that ship their own catalogue can't re-render it.

## Localisation section — SKILL.md template

Every bundled rubix `SKILL.md` carries this section verbatim,
adjusted only for the goal-specific MessageKey prefix. The
purpose is to steer the LLM away from hallucinating raw floats
or pre-rendered strings.

```markdown
## Localisation

When you call a rubix tool, the reply is a structured
`Diagnostic` (a stable `code` plus typed `params`) plus
`data`. The transport layer renders the diagnostic into the
caller's language and units before the human sees it.

You MUST:

- Emit MessageKey codes that already exist in the rubix
  catalogue (prefix: `rubix.<goal>.*`). Do not invent new
  keys at runtime — request a catalogue entry instead.
- Pass numeric measurements through the tool's typed
  `Quantity` slot. Never format units yourself ("12.5 GB"
  is the renderer's job, not yours).
- Leave dates / times as RFC3339 UTC; the renderer applies
  the caller's timezone.

You MUST NOT:

- Concatenate localised strings yourself. Compose
  `Diagnostic` instances instead.
- Pick a language for the user. The transport does that.
- Translate skill text or descriptor text. Both stay EN.
```

A skill that omits this section fails review.
