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
2. An MCP-session `Accept-Language` initial handshake header.
3. The agent's fallback (`"en"`).

### CLI — `LANG` / `LC_*` env

The CLI reads `std::env::var("LANG")`, picks the matching
catalogue, and calls the same `MessageBundle::render_diagnostic`
the MCP transport uses. No new mechanism — same renderer, same
output shape.

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
