# starter-config

Layered configuration loader built on `figment`. Reads TOML files +
environment variables and produces a strongly-typed config struct.

## Usage

```rust
use starter_config::Loader;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct AppConfig {
    bind: String,
    database_url: String,
}

let cfg: AppConfig = Loader::new("STARTER")
    .with_file("config.toml")
    .load()?;
```

Env-var layer takes precedence over file layer; both fall back to
`serde` defaults on fields they don't set.

No features. Uses `figment::Error` (boxed via `Box<figment::Error>` to
keep `Result` cheap).
