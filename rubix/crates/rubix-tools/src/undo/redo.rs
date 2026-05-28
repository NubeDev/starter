//! `rubix.undo.redo` — tool dispatch.
//!
//! Mirror of [`crate::undo::last`]: pops the calling actor's most
//! recently undone group from the redo cursor and replays it
//! forward via [`starter_undo::redo_last`]. Request/response DTOs
//! match `rubix.undo.last` so the client surface stays symmetric.
//!
//! See [`docs/design/undo/`](../../../../docs/design/undo/README.md).

use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::Value;
use starter_spi::changelog::Actor;
use starter_spi::error::{Error, Result};
use starter_spi::tool::{Tool, ToolDefinition};
use starter_undo::{redo_last, UndoService};

use crate::undo::dispatch::ActorSource;

/// Concrete [`Tool`] for `rubix.undo.redo`. Holds the shared
/// [`UndoService`] and an [`ActorSource`] so the verb runs as the
/// caller, not as `Actor::System`.
pub struct UndoRedoTool {
    service: Arc<UndoService>,
    actor: Arc<dyn ActorSource>,
}

impl UndoRedoTool {
    /// New tool.
    pub fn new(service: Arc<UndoService>, actor: Arc<dyn ActorSource>) -> Self {
        Self { service, actor }
    }
}

#[derive(Debug, Default, Deserialize)]
struct UndoRedoInput {
    // Reserved per-resource scope filter; mirrors `rubix.undo.last`
    // so a single client serialiser handles both verbs.
    #[serde(default)]
    #[allow(dead_code)]
    scope: Option<serde_json::Value>,
}

#[async_trait]
impl Tool for UndoRedoTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "rubix.undo.redo".to_owned(),
            description:
                "Redo the most recently undone change made by the calling actor."
                    .to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "scope": {
                        "type": "object",
                        "description": "Reserved per-resource scope filter; ignored in this release."
                    }
                },
                "additionalProperties": false
            }),
        }
    }

    async fn invoke(&self, input: Value) -> Result<Value> {
        let _parsed: UndoRedoInput = serde_json::from_value(input).map_err(|e| Error::Invalid {
            message: format!("UndoRedoInput: {e}"),
        })?;

        let actor: Actor = self.actor.actor();
        let group = redo_last(&self.service, &actor, None).await?;
        Ok(serde_json::json!({ "group_id": group.0 }))
    }
}
