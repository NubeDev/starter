//! MCP `prompts` capability: parameterised, host-invokable templates.
//!
//! Mirrors [`ToolRegistry`](super::ToolRegistry) — registered at
//! startup, immutable for the rest of the process. Hosts that
//! advertise prompts (e.g. Claude Code) surface each entry as a
//! slash command (`/mcp__<server>__<name>`); tools never reach
//! that surface, which is why consumers that ship skill bundles
//! register them both as tools (for model-driven invocation) and
//! as prompts (for user-driven slash invocation).
//!
//! Spec: <https://modelcontextprotocol.io/specification/2024-11-05/server/prompts>

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use starter_spi::Result;

/// One argument accepted by a [`Prompt`].
#[derive(Debug, Clone)]
pub struct PromptArgument {
    /// Argument key the caller supplies under `arguments.<name>`.
    pub name: String,
    /// Human-readable description shown by the host's slash UI.
    pub description: Option<String>,
    /// If true, the host must collect a value before calling
    /// `prompts/get`.
    pub required: bool,
}

/// Static metadata for a registered [`Prompt`].
#[derive(Debug, Clone)]
pub struct PromptDefinition {
    /// Unique prompt name. Becomes the slash-command suffix in
    /// hosts that map prompts to slash commands.
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// Argument schema, in display order.
    pub arguments: Vec<PromptArgument>,
}

/// Role of a single [`PromptMessage`]. The MCP spec also defines
/// `system`, but the 2024-11-05 prompts surface restricts message
/// roles to `user` and `assistant`.
#[derive(Debug, Clone, Copy)]
pub enum PromptRole {
    /// Message authored by the human / host user.
    User,
    /// Message authored by the model.
    Assistant,
}

impl PromptRole {
    /// Wire representation: `"user"` or `"assistant"`.
    pub fn as_str(self) -> &'static str {
        match self {
            PromptRole::User => "user",
            PromptRole::Assistant => "assistant",
        }
    }
}

/// One message in the rendered prompt. v1 only ships text content;
/// the image / resource variants from the spec are not yet
/// emitted by any consumer, so they are deferred until a real
/// caller appears.
#[derive(Debug, Clone)]
pub struct PromptMessage {
    /// Sender role.
    pub role: PromptRole,
    /// Verbatim text content. Markdown-flavoured per host
    /// convention; this layer does not transform it.
    pub text: String,
}

/// Result of rendering a [`Prompt`].
#[derive(Debug, Clone)]
pub struct PromptResponse {
    /// Optional human-readable description override. When `None`
    /// the dispatcher omits the field from the response.
    pub description: Option<String>,
    /// Rendered message sequence.
    pub messages: Vec<PromptMessage>,
}

/// A registered prompt template. Implementors describe themselves
/// via [`Self::definition`] and render via [`Self::render`].
#[async_trait]
pub trait Prompt: Send + Sync + 'static {
    /// Static metadata returned by `prompts/list`.
    fn definition(&self) -> PromptDefinition;

    /// Render the prompt with caller-supplied arguments. The
    /// dispatcher passes the raw `arguments` JSON value (object
    /// or `null`); implementors are responsible for validation.
    async fn render(&self, arguments: Value) -> Result<PromptResponse>;
}

/// Map of prompt-name → boxed [`Prompt`]. Built at startup,
/// immutable during the server's lifetime.
#[derive(Default)]
pub struct PromptRegistry {
    prompts: HashMap<String, Arc<dyn Prompt>>,
}

impl PromptRegistry {
    /// Empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a prompt. Last-write-wins on duplicate names.
    pub fn register<P: Prompt>(mut self, prompt: P) -> Self {
        let def = prompt.definition();
        self.prompts.insert(def.name, Arc::new(prompt));
        self
    }

    /// Register an already-`Arc`-wrapped prompt. Same shape as
    /// [`Self::register`] for callers that need to share the same
    /// `Arc<dyn Prompt>` across multiple registries.
    pub fn register_arc(mut self, prompt: Arc<dyn Prompt>) -> Self {
        let def = prompt.definition();
        self.prompts.insert(def.name, prompt);
        self
    }

    /// All registered prompt definitions.
    pub fn list(&self) -> Vec<PromptDefinition> {
        self.prompts.values().map(|p| p.definition()).collect()
    }

    /// Look up a prompt by name.
    pub fn get(&self, name: &str) -> Option<Arc<dyn Prompt>> {
        self.prompts.get(name).cloned()
    }

    /// True when no prompts have been registered. Used by the
    /// dispatcher to decide whether to advertise the `prompts`
    /// capability in `initialize`.
    pub fn is_empty(&self) -> bool {
        self.prompts.is_empty()
    }
}
