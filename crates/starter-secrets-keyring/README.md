# starter-secrets-keyring

`SecretStore` impl wrapping the OS keychain (macOS Keychain, Windows
Credential Manager, Linux Secret Service via DBus). Use this when the
consumer is a desktop / interactive binary.

## Usage

```rust
use starter_secrets_keyring::KeyringSecretStore;
use starter_spi::secrets::SecretStore;

let store = KeyringSecretStore::new("my-binary-name");
if !store.ready() {
    // Fall back to starter-secrets-file in headless / CI environments.
}
store.put("ai:anthropic:api_key", "sk-...".into())?;
```

Service name is the consumer's binary crate name; each entry's "user"
field is `<binary>:<name>` so two starter-based apps on the same
machine don't collide.

`ready()` probes with a benign `get_password` against the platform
service. `NoEntry` counts as ready; every other error means the
backend can't serve (Linux without DBus, etc.) — consumers should
feature-swap to `starter-secrets-file`.

## Features

Backend selection passed through to the `keyring` crate:

- `apple-native` (default on macOS) — uses Keychain Services.
- `windows-native` (default on Windows) — uses Credential Manager.
- `sync-secret-service` (default on Linux) — uses Secret Service via
  DBus.
- `crypto-rust` (default) — RustCrypto-based crypto so a default build
  doesn't pull libssl.
