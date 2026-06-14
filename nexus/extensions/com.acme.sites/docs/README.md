# Acme Sites

A **caller** demo for WS-18 (extension-to-extension API). Its `register_site`
tool exercises both peer channels:

- **Synchronous peer call** — resolves the site address by calling
  `com.acme.geocode.lookup` via `ctx.extension_call()`, gated by
  `requires_extensions[]` + the `extension` capability.
- **Async event-bus publish** — announces the registered site on
  `com.acme.sites.registered` (a topic it owns) via `ctx.event_bus().publish()`,
  for any same-tenant subscriber to react to.

Run order matters: `com.acme.geocode` must be installed + enabled first (the
host fails this extension's load otherwise, since the peer edge is declared).
