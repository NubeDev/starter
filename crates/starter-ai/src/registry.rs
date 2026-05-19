//! Thread-safe registry mapping `Provider` -> `Arc<dyn AiRunner>`.
//!
//! Lifted from `codeless-workspace/ai-runner` per SCOPE q7. Each
//! built-in registration is gated on its provider feature, so a binary
//! that compiles with `--features provider-anthropic` populates only
//! that one runner and pulls only that one transport dep.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use starter_spi::ai::{AiRunner, Provider};

/// Thread-safe registry of AI runners. Clone the `Arc<Registry>` to
/// share it across tasks.
#[derive(Default)]
pub struct Registry {
    runners: RwLock<HashMap<Provider, Arc<dyn AiRunner>>>,
}

impl Registry {
    /// Build an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Build a registry pre-loaded with every provider whose feature
    /// is enabled at compile time.
    pub fn with_defaults() -> Self {
        let r = Self::new();
        #[cfg(feature = "provider-claude")]
        r.register(Arc::new(crate::runners::claude::ClaudeRunner));
        #[cfg(feature = "provider-codex")]
        r.register(Arc::new(crate::runners::codex::CodexRunner));
        #[cfg(feature = "provider-copilot")]
        r.register(Arc::new(crate::runners::copilot::CopilotRunner));
        #[cfg(feature = "provider-anthropic")]
        r.register(Arc::new(crate::runners::anthropic::AnthropicRunner));
        #[cfg(feature = "provider-openai")]
        r.register(Arc::new(crate::runners::openai::OpenAiRunner));
        r
    }

    /// Register (or replace) a runner.
    pub fn register(&self, runner: Arc<dyn AiRunner>) {
        let key = runner.provider().clone();
        self.runners
            .write()
            .expect("registry lock")
            .insert(key, runner);
    }

    /// Look up a runner by provider.
    pub fn get(&self, provider: &Provider) -> Option<Arc<dyn AiRunner>> {
        self.runners
            .read()
            .expect("registry lock")
            .get(provider)
            .cloned()
    }

    /// List all registered providers and their current readiness.
    ///
    /// The lock is released before any `await` so readiness probes do
    /// not block registration.
    pub async fn list(&self) -> Vec<ProviderStatus> {
        let runners: Vec<Arc<dyn AiRunner>> = self
            .runners
            .read()
            .expect("registry lock")
            .values()
            .cloned()
            .collect();
        let mut out = Vec::with_capacity(runners.len());
        for r in runners {
            let ready = r.ready().await;
            out.push(ProviderStatus {
                provider: r.provider().clone(),
                available: ready,
            });
        }
        out
    }
}

/// One row of [`Registry::list`].
#[derive(Debug, Clone)]
pub struct ProviderStatus {
    /// Which provider this row is for.
    pub provider: Provider,
    /// `true` when [`AiRunner::ready`] returned true at probe time.
    pub available: bool,
}
