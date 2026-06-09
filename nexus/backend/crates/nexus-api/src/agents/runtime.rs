//! In-memory runtime for agent sessions.
//!
//! Mirrors how live streams work: a session run is driven on a background task
//! that pushes events onto a per-session `broadcast` channel; the SSE endpoint
//! subscribes to that channel. The durable record (status + transcript) is
//! persisted to the store as the run progresses, so a session survives the
//! channel's lifetime even though the channel itself is ephemeral.
//!
//! The runtime drives the **inference** tier of the nexus-ai facade. The agent
//! tier (zag) plugs in here the same way once its adapter is wired — the
//! broadcast/persist machinery is tier-agnostic.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use nexus_ai::{ChatRequest, Client, Event, Inference as _, Message, ModelRef};
use nexus_skills::{BrevityMode, KnowledgeStore};
use sqlx::PgPool;
use tokio::sync::broadcast;
use uuid::Uuid;

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
    pub fn new(knowledge_root: impl Into<std::path::PathBuf>, brevity_default: BrevityMode) -> Self {
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

        // Assemble the system message off the runner's handles before spawning, so
        // the task captures only the finished strings.
        let system = self.assemble_system(&inputs, system_prompt);

        let client = self.client.clone();
        let channels = self.channels.clone();

        tokio::spawn(async move {
            // Build the message list: the assembled system block (brevity rule +
            // knowledge prefix + the agent's own system prompt), then the user
            // turn.
            let mut messages = Vec::new();
            if let Some(sys) = system.filter(|s| !s.is_empty()) {
                messages.push(Message::system(sys));
            }
            messages.push(Message::user(&prompt));

            let req = ChatRequest::new(model, messages);
            let outcome = run_and_broadcast(&client, req, &tx).await;

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
                &metadata, &tenant, session_id, &transcript,
            )
            .await;
            let _ =
                nexus_store::agent::set_session_status(&metadata, &tenant, session_id, status).await;

            channels
                .lock()
                .expect("session channels mutex")
                .remove(&session_id);
        });
    }

    /// One-shot, non-streaming completion. Drives the inference tier over a
    /// `system` + `user` pair and returns the full reply text. Unlike [`start`]
    /// this records nothing and opens no channel — it backs the synchronous AI
    /// assist endpoint (SQL generation, panel suggestion), where the caller wants
    /// a single structured answer, not a transcript. `system` is used verbatim
    /// (assist callers build their own task-specific instructions).
    pub async fn chat_once(
        &self,
        model: ModelRef,
        system: Option<String>,
        prompt: String,
    ) -> Result<String, String> {
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

/// Drive the inference stream, forwarding each event to subscribers and
/// accumulating the assistant text. Returns the full reply or an error message.
async fn run_and_broadcast(
    client: &Client,
    req: ChatRequest,
    tx: &broadcast::Sender<Event>,
) -> Result<String, String> {
    use futures::StreamExt;

    let mut stream = client
        .inference()
        .stream(req)
        .await
        .map_err(|e| e.to_string())?;

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
