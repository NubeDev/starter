//! `rubix.undo.last` — tool dispatch.
//!
//! Calls [`starter_undo::undo_last`] for the supplied actor.
//! Request/response DTOs and the [`ToolDefinition`] live in
//! [`rubix_spi::dto::undo::last`] per FILE-LAYOUT §2.
//!
//! See [`docs/design/undo/`](../../../../docs/design/undo/README.md).

use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::Value;
use starter_spi::changelog::Actor;
use starter_spi::error::{Error, Result};
use starter_spi::tool::{Tool, ToolDefinition};
use starter_undo::{undo_last, UndoService};

use crate::undo::dispatch::ActorSource;

/// Concrete [`Tool`] for `rubix.undo.last`. Holds the shared
/// [`UndoService`] and an [`ActorSource`] so the verb runs as the
/// caller, not as `Actor::System`.
pub struct UndoLastTool {
    service: Arc<UndoService>,
    actor: Arc<dyn ActorSource>,
}

impl UndoLastTool {
    /// New tool.
    pub fn new(service: Arc<UndoService>, actor: Arc<dyn ActorSource>) -> Self {
        Self { service, actor }
    }
}

#[derive(Debug, Default, Deserialize)]
struct UndoLastInput {
    // Reserved per-resource scope filter. Currently ignored — see
    // `starter_undo::undo_last` for the migration note. Held only so
    // the `additionalProperties: false` schema accepts the field
    // when clients send it.
    #[serde(default)]
    #[allow(dead_code)]
    scope: Option<serde_json::Value>,
}

#[async_trait]
impl Tool for UndoLastTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "rubix.undo.last".to_owned(),
            description: "Undo the most recent reversible change made by the calling actor."
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
        let _parsed: UndoLastInput = serde_json::from_value(input).map_err(|e| Error::Invalid {
            message: format!("UndoLastInput: {e}"),
        })?;

        let actor: Actor = self.actor.actor();
        let group = undo_last(&self.service, &actor, None).await?;
        Ok(serde_json::json!({ "group_id": group.0 }))
    }
}
