# `com.nubeio.ce` — Control Engine device manager

CRUD over remote **control engines** — flow-based runtimes such as a
Niagara-kit controller or a Sedona device — plus a **wiresheet** for
remotely programming them.

The operator:

1. Registers an engine by its connection details (IP, port, username,
   password) — a row in the `ce_devices` warehouse table.
2. Opens the **wiresheet** (a React-Flow canvas built on
   `@nube/starter-ui-flow`'s `BaseNode`) for a device.
3. Reads / writes the engine's program over its REST API, proxied by
   this extension's process.

## Shape

| Half            | Where                                            |
| --------------- | ------------------------------------------------ |
| Device catalog  | `ce_devices` warehouse table + `device_*` tools  |
| Engine REST     | `engine_*` tools — process forwards over HTTP    |
| Wiresheet UI    | `ui-src/wiresheet/` (BaseNode-based canvas)       |
| Device admin UI | `ui-src/devices/`                                 |

## Status

**Scaffold only.** Tool handlers and UI panels are laid out with the
correct call surface but no business logic — the REST client to the
control engine, the wiresheet graph (de)serialisation, and the device
form submits are all `TODO`. See `process/src/engine/` and
`process/src/device/`.

## Build

```sh
make all        # cargo build + vite build + install + reload
make build      # cargo build only
make ui-build   # vite build → ui/remoteEntry.js
```

The only Rust dependency is `starter-ext-sdk` (SCOPE R8). The UI is a
module-federation remote; React is the only externalised singleton, so
the wiresheet pulls `@nube/starter-ui-flow` + `@xyflow/react` into the
bundle.
