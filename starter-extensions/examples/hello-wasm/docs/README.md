# hello-wasm

WASM-flavour mirror of `hello-builtin` and `hello-process`. Same trait
impl, same `block.yaml` (modulo `runtime.kind`), same handler — one
cargo feature flip (`starter-ext-sdk = { features = ["wasm"] }`) is the
only delta. SCOPE.md R1 / "One source, three flavours".

Build with `cargo component build --release --target wasm32-wasip2`
(or wire it into the workspace's release pipeline) to produce
`target/wasm32-wasip2/release/hello_wasm.wasm`. Drop that artefact next
to the bundle's `block.yaml` and point `starter-ext-wasm`'s host at
the directory; the linker will refuse to instantiate it the moment any
imported WASI interface is not granted in the manifest's
`capabilities:` block.
