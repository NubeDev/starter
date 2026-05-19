Echo back the supplied `message` field. Useful as a liveness probe for the
extension substrate: a successful round-trip proves the host's MCP adapter
dispatched a call through the kernel, the builtin dispatch table located the
extension, and the handler executed inside the host process.
