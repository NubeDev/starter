# hello-builtin

Smallest possible builtin-flavour extension. Exposes one tool
(`com.acme.hello.echo`) that echoes its input.

Phase 1 of `starter-extensions` uses this crate as the end-to-end smoke
test: `starter-ext-host` discovers the bundle, `starter-ext-mcp`
registers the tool into `starter-mcp`'s `ToolRegistry`, and a call to
`com.acme.hello.echo` round-trips through the kernel into this crate's
`handle_com_acme_hello_echo` handler.
