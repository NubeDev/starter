# Acme Geocode

A minimal **callee** demo for WS-18 (extension-to-extension API). It contributes
one tool, `com.acme.geocode.lookup`, and marks it peer-callable in
`contributes.provides[]`. Another extension that declares the dependency
(`requires_extensions[]`) and is granted the target (`capabilities.extension`)
can invoke it synchronously through the `extension.call` host method.

`lookup` is a pure transform (a deterministic pseudo-geocode of the address
string), so the extension needs no host capabilities of its own — when invoked
via `extension.call` it runs under the **caller's** identity, never widening it.
