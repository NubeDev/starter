//! `rubix.user.create` — tool dispatch.
//!
//! Provisions a new user via the shared [`UserAdminStore`]. The
//! successful response carries a `Diagnostic` keyed
//! `rubix.user.created`. The companion `change_for` impl produces
//! a snapshot-shaped [`ChangeDraft`] (Op::Create, `after` =
//! [`UserRow`] JSON) so the undo dispatcher walks it back through
//! [`super::store::UserReversible`].
//!
//! See [docs/design/user-admin/](../../../../docs/design/user-admin/README.md).

use std::sync::Arc;

use async_trait::async_trait;
use rubix_spi::dto::user::create::{UserCreateRequest, UserCreateResponse};
use serde_json::Value;
use starter_spi::authz::ResourceRef;
use starter_spi::changelog::Op;
use starter_spi::error::{Error, Result};
use starter_spi::i18n::{Diagnostic, DiagnosticParam, MessageKey};
use starter_spi::tool::{Tool, ToolDefinition};
use starter_undo::ChangeDraft;
use uuid::Uuid;

use crate::undo::dispatch::ReversibleTool;
use crate::user::store::{UserAdminStore, UserRow, USER_KIND};

/// Concrete [`Tool`] for `rubix.user.create`.
pub struct UserCreateTool {
    store: Arc<dyn UserAdminStore>,
}

impl UserCreateTool {
    /// Wrap the shared store.
    pub fn new(store: Arc<dyn UserAdminStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl Tool for UserCreateTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "rubix.user.create".to_owned(),
            description: rubix_spi::dto::user::create::DESCRIPTOR.purpose.to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "email": { "type": "string", "format": "email" },
                    "role": { "type": "string", "enum": ["reader", "writer", "admin"] },
                    "password_hash": { "type": ["string", "null"] }
                },
                "required": ["email", "role"],
                "additionalProperties": false
            }),
        }
    }

    async fn invoke(&self, input: Value) -> Result<Value> {
        let req: UserCreateRequest =
            serde_json::from_value(input).map_err(|e| Error::Invalid {
                message: format!("UserCreateRequest: {e}"),
            })?;
        validate_role(&req.role)?;
        let user_id = format!("u-{}", Uuid::new_v4().simple());
        let created_at_ms = now_epoch_ms();
        let row = UserRow {
            user_id: user_id.clone(),
            email: req.email.clone(),
            role: req.role.clone(),
            disabled_at_ms: None,
        };
        let row = self.store.create(row).await?;

        let summary = Diagnostic::new(
            MessageKey::parse("rubix.user.created").expect("hard-coded key parses"),
        )
        .with_param("email", DiagnosticParam::String(row.email.clone()))
        .with_param("role", DiagnosticParam::String(row.role.clone()))
        .with_param("at", DiagnosticParam::Timestamp(created_at_ms));

        let response = UserCreateResponse {
            summary,
            user_id: row.user_id,
            email: row.email,
            role: row.role,
            created_at_ms,
        };
        serde_json::to_value(response).map_err(|e| Error::Internal {
            source: Box::new(e),
        })
    }
}

impl ReversibleTool for UserCreateTool {
    fn change_for(&self, _input: &Value, output: &Value) -> Option<ChangeDraft> {
        let resp: UserCreateResponse = serde_json::from_value(output.clone()).ok()?;
        let row = UserRow {
            user_id: resp.user_id.clone(),
            email: resp.email,
            role: resp.role,
            disabled_at_ms: None,
        };
        let after = serde_json::to_value(&row).ok()?;
        Some(ChangeDraft {
            resource: ResourceRef {
                kind: USER_KIND.into(),
                id: Some(row.user_id),
                owner: None,
                tenant: None,
            },
            op: Op::Create,
            before: None,
            after: Some(after),
            resource_version: None,
            correlation: None,
        })
    }
}

fn validate_role(role: &str) -> Result<()> {
    match role {
        "reader" | "writer" | "admin" => Ok(()),
        other => Err(Error::Invalid {
            message: format!("unknown role {other:?}; expected reader|writer|admin"),
        }),
    }
}

fn now_epoch_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::user::store::InMemoryUserStore;

    #[tokio::test]
    async fn create_emits_created_diagnostic_and_persists_row() {
        let store = Arc::new(InMemoryUserStore::new());
        let tool = UserCreateTool::new(store.clone());
        let out = tool
            .invoke(serde_json::json!({"email": "ada@x", "role": "admin"}))
            .await
            .unwrap();
        let resp: UserCreateResponse = serde_json::from_value(out).unwrap();
        assert_eq!(resp.summary.code.as_str(), "rubix.user.created");
        assert!(resp.user_id.starts_with("u-"));
        let row = store.get(&resp.user_id).await.unwrap().unwrap();
        assert_eq!(row.email, "ada@x");
    }

    #[tokio::test]
    async fn change_for_returns_create_draft_with_user_row_after() {
        let store = Arc::new(InMemoryUserStore::new());
        let tool = UserCreateTool::new(store);
        let input = serde_json::json!({"email": "a@x", "role": "reader"});
        let output = tool.invoke(input.clone()).await.unwrap();
        let draft = tool.change_for(&input, &output).expect("draft present");
        assert_eq!(draft.resource.kind, USER_KIND);
        assert!(matches!(draft.op, Op::Create));
        let row: UserRow = serde_json::from_value(draft.after.unwrap()).unwrap();
        assert_eq!(row.email, "a@x");
        assert_eq!(row.role, "reader");
    }

    #[tokio::test]
    async fn unknown_role_is_rejected() {
        let tool = UserCreateTool::new(Arc::new(InMemoryUserStore::new()));
        let err = tool
            .invoke(serde_json::json!({"email": "a@x", "role": "wizard"}))
            .await
            .unwrap_err();
        assert!(matches!(err, Error::Invalid { .. }));
    }
}
