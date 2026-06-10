//! Nexus unified AI surface.
//!
//! Two stacks live at different layers of the AI stack and this crate makes them
//! one entry point without merging them into a confused single API:
//!
//!   * **Inference** (tier 1) — messages -> completion, backed by `genai`.
//!   * **Agent**     (tier 2) — task -> repo-editing session, backed by `zag`.
//!
//! A "chat completion" and "run an autonomous agent on my repo" are genuinely
//! different operations, so they are two traits ([`Inference`], [`Agent`]) sharing
//! one [`Client`], one credentials/alias story, one [`Event`] stream, and one
//! [`Error`]. Provider-specific power features stay reachable through typed escape
//! hatches (e.g. [`Event::Raw`]) rather than being flattened away.
//!
//! ```ignore
//! let ai = Client::new();
//! // tier 1 — inference
//! let res = ai.chat(ChatRequest::new(ModelRef::large(), vec![Message::user("hi")])).await?;
//! // tier 2 — agents (requires the `agent` feature)
//! let out = ai.agent()?.run(AgentTask::new("claude", "refactor authz panel")).await?;
//! ```

pub mod agent;
pub mod error;
pub mod event;
pub mod inference;
pub mod model;

pub use agent::{Agent, AgentOutcome, AgentTask};
pub use error::{Error, Result};
pub use event::{Event, Usage};
pub use inference::{ChatRequest, ChatResponse, Inference, Message, Role};
pub use model::{AliasMap, ModelRef, Size};

/// The unified front door. Holds whichever capability impls the build enabled.
pub struct Client {
    aliases: AliasMap,
    #[cfg(feature = "inference")]
    inference: inference::GenaiInference,
    #[cfg(feature = "agent")]
    agent: agent::ZagAgent,
}

impl Client {
    /// Construct with the default Claude-tier alias map.
    pub fn new() -> Self {
        Self::with_aliases(AliasMap::default())
    }

    /// Construct with a custom size-alias map (e.g. point `large` at a different
    /// provider's flagship).
    pub fn with_aliases(aliases: AliasMap) -> Self {
        Self {
            #[cfg(feature = "inference")]
            inference: inference::GenaiInference::new(aliases.clone()),
            #[cfg(feature = "agent")]
            agent: agent::ZagAgent::new(aliases.clone()),
            aliases,
        }
    }

    /// The configured size-alias map.
    pub fn aliases(&self) -> &AliasMap {
        &self.aliases
    }

    /// Tier-1 inference handle. Available only with the `inference` feature.
    #[cfg(feature = "inference")]
    pub fn inference(&self) -> &impl Inference {
        &self.inference
    }

    /// Tier-2 agent handle. `Err(Unsupported)` if the `agent` feature is off, so
    /// callers get a clear runtime signal rather than a missing method.
    pub fn agent(&self) -> Result<&dyn Agent> {
        #[cfg(feature = "agent")]
        {
            Ok(&self.agent)
        }
        #[cfg(not(feature = "agent"))]
        {
            Err(Error::Unsupported("agent feature not enabled"))
        }
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alias_resolves_to_claude_tiers() {
        let m = AliasMap::default();
        assert_eq!(m.resolve(&ModelRef::large()), "claude-opus-4-8");
        assert_eq!(m.resolve(&ModelRef::small()), "claude-haiku-4-5");
        assert_eq!(m.resolve(&ModelRef::concrete("gpt-5")), "gpt-5");
    }

    #[test]
    fn agent_handle_errors_without_feature() {
        let c = Client::new();
        #[cfg(not(feature = "agent"))]
        assert!(matches!(c.agent(), Err(Error::Unsupported(_))));
        let _ = c;
    }
}
