# `com.nubeio.ce.engine_*`

REST proxy to a remote control engine (Niagara-kit / Sedona-style
flow runtime). The extension process resolves the engine's connection
row from `ce_devices`, then forwards the call over HTTP — credentials
stay server-side.

- `engine_status` — reachability + version.
- `engine_wiresheet_get` — read the engine's program as a node/edge graph.
- `engine_wiresheet_put` — write an edited graph back, reprogramming it.

The graph shape is the vocabulary the wiresheet's `BaseNode` renders
(`nodes[].kind`, slot-named `edges`).
