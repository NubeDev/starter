//! In-memory runtime for agent sessions.
//!
//! Mirrors how live streams work: a session run is driven on a background task
//! that pushes events onto a per-session `broadcast` channel; the SSE endpoint
//! subscribes to that channel. The durable record (status + transcript) is
//! persisted to the store as the run progresses, so a session survives the
//! channel's lifetime even though the channel itself is ephemeral.
//!
//! The runtime drives **either tier** of the nexus-ai facade depending on the
//! agent's backend, and the broadcast/persist machinery is identical for both:
//!
//!   * A **CLI agent** backend (`claude`, `codex`, `gemini`, `ollama`) routes to
//!     the zag *agent* tier, which drives the locally-installed coding-agent CLI
//!     using *its own* authentication — no provider API key in the control plane.
//!     This is the headline capability: the platform can run agents wherever the
//!     operator has a CLI logged in.
//!   * A raw **inference provider** backend (`anthropic`, `openai`, `gemini-api`,
//!     …) routes to the genai *inference* tier, which calls the provider HTTP API
//!     and needs a key in the environment.
//!
//! [`Backend::classify`] decides per agent from the backend string.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use nexus_ai::{AgentTask, ChatRequest, Client, Event, Inference as _, Message, ModelRef};
use nexus_skills::{BrevityMode, KnowledgeStore};
use sqlx::PgPool;
use tokio::sync::broadcast;
use uuid::Uuid;

/// Which nexus-ai tier an agent's backend routes to. The backend string on an
/// agent record is the discriminator; unknown backends default to the CLI tier
/// (the no-key path), since that is the project's primary mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    /// A coding-agent CLI driven by zag, using the CLI's own auth (no key).
    Cli,
    /// A raw inference provider called over HTTP by genai (needs an API key).
    Inference,
}

impl Backend {
    /// Classify a backend string. The known raw-provider names route to the
    /// inference tier; everything else (claude/codex/gemini/ollama and any
    /// unrecognised value) routes to the CLI tier, the no-key default.
    pub fn classify(backend: &str) -> Self {
        match backend.trim().to_ascii_lowercase().as_str() {
            // Raw HTTP providers — need a key, go through genai.
            "anthropic" | "openai" | "gemini-api" | "google" | "groq" | "xai" | "mistral"
            | "cohere" | "deepseek" | "inference" => Backend::Inference,
            // CLI agents (and anything unknown) — driven by zag, no key.
            _ => Backend::Cli,
        }
    }
}

/// How many events a slow SSE subscriber may fall behind before it lags. Matches
/// the live-stream broadcast sizing.
const CHANNEL_CAPACITY: usize = 256;

/// Per-run knowledge selection, parsed from an agent's `config` blob. An agent
/// names exactly the skills/rules it wants and an optional brevity level; the
/// runner resolves these against the knowledge root and the service brevity
/// default to build the prompt prefix. Absent/malformed fields default to empty,
/// so an agent with no config simply gets no injected knowledge.
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct PromptInputs {
    #[serde(default)]
    pub skills: Vec<String>,
    #[serde(default)]
    pub rules: Vec<String>,
    #[serde(default)]
    pub brevity: BrevityMode,
}

impl PromptInputs {
    /// Parse the inputs from an agent config `Value`, tolerating any shape: a
    /// non-object or missing fields yield defaults rather than an error.
    pub fn from_config(config: &serde_json::Value) -> Self {
        serde_json::from_value(config.clone()).unwrap_or_default()
    }
}

/// The parameters of one session run, bundled so [`SessionRunner::start`] takes
/// a single argument. `metadata` is the control-plane pool the transcript is
/// persisted to under `tenant`'s RLS; `system_prompt`/`inputs` build the system
/// message; `prompt` is the opening user turn.
pub struct SessionRun {
    pub metadata: PgPool,
    pub tenant: String,
    pub session_id: Uuid,
    /// The agent's backend string — decides which tier drives the run
    /// ([`Backend::classify`]).
    pub backend: String,
    pub model: ModelRef,
    pub system_prompt: Option<String>,
    pub inputs: PromptInputs,
    pub prompt: String,
}

/// Cloneable handle to the session runtime. Holds the AI client, the knowledge
/// store, the service brevity default, and the live broadcast channels keyed by
/// session id.
#[derive(Clone)]
pub struct SessionRunner {
    client: Arc<Client>,
    knowledge: Arc<KnowledgeStore>,
    brevity_default: BrevityMode,
    channels: Arc<Mutex<HashMap<Uuid, broadcast::Sender<Event>>>>,
}

impl SessionRunner {
    /// Build with a knowledge root and the service-wide brevity default. The root
    /// is expected to contain `skills/` and `rules/` subdirs; a missing root just
    /// means every named skill resolves to "missing" and nothing is injected.
    pub fn new(
        knowledge_root: impl Into<std::path::PathBuf>,
        brevity_default: BrevityMode,
    ) -> Self {
        Self {
            client: Arc::new(Client::new()),
            knowledge: Arc::new(KnowledgeStore::new(knowledge_root)),
            brevity_default,
            channels: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Subscribe to a running session's event feed. `None` if no run is live for
    /// that id (it already finished, or never started on this node).
    pub fn subscribe(&self, session_id: Uuid) -> Option<broadcast::Receiver<Event>> {
        self.channels
            .lock()
            .expect("session channels mutex")
            .get(&session_id)
            .map(|tx| tx.subscribe())
    }

    /// Start streaming a session run on a background task. The run drives the
    /// inference tier over `messages`, broadcasts each [`Event`], accumulates the
    /// assistant reply, and on completion persists the full transcript and a
    /// terminal status. `run.metadata` is the control-plane pool the transcript is
    /// written back to under the tenant's RLS.
    pub fn start(&self, run: SessionRun) {
        let SessionRun {
            metadata,
            tenant,
            session_id,
            backend,
            model,
            system_prompt,
            inputs,
            prompt,
        } = run;

        let (tx, _rx) = broadcast::channel(CHANNEL_CAPACITY);
        self.channels
            .lock()
            .expect("session channels mutex")
            .insert(session_id, tx.clone());

        // Assemble the system block (brevity rule + knowledge prefix + the agent's
        // own system prompt) off the runner's handles before spawning, so the task
        // captures only the finished strings.
        let system = self.assemble_system(&inputs, system_prompt);
        let tier = Backend::classify(&backend);

        let client = self.client.clone();
        let channels = self.channels.clone();

        tokio::spawn(async move {
            // Route by tier. The CLI tier drives the local agent binary via zag
            // (no key); the inference tier calls the provider API via genai. Both
            // broadcast unified events onto `tx` and return the full reply.
            let outcome = match tier {
                Backend::Cli => {
                    run_agent_broadcast(&client, &backend, model, system.clone(), &prompt, &tx)
                        .await
                }
                Backend::Inference => {
                    let mut messages = Vec::new();
                    if let Some(sys) = system.clone().filter(|s| !s.is_empty()) {
                        messages.push(Message::system(sys));
                    }
                    messages.push(Message::user(&prompt));
                    run_inference_broadcast(&client, ChatRequest::new(model, messages), &tx).await
                }
            };

            // Persist terminal state and the assembled transcript, then drop the
            // channel so subscribers see the stream close.
            let (status, reply) = match outcome {
                Ok(text) => ("completed", text),
                Err(msg) => {
                    let _ = tx.send(Event::Raw(serde_json::json!({ "error": msg })));
                    ("failed", String::new())
                }
            };

            let transcript = serde_json::json!([
                { "role": "user", "content": prompt },
                { "role": "assistant", "content": reply },
            ]);
            let _ = nexus_store::agent::set_session_transcript(
                &metadata,
                &tenant,
                session_id,
                &transcript,
            )
            .await;
            let _ = nexus_store::agent::set_session_status(&metadata, &tenant, session_id, status)
                .await;

            channels
                .lock()
                .expect("session channels mutex")
                .remove(&session_id);
        });
    }

    /// One-shot, non-streaming completion routed by `backend`. Returns the full
    /// reply text. Unlike [`start`] this records nothing and opens no channel — it
    /// backs the synchronous AI assist endpoint (SQL generation, panel
    /// suggestion). A CLI backend runs the local agent via zag (no key); a
    /// provider backend calls genai. `system` is used verbatim (assist callers
    /// build their own task-specific instructions).
    pub async fn chat_once(
        &self,
        backend: &str,
        model: ModelRef,
        system: Option<String>,
        prompt: String,
    ) -> Result<String, String> {
        match Backend::classify(backend) {
            Backend::Cli => {
                // CLI agents take a single prompt; prepend the system block so the
                // task-specific instructions still apply.
                let composed = compose_prompt(system.as_deref(), &prompt);
                let task = AgentTask {
                    backend: backend.to_string(),
                    prompt: composed,
                    model: Some(model),
                    cwd: None,
                    isolate_worktree: false,
                };
                let agent = self.client.agent().map_err(|e| e.to_string())?;
                agent
                    .run(task)
                    .await
                    .map(|o| o.text)
                    .map_err(|e| e.to_string())
            }
            Backend::Inference => {
                let mut messages = Vec::new();
                if let Some(sys) = system.filter(|s| !s.is_empty()) {
                    messages.push(Message::system(sys));
                }
                messages.push(Message::user(&prompt));
                let req = ChatRequest::new(model, messages);
                self.client
                    .inference()
                    .chat(req)
                    .await
                    .map(|res| res.text)
                    .map_err(|e| e.to_string())
            }
        }
    }

    /// Build the system message: the brevity rule, then the resolved knowledge
    /// prefix (named skills + rules), then the agent's own system prompt — each
    /// self-delimiting and separator-terminated so they concatenate cleanly.
    /// `None` when every part is empty.
    fn assemble_system(
        &self,
        inputs: &PromptInputs,
        system_prompt: Option<String>,
    ) -> Option<String> {
        let mut out = String::new();
        if let Some(b) = inputs.brevity.render_prompt_prefix(self.brevity_default) {
            out.push_str(&b);
        }
        let bundle = self.knowledge.load(&inputs.skills, &inputs.rules);
        if let Some(k) = bundle.render_prompt_prefix() {
            out.push_str(&k);
        }
        if let Some(sys) = system_prompt.filter(|s| !s.is_empty()) {
            out.push_str(sys.trim_end());
        }
        let out = out.trim().to_string();
        (!out.is_empty()).then_some(out)
    }
}

/// Compose a CLI agent's single prompt from an optional system block and the
/// user turn. CLI agents (Claude Code, Codex, …) take one prompt string rather
/// than a role-tagged message list, so the system context is prepended.
fn compose_prompt(system: Option<&str>, prompt: &str) -> String {
    match system.map(str::trim).filter(|s| !s.is_empty()) {
        Some(sys) => format!("{sys}\n\n{prompt}"),
        None => prompt.to_string(),
    }
}

/// Drive the **inference** tier stream into the broadcast. Returns the full reply.
async fn run_inference_broadcast(
    client: &Client,
    req: ChatRequest,
    tx: &broadcast::Sender<Event>,
) -> Result<String, String> {
    let stream = client
        .inference()
        .stream(req)
        .await
        .map_err(|e| e.to_string())?;
    forward_stream(stream, tx).await
}

/// Drive the **agent** (CLI) tier stream into the broadcast. Builds the zag task
/// from the backend + composed prompt, then forwards its events. Returns the
/// full reply. The CLI uses its own auth — no provider key is read here.
async fn run_agent_broadcast(
    client: &Client,
    backend: &str,
    model: ModelRef,
    system: Option<String>,
    prompt: &str,
    tx: &broadcast::Sender<Event>,
) -> Result<String, String> {
    let task = AgentTask {
        backend: backend.to_string(),
        prompt: compose_prompt(system.as_deref(), prompt),
        model: Some(model),
        cwd: None,
        isolate_worktree: false,
    };
    let agent = client.agent().map_err(|e| e.to_string())?;
    let stream = agent.run_stream(task).await.map_err(|e| e.to_string())?;
    forward_stream(stream, tx).await
}

/// Forward a unified event stream onto the broadcast channel, accumulating the
/// assistant text. Shared by both tiers — the event shape is the same. Returns
/// the full reply or an error message.
async fn forward_stream(
    mut stream: futures::stream::BoxStream<'static, nexus_ai::Result<Event>>,
    tx: &broadcast::Sender<Event>,
) -> Result<String, String> {
    use futures::StreamExt;

    let mut reply = String::new();
    while let Some(item) = stream.next().await {
        match item {
            Ok(ev) => {
                match &ev {
                    Event::TextDelta { text } => reply.push_str(text),
                    Event::Done { text, .. } if !text.is_empty() => reply = text.clone(),
                    _ => {}
                }
                // A send error means no subscribers are attached yet/anymore; the
                // run still completes and persists, so the error is ignored.
                let _ = tx.send(ev);
            }
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(reply)
}
