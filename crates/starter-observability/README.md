# starter-observability

Tracing init, Prometheus metrics, and the data types used by
`starter-server` middleware. Pure data + setup; no HTTP layer here
(those live in `starter-server::middleware`).

## What's inside

- `tracing::init(filter, format)` → `TracingGuard` (Pretty / Json).
- `metrics::StandardMetrics::register(&registry)` — registers
  `starter_requests_total`, `starter_request_duration_seconds`,
  `starter_requests_in_flight`.
- `middleware::{RequestId, REQUEST_ID_HEADER}` — types shared with
  `starter-server`'s `with_request_id` / `with_latency` helpers.

## Usage

```rust
use starter_observability::{metrics::StandardMetrics, tracing::{init, Format}};

let _guard = init("info", Format::Pretty)?;

let registry = std::sync::Arc::new(prometheus::Registry::new());
let metrics = std::sync::Arc::new(StandardMetrics::register(&registry)?);
```

Hand `metrics` + `registry` to `ServerBuilder::with_metrics`. No
features.
