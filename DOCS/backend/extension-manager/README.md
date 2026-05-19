# Extension Manager — Backend Lifecycle & Frontend Module Federation

How the `starter-extensions` substrate manages extensions at runtime:
process supervision on the Rust side and Module-Federation bundle
serving + singleton negotiation on the frontend side.

---

## 1. Rust-side: Process Supervision

### Overview

Every extension declares one of three `runtime.kind` values in its
`block.yaml` manifest:

| Kind      | Where it runs                     | Lifecycle owner              |
|-----------|-----------------------------------|------------------------------|
| `builtin` | In the host's address space       | Host process (no PID)        |
| `process` | Separate child process (any lang) | `starter-ext-supervisor`     |
| `wasm`    | WASI-p2 sandbox inside the host   | `starter-ext-wasm` (future)  |

The **process** flavour is the one with PID management.

### Load sequence

```
1. Loader::scan(extensions_dir)
      ↓  walks directories, parses each block.yaml
2. Loader::validate_all()
      ↓  namespace checks (R4), capability compat (R6), schema validation
3. Loader::commit(candidates, &mut ExtensionRegistry)
      ↓  all-or-nothing: registry either accepts all valid or none
4. registry.seal()
      ↓  immutable from here — no runtime additions
```

After the registry is sealed, process-flavour extensions are handed to
the **Supervisor**.

### Supervisor (starter-ext-supervisor)

One `Supervisor` instance per process-flavour extension. Each spawns a
child via `tokio::process::Command` and manages its full lifecycle:

```
                      ┌─────────────────────────────────┐
                      │         Supervisor Task         │
                      │                                 │
  spawn ─────────────►│  Child process (extension bin)  │
                      │       stdio JSON-RPC            │
                      │                                 │
                      │  ┌──────────────────────────┐   │
                      │  │ Init handshake           │   │
                      │  │  • host sends config     │   │
                      │  │  • child sends "ready"   │   │
                      │  │  • manifest hash check   │   │
                      │  └──────────────────────────┘   │
                      │                                 │
                      │  Health pinger (periodic)       │
                      │  Stderr forwarder → tracing     │
                      │  Capability gate (wire-level)   │
                      │  Event ring (diagnostics)       │
                      └─────────────────────────────────┘
```

**Key behaviours:**

- **PID tracking**: `tokio::process::Child` owns the OS process handle.
  The supervisor observes exit via `.wait()` and decides whether to
  restart based on the manifest's `supervision.restart` policy.

- **Restart policy** (`supervision.restart` in `block.yaml`):
  - `always` — restart on any exit (clean or crash)
  - `on_crash` — restart only on non-zero exit (default)
  - `never` — do not restart; mark as Stopped

- **Intensity cap**: `max_restarts` within `within_seconds`. Exceeded →
  extension transitions to `Failed` state permanently (until operator
  re-enables via admin API).

- **Exponential backoff with jitter**: between restarts. Configurable
  via `supervision.backoff.{initial_ms, max_ms, jitter}`.

- **Health checks**: periodic `health` JSON-RPC notification. Missed
  pings (past `health.timeout_ms`) count as a crash and trigger the
  restart policy.

- **Graceful shutdown**: `SIGTERM` → wait `shutdown_grace_ms` → `SIGKILL`.

### IPC: stdio JSON-RPC (R10)

All communication between host and child process uses **stdin/stdout**
with Content-Length-framed JSON-RPC 2.0 messages (same framing as
LSP/MCP). No sockets, no gRPC between host and child.

```
Host → Child (stdin):   { "jsonrpc": "2.0", "method": "tool.call", "id": 1, "params": {...} }
Child → Host (stdout):  { "jsonrpc": "2.0", "result": {...}, "id": 1 }
Child stderr:           forwarded to host tracing with extension id as span tag
```

**Capability enforcement at the wire boundary**: if a child sends a
request for a host method it did not declare in `requires:`, the host
returns a JSON-RPC error and increments a `capability_violation`
counter (visible on `GET /extensions/<id>`).

### Lifecycle states

```
Discovered → Validated → Starting → Running → Stopping → Stopped
                                        ↓
                                     Crashed → (restart or Failed)
```

All transitions are recorded in a per-extension **EventRing** (bounded
ring buffer, default 1000 entries). Surfaced at
`GET /extensions/<id>/events`.

### SupervisorHandle (public API)

```rust
pub struct SupervisorHandle {
    id: ExtensionId,
    state: watch::Receiver<LifecycleState>,   // observe transitions
    shutdown_tx: mpsc::Sender<()>,            // request shutdown
    events: Arc<EventRing>,                   // diagnostic ring
    violations: Arc<CapabilityViolationCounter>,
    inbound: mpsc::UnboundedSender<Value>,    // send JSON-RPC to child
}
```

The admin endpoints (`POST /extensions/<id>/enable|disable`) use this
handle to stop/restart individual extensions at runtime.

---

## 2. Frontend: Module Federation

### How the server serves extension UI bundles

Each extension with a `contributes.ui` block ships a
`remoteEntry.js` (+ chunks) in its bundle directory. The server
(`starter-ext-server`) serves these at:

```
GET /extensions/<id>/ui/<path>
```

- Path traversal protection (component-level + `canonicalize` check)
- Strong ETags (SHA-256 of file bytes, memoised by path + mtime + size)
- `If-None-Match` → `304 Not Modified` (no re-download)
- MIME type guessing from file extension

This endpoint is **unauthenticated** — the JS bundles are public
assets. The data they fetch once mounted goes through authenticated
`StarterClient` calls.

### Host-side federation runtime (@nube/starter-ext-ui)

```
┌─────────────────────────────────────────────────────────┐
│  Host shell (notes app, any consumer)                   │
│                                                         │
│  <ExtensionHostProvider host={manager}>                 │
│    ├── <ExtensionSlot id="sidebar"/>                    │
│    │     └── WeatherPanel (from ext A)                  │
│    │     └── NotesPanel  (from ext B)                   │
│    └── <ExtensionSlot id="header"/>                     │
│          └── AlertBanner (from ext C)                   │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

**Bootstrap sequence:**

1. Host constructs an `ExtensionHostManager` with singleton provisions:
   ```ts
   const host = new ExtensionHostManager({
     client,
     singletons: {
       react:       { version: "18.3.1", instance: React },
       "react-dom": { version: "18.3.1", instance: ReactDOM },
     },
   });
   ```

2. Host fetches `GET /extensions` to discover loaded extensions.

3. For each extension with `contributes.ui`:
   ```ts
   const mod = await import(`/extensions/${id}/ui/remoteEntry.js`);
   await host.registerExtensionRemote(id, manifestUi, mod.default);
   ```

4. Inside `registerExtensionRemote`:
   - **Singleton negotiation**: compare declared majors vs host's
     provisioned versions. Mismatch → `SingletonMismatchError` (extension
     fails to load; others continue).
   - Call `factory.init(handle)` where handle exposes the host's React
     instance and a `register(contributions)` callback.
   - Extension calls `registerExtensionContributions(handle, { components: { Panel } })`.

5. `<ExtensionSlot id="sidebar"/>` resolves all contributions whose
   manifest declares `slot: "sidebar"` and renders them wrapped in
   `SlotContextProvider`.

### Extension-author side (@nube/starter-ext-sdk-ts)

What the extension's `remoteEntry.ts` does:

```ts
import { registerExtensionContributions } from "@nube/starter-ext-sdk-ts";
import Panel from "./Panel.js";

export default {
  singletons: { react: { version: "18.3.1" } },
  init(handle) {
    registerExtensionContributions(handle, {
      components: { Panel },
    });
  },
};
```

The extension **never issues raw `fetch`**. It uses `useHostClient()`
which routes through `StarterClient` (auth + tracing + retry uniform
across all extensions).

### Singleton negotiation (why it matters)

Two React instances on the same page = broken hooks, broken context.
The federation runtime prevents this by:

1. Host declares which packages it provides (React, react-dom, etc.)
2. Extension declares which it consumes + the version it built against
3. Host checks `matchingMajor(host.version, ext.version)` — match →
   bind extension to host's instance; mismatch → refuse to load

This means extensions share the host's React — no duplicate bundles,
no hook tearing.

---

## 3. How They Connect

```
                    ┌─────────────────────────────┐
                    │   Consumer binary (notes)   │
                    │                             │
 Rust backend       │  ExtensionRegistry (sealed) │
                    │       ↓                     │
                    │  Supervisor (process PIDs)  │──── stdio JSON-RPC ──── child processes
                    │       ↓                     │
                    │  Admin routes:              │
                    │    GET /extensions          │
                    │    GET /extensions/:id      │
                    │    POST .../enable|disable  │
                    │    GET .../ui/*             │──── serves remoteEntry.js + chunks
                    │    GET .../events           │
                    └─────────────────────────────┘
                                 │
                                 │ HTTP
                                 ▼
                    ┌─────────────────────────────┐
                    │   Frontend (Vite / browser)  │
                    │                             │
                    │  ExtensionHostManager       │
                    │    • fetches /extensions    │
                    │    • import(remoteEntry.js) │
                    │    • singleton negotiation  │
                    │    • registerRemote()       │
                    │                             │
                    │  <ExtensionSlot id="…"/>    │
                    │    • resolves contributions │
                    │    • mounts React panels    │
                    └─────────────────────────────┘
```

### Data flow for a tool call (process-flavour, end-to-end)

1. User clicks button in extension UI panel
2. Panel calls `useHostClient().post("/tools/com.acme.weather.current", { city: "NYC" })`
3. `StarterClient` adds bearer token, sends to Vite proxy → backend
4. REST adapter looks up `com.acme.weather.current` in registry
5. Adapter sees `runtime.kind: process`, sends JSON-RPC `tool.call` over child's stdin
6. Child extension processes, writes JSON-RPC result to stdout
7. Supervisor reads result, returns to adapter → HTTP 200 JSON → browser

---

## 4. Admin API Summary

| Endpoint                        | Auth       | Purpose                                      |
|---------------------------------|------------|----------------------------------------------|
| `GET /extensions`               | Admin      | List all extensions with state + version      |
| `GET /extensions/:id`           | Admin      | Full detail: manifest, capabilities, violations |
| `GET /extensions/:id/events`    | Admin      | Event ring (paginated JSON or SSE live tail)  |
| `POST /extensions/:id/enable`   | Admin      | Re-spawn a stopped/disabled extension         |
| `POST /extensions/:id/disable`  | Admin      | Graceful shutdown of a running extension      |
| `GET /extensions/:id/ui/*path`  | Public     | Serve the extension's MF bundle files         |

---

## 5. Configuration (block.yaml excerpt)

```yaml
runtime:
  kind: process
  bin: dist/my-extension       # path to spawned binary

supervision:
  restart: on_crash            # always | on_crash | never
  max_restarts: 5
  within_seconds: 60
  backoff:
    initial_ms: 200
    max_ms: 30000
    jitter: true
  health:
    interval_ms: 5000
    timeout_ms: 2000
  shutdown_grace_ms: 5000

contributes:
  ui:
    entry: ui/remoteEntry.js
    exposes:
      - { name: MyPanel, module: "./Panel", slot: sidebar }
```

---

## 6. Key Design Decisions

- **One PID per extension** — no supervisor groups in v0.1 (SCOPE R9).
  Each extension restarts independently; coupled restarts add complexity
  without matching the isolation model.

- **stdio JSON-RPC only** — no Unix sockets, no gRPC between host and
  child (SCOPE R10). Debuggable, language-agnostic, no socket lifecycle.

- **Default-deny capabilities** — process-flavour enforcement is
  advisory at the wire boundary. WASM-flavour is hard-enforced via WASI
  grants.

- **UI bundles are public** — `GET /extensions/:id/ui/*` is unauthenticated.
  The data extensions access is gated through `useHostClient()` which
  carries the user's bearer.

- **Singleton negotiation is strict** — major mismatch refuses the
  extension entirely rather than loading with a duplicate React. One
  broken extension never poisons the host.
