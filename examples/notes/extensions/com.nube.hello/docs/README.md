# com.nube.hello

Minimal demo extension bundled with `examples/notes`. Proves the
end-to-end extension round trip on top of the `starter-extensions`
substrate:

- **Tool** `com.nube.hello.greet` — registered into the host's
  `ToolRegistry` by `starter-ext-mcp::register_tools` and reachable
  over the same `/mcp` endpoint as the consumer's own
  `NoteSearchTool`.
- **REST** `GET /hello` — mounted by `starter-ext-server::rest_router`
  via `BuiltinRestDispatcher`.
- **UI** `HelloPanel` — a hand-written `remoteEntry.js` loaded by the
  notes frontend through `@nube/starter-ext-ui`'s Module-Federation
  runtime and mounted into the `sidebar` slot.

The extension is intentionally tiny — no schemas with required fields,
no auth gate, no persistence — so the wiring is the only thing worth
reading.
