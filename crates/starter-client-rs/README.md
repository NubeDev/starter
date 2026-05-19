# starter-client-rs

Thin Rust HTTP client for a running starter-server. Used by the
`starter-cli` remote subcommands (`health`, `openapi`) and by consumer
integration tests.

## Usage

```rust
use starter_client_rs::Client;

let client = Client::new("http://localhost:8080".into(), None, None)?;
let health = client.health().await?;
let doc    = client.openapi().await?;
```

Optional `auth_token` and timeout overrides via the second/third
arguments.

No features. Built on `reqwest` with rustls.
