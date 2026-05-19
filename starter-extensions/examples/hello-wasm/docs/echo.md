Echo back the supplied `message` field. Identical to the builtin and
process flavour's `echo` tool — proves the kernel routes a WASM-flavour
extension's `dispatch-tool` call through the same code path adapters
already use for builtin / process.
