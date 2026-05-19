//! Smoke test 4 — secrets backend is swappable.
//!
//! Switching from `starter-secrets-file` to `starter-secrets-keyring`
//! (or any other `SecretStore` impl) must require **zero changes** to
//! a provider crate's `Config` struct or its tests. The contract — per
//! SCOPE rule R5 — is "the `Config` takes already-resolved
//! `SecretString` values; how the consumer resolved them is none of
//! the provider crate's business."
//!
//! The test verifies this by building each provider `Config` twice,
//! sourcing the same logical secret first through a fake "file-shaped"
//! `SecretStore` and then through a fake "keyring-shaped"
//! `SecretStore`. The provider `Config` type, its field names, and
//! the construction site are byte-identical across both branches —
//! the only thing that changes is the `Arc<dyn SecretStore>` passed to
//! `resolve`. If a future provider crate ever grew a backend-specific
//! field (`keyring_service_name: String`, `file_age_recipient: …`),
//! this test would fail to compile.
//!
//! We use hand-rolled in-memory `SecretStore` stand-ins rather than
//! the real `FileSecretStore` / `KeyringSecretStore` so the test does
//! not require a real OS keyring or age key-pair on the CI runner.
//! The real backends' `SecretStore` impls are exercised by their own
//! test suites — what matters here is the *seam*, not the backend.

use std::collections::HashMap;
use std::sync::Mutex;

use prometheus::Registry;
use starter_service_slack::{SlackSocketModeConfig, SlackSocketModeService};
use starter_service_telegram::{TelegramBotConfig, TelegramBotService};
use starter_spi::secrets::{Secret, SecretError, SecretStore};
use starter_spi::SecretString;
use starter_tool_gmail::{GmailConfig, GmailSendTool};
use starter_tool_slack::{SlackConfig, SlackPostTool};
use starter_tool_telegram::{TelegramConfig, TelegramSendMessageTool};

// A `SecretStore` impl that matches the file backend's shape (synchronous,
// fallible reads, owns its own storage). Stand-in for `FileSecretStore`.
struct FakeFileStore {
    entries: Mutex<HashMap<String, Secret>>,
}

impl FakeFileStore {
    fn with(seed: &[(&str, &str)]) -> Self {
        let entries = seed
            .iter()
            .map(|(k, v)| ((*k).to_string(), Secret::new((*v).to_string())))
            .collect();
        Self {
            entries: Mutex::new(entries),
        }
    }
}

impl SecretStore for FakeFileStore {
    fn ready(&self) -> bool {
        true
    }
    fn get(&self, name: &str) -> Result<Option<Secret>, SecretError> {
        Ok(self.entries.lock().unwrap().get(name).cloned())
    }
    fn put(&self, name: &str, value: Secret) -> Result<(), SecretError> {
        self.entries.lock().unwrap().insert(name.to_string(), value);
        Ok(())
    }
    fn delete(&self, name: &str) -> Result<(), SecretError> {
        self.entries.lock().unwrap().remove(name);
        Ok(())
    }
}

// A `SecretStore` impl that matches the keyring backend's shape. Stand-in
// for `KeyringSecretStore`.
struct FakeKeyringStore {
    entries: Mutex<HashMap<String, Secret>>,
}

impl FakeKeyringStore {
    fn with(seed: &[(&str, &str)]) -> Self {
        let entries = seed
            .iter()
            .map(|(k, v)| ((*k).to_string(), Secret::new((*v).to_string())))
            .collect();
        Self {
            entries: Mutex::new(entries),
        }
    }
}

impl SecretStore for FakeKeyringStore {
    fn ready(&self) -> bool {
        true
    }
    fn get(&self, name: &str) -> Result<Option<Secret>, SecretError> {
        Ok(self.entries.lock().unwrap().get(name).cloned())
    }
    fn put(&self, name: &str, value: Secret) -> Result<(), SecretError> {
        self.entries.lock().unwrap().insert(name.to_string(), value);
        Ok(())
    }
    fn delete(&self, name: &str) -> Result<(), SecretError> {
        self.entries.lock().unwrap().remove(name);
        Ok(())
    }
}

/// Resolve a named secret from a `SecretStore` into a
/// [`SecretString`] — the exact shape every provider `Config` accepts.
///
/// This is the only function the consumer's `main.rs` writes (or
/// imports from `starter-config`); the provider crates never see it.
fn resolve(store: &dyn SecretStore, name: &str) -> SecretString {
    let secret = store
        .get(name)
        .expect("store get")
        .unwrap_or_else(|| panic!("secret '{name}' missing"));
    SecretString::from(secret.expose().to_string())
}

#[test]
fn provider_configs_compile_unchanged_against_both_backends() {
    let seed = [
        ("slack.bot_token", "xoxb-test"),
        ("slack.signing_secret", "sig"),
        ("slack.app_token", "xapp-test"),
        ("telegram.bot_token", "12345:test"),
        ("gmail.oauth_access_token", "ya29.test"),
    ];

    let file_store = FakeFileStore::with(&seed);
    let keyring_store = FakeKeyringStore::with(&seed);

    // Build every Config struct twice — once per backend. The struct
    // body, field names, and types are identical across branches.
    // The only delta is the `&store` borrow inside `resolve(...)`.
    for (label, store) in [
        ("file", &file_store as &dyn SecretStore),
        ("keyring", &keyring_store as &dyn SecretStore),
    ] {
        let prom = Registry::new();

        let slack_tool_cfg = SlackConfig {
            bot_token: resolve(store, "slack.bot_token"),
            signing_secret: resolve(store, "slack.signing_secret"),
            base_url: SlackConfig::default_base_url(),
        };
        let _slack_tool = SlackPostTool::new(slack_tool_cfg, &prom)
            .unwrap_or_else(|e| panic!("[{label}] slack tool: {e}"));

        let slack_svc_cfg = SlackSocketModeConfig {
            app_token: resolve(store, "slack.app_token"),
            base_url: SlackSocketModeConfig::default_base_url(),
        };
        let _slack_svc = SlackSocketModeService::new(slack_svc_cfg);

        let tg_tool_cfg = TelegramConfig {
            bot_token: resolve(store, "telegram.bot_token"),
            base_url: TelegramConfig::default_base_url(),
        };
        let _tg_tool = TelegramSendMessageTool::new(tg_tool_cfg, &prom)
            .unwrap_or_else(|e| panic!("[{label}] telegram tool: {e}"));

        let tg_svc_cfg = TelegramBotConfig {
            bot_token: resolve(store, "telegram.bot_token"),
            base_url: TelegramBotConfig::default_base_url(),
        };
        let _tg_svc = TelegramBotService::new(tg_svc_cfg);

        let gmail_cfg = GmailConfig {
            oauth_access_token: resolve(store, "gmail.oauth_access_token"),
            user_id: GmailConfig::default_user_id(),
            base_url: GmailConfig::default_base_url(),
        };
        let _gmail = GmailSendTool::new(gmail_cfg, &prom)
            .unwrap_or_else(|e| panic!("[{label}] gmail tool: {e}"));
    }
}
