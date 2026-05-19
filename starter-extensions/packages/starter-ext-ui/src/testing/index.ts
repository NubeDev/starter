// Test harness for host-shell and extension-author tests.
//
// `renderWithExtensionHost(node, options)` mounts a tree with
// `ExtensionHostProvider` already wired and (optionally) one or
// more pre-registered remote factories. `msw` integration is the
// consumer's responsibility — the harness only stubs the host
// manager, not the HTTP layer.

export {
  renderWithExtensionHost,
  type RenderWithExtensionHostOptions,
} from "./render.js";
