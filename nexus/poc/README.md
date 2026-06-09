# Nexus · ArkFlow POC

A small, clean proof-of-concept that **embeds the [ArkFlow](https://github.com/arkflow-rs/arkflow)
stream-processing engine** in a Rust/Axum backend and drives it from a React config-builder UI.

It is a POC, not production — but the code follows the repo's
[FILE-LAYOUT](../../rubix/FILE-LAYOUT.md) discipline (one verb per file, thin handlers,
concept-named files).

```
poc/
├── backend/        Rust · Axum · embeds ArkFlow (arkflow-core + trimmed arkflow-plugin)
├── ui/             React 18 · Vite · TypeScript
└── vendor/arkflow/ vendored ArkFlow, trimmed to build without native toolchains
```

## What it does

The UI has one section per major ArkFlow concept:

| Section | Backend route | What it shows |
|---|---|---|
| **Stream Builder** | `POST /api/streams/{validate,run}` | compose input + buffer + processors + output, validate, then **run it on the real engine** and see the rows |
| **SQL Playground** | `POST /api/sql/query` | run DataFusion SQL over inline JSON (`memory → json_to_arrow → sql → collector`) |
| **Inputs / Outputs / Processors / Buffers** | `GET /api/{inputs,outputs,processors,buffers}` | reference catalog with each type's configurable fields |
| **Plugins** | `GET /api/plugins` | every registered component, marking the custom ones |

Runs are **real ArkFlow `Stream`s**. The backend swaps the output for a custom in-memory
**`collector`** sink (registered via `register_output_builder`), runs the stream under a
`CancellationToken` bounded by a timeout, and returns the captured rows.

## Run it

**1. Backend** (needs `protoc` on PATH or `PROTOC` set):

```sh
cd backend
PROTOC=$(which protoc) cargo run      # → http://127.0.0.1:8787
```

**2. UI:**

```sh
cd ui
npm install
npm run dev                            # → http://localhost:5274 (proxies /api → :8787)
```

## The vendored, trimmed ArkFlow

ArkFlow's `arkflow-plugin` bundles every connector (Kafka, Pulsar, NATS, Redis, MQTT,
Modbus, DuckDB, SQL DBs, Python, object stores) with **no feature flags**, so a stock build
needs librdkafka, libcurl, libduckdb, libpython and `protoc`. For a toolchain-free POC,
`vendor/arkflow` disables those modules in each `src/*/mod.rs` and drops their deps from
`Cargo.toml`. Every change is tagged `Nexus POC trim:`.

**Kept:** inputs `generate · http · memory`; outputs `stdout · http · drop` (+ our
`collector`); processors `json_to_arrow · arrow_to_json · sql · vrl · batch`; all buffers
(`memory · tumbling/sliding/session window`).

To use the full upstream engine instead, point `backend/Cargo.toml` back at the git
dependency and install the native toolchains.
