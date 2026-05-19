# starter-secrets-file

`SecretStore` impl backed by a single age-encrypted JSON file. Use
this for headless / CI / appliance deployments where there's no OS
keyring.

## Usage

```rust
use starter_secrets_file::FileSecretStoreBuilder;
use starter_spi::secrets::SecretStore;

let store = FileSecretStoreBuilder::new("my-binary-name")
    .identity_path("./identity.age-key")  // optional override
    .build()?;

store.put("ai:openai:api_key", "sk-...".into())?;
```

## File layout

- Secrets: `$XDG_DATA_HOME/<binary>/secrets.age` (ASCII-armored age,
  JSON object inside).
- Identity: resolved in order — `STARTER_SECRETS_KEY` env var, then
  the consumer's config path, then first-run generation (writes
  `identity.age-key` next to the secrets file and emits a single
  `tracing::warn!` with the public key + backup path).

Atomic rename on write. `parking_lot::Mutex` cache so steady-state
reads don't re-decrypt.

No features.
