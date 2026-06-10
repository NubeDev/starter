//! The inference capability: messages in, completion out. Tier 1 of the facade.
//!
//! The `Inference` trait is defined unconditionally so the unified surface is the
//! same shape regardless of build features. The genai-backed implementation lives
//! behind the `inference` feature.

use async_trait::async_trait;
use futures::stream::BoxStream;
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::event::Event;
use crate::model::ModelRef;

/// One message in a chat exchange.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
}

impl Message {
    pub fn system(c: impl Into<String>) -> Self {
        Self { role: Role::System, content: c.into() }
    }
    pub fn user(c: impl Into<String>) -> Self {
        Self { role: Role::User, content: c.into() }
    }
    pub fn assistant(c: impl Into<String>) -> Self {
        Self { role: Role::Assistant, content: c.into() }
    }
}

/// A chat request. Facade-owned so callers never depend on genai's types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub model: ModelRef,
    pub messages: Vec<Message>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
}

impl ChatRequest {
    pub fn new(model: impl Into<ModelRef>, messages: Vec<Message>) -> Self {
        Self { model: model.into(), messages, temperature: None, max_tokens: None }
    }
}

/// A non-streaming chat response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub text: String,
    pub model: String,
    pub usage: Option<crate::event::Usage>,
}

/// Tier-1 capability: chat completions, blocking or streamed.
#[async_trait]
pub trait Inference: Send + Sync {
    async fn chat(&self, req: ChatRequest) -> Result<ChatResponse>;

    async fn stream(&self, req: ChatRequest) -> Result<BoxStream<'static, Result<Event>>>;
}

#[cfg(feature = "inference")]
pub use genai_impl::GenaiInference;

#[cfg(feature = "inference")]
mod genai_impl {
    use super::*;
    use crate::error::Error;
    use crate::event::{Event, Usage};
    use crate::model::AliasMap;
    use futures::StreamExt;
    use genai::chat::{ChatMessage, ChatRequest as GChatRequest, ChatRole};
    use genai::Client as GenaiClient;

    /// genai-backed [`Inference`]. Holds a genai client plus the size-alias map
    /// used to resolve [`ModelRef::Alias`] to concrete ids.
    pub struct GenaiInference {
        client: GenaiClient,
        aliases: AliasMap,
    }

    impl GenaiInference {
        pub fn new(aliases: AliasMap) -> Self {
            Self { client: GenaiClient::default(), aliases }
        }

        fn to_genai(&self, req: &ChatRequest) -> (String, GChatRequest) {
            let msgs: Vec<ChatMessage> = req
                .messages
                .iter()
                .map(|m| match m.role {
                    Role::System => ChatMessage::system(&m.content),
                    Role::User => ChatMessage::user(&m.content),
                    Role::Assistant => ChatMessage::assistant(&m.content),
                })
                .collect();
            let _ = ChatRole::System; // keep the import meaningful across genai versions
            (self.aliases.resolve(&req.model), GChatRequest::new(msgs))
        }
    }

    #[async_trait]
    impl Inference for GenaiInference {
        async fn chat(&self, req: ChatRequest) -> Result<ChatResponse> {
            let (model, greq) = self.to_genai(&req);
            let res = self
                .client
                .exec_chat(&model, greq, None)
                .await
                .map_err(|e| Error::Provider(e.to_string()))?;
            let usage = Some(Usage {
                input_tokens: res.usage.prompt_tokens.unwrap_or(0) as u64,
                output_tokens: res.usage.completion_tokens.unwrap_or(0) as u64,
            });
            Ok(ChatResponse {
                text: res.into_first_text().unwrap_or_default(),
                model,
                usage,
            })
        }

        async fn stream(
            &self,
            req: ChatRequest,
        ) -> Result<BoxStream<'static, Result<Event>>> {
            use genai::chat::ChatStreamEvent;
            let (model, greq) = self.to_genai(&req);
            let stream = self
                .client
                .exec_chat_stream(&model, greq, None)
                .await
                .map_err(|e| Error::Provider(e.to_string()))?;

            // Normalise genai's native stream events into the unified `Event`.
            let mapped = stream.stream.filter_map(|ev| async move {
                match ev {
                    Ok(ChatStreamEvent::Chunk(c)) => {
                        Some(Ok(Event::TextDelta { text: c.content }))
                    }
                    Ok(ChatStreamEvent::End(end)) => Some(Ok(Event::Done {
                        text: end
                            .captured_first_text()
                            .map(|s| s.to_string())
                            .unwrap_or_default(),
                        usage: None,
                    })),
                    // Start / reasoning chunks aren't part of the common surface;
                    // drop them rather than leak provider detail.
                    Ok(_) => None,
                    Err(e) => Some(Err(Error::Provider(e.to_string()))),
                }
            });

            Ok(mapped.boxed())
        }
    }
}
