//! Per-provider `AiRunner` impls. Each module is gated on its feature.

/// Anthropic cloud REST provider (`anthropic-ai-sdk`).
#[cfg(feature = "provider-anthropic")]
pub mod anthropic;
/// Claude Code CLI provider (`claude-wrapper`).
#[cfg(feature = "provider-claude")]
pub mod claude;
/// OpenAI Codex CLI provider (subprocess).
#[cfg(feature = "provider-codex")]
pub mod codex;
/// GitHub Copilot CLI provider (subprocess).
#[cfg(feature = "provider-copilot")]
pub mod copilot;
/// OpenAI cloud REST provider (`async-openai`).
#[cfg(feature = "provider-openai")]
pub mod openai;
