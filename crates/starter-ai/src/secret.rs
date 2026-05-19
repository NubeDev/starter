//! Read provider API keys via `SecretStore`, falling back to the
//! standard env var for each provider.
//!
//! Wire-up: pass a `&dyn SecretStore` (or `None`) and the provider;
//! the helper checks `ai:<provider>:api_key` first, then the env var.

use starter_spi::ai::Provider;
use starter_spi::secrets::SecretStore;

/// Resolve the API key a REST runner should use. Lookup order:
///
/// 1. `SecretStore` (if supplied) at `ai:<provider>:api_key`.
/// 2. Standard provider env var (`ANTHROPIC_API_KEY`,
///    `OPENAI_API_KEY`).
///
/// Returns `None` for providers that don't take a key (CLI runners).
pub fn api_key_for(secrets: Option<&dyn SecretStore>, provider: &Provider) -> Option<String> {
    let key_name = match provider {
        Provider::Anthropic => "ai:anthropic:api_key",
        Provider::OpenAi => "ai:openai:api_key",
        Provider::Claude | Provider::Codex | Provider::Copilot => return None,
    };
    if let Some(store) = secrets {
        if let Ok(Some(v)) = store.get(key_name) {
            return Some(v.into_inner());
        }
    }
    let env_name = match provider {
        Provider::Anthropic => "ANTHROPIC_API_KEY",
        Provider::OpenAi => "OPENAI_API_KEY",
        _ => return None,
    };
    std::env::var(env_name)
        .ok()
        .filter(|v| !v.trim().is_empty())
}
