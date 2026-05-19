# starter-server

axum app builder. Consumers construct `axum::Router<AppState>`s with
their own routes and hand them to `ServerBuilder`; this crate merges
them, mounts the starter-owned routes (`/health`, `/metrics`,
`/openapi.json`), wires middleware (CORS, tracing, request-id,
latency), and binds the listener.

## Usage

```rust
use std::sync::Arc;
use prometheus::Registry;
use starter_observability::metrics::StandardMetrics;
use starter_server::{builder::bind, ServerBuilder};

let registry = Arc::new(Registry::new());
let metrics  = Arc::new(StandardMetrics::register(&registry)?);

let router = ServerBuilder::<AppState>::new(state)
    .merge_router(my_routes())
    .with_metrics(registry, metrics)
    .with_openapi(MyApi::openapi())
    .build();

bind(router, "127.0.0.1:8080".parse()?).await?;
```

## Auth helpers

`auth::{with_principal, with_role, with_scope}` are router-extension
helpers parameterised over `Authenticator`. Apply `with_principal`
outermost so guards downstream see the populated extension.

## Features

- `testing` — opt-in test harness exposing `TestApp` (spawns a router
  on an ephemeral port). Pulls `reqwest` + `tokio` net.

Dependencies: `axum 0.8`, `tower-http`, `prometheus`, `utoipa 5`.
