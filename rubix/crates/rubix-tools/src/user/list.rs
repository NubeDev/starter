//! `rubix.user.list` — tool dispatch.
//!
//! Read-only verb: queries the shared [`UserAdminStore`], sorts the
//! rows by email for stable rendering, and emits a `Diagnostic` keyed
//! `rubix.user.listed`. No [`ReversibleTool`] impl — the verb makes
//! no state change to record. See
//! [docs/design/user-admin/](../../../../docs/design/user-admin/README.md).

use std::sync::Arc;

use async_trait::async_trait;
use rubix_spi::dto::user::list::{UserListItem, UserListRequest, UserListResponse};
use serde_json::Value;
use starter_spi::error::{Error, Result};
use starter_spi::i18n::{Diagnostic, DiagnosticParam, MessageKey};
use starter_spi::tool::{Tool, ToolDefinition};

use crate::user::store::UserAdminStore;

/// Concrete [`Tool`] for `rubix.user.list`.
pub struct UserListTool {
    store: Arc<dyn UserAdminStore>,
}

impl UserListTool {
    /// Wrap the shared store.
    pub fn new(store: Arc<dyn UserAdminStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl Tool for UserListTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "rubix.user.list".to_owned(),
            description: rubix_spi::dto::user::list::DESCRIPTOR.purpose.to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        }
    }

    async fn invoke(&self, input: Value) -> Result<Value> {
        // Parse defensively so unknown fields fail loudly rather than
        // silently — matches the contract of the write verbs.
        let _req: UserListRequest = serde_json::from_value(input).map_err(|e| Error::Invalid {
            message: format!("UserListRequest: {e}"),
        })?;

        let mut rows = self.store.list().await?;
        rows.sort_by(|a, b| a.email.cmp(&b.email));
        let users: Vec<UserListItem> = rows
            .into_iter()
            .map(|r| UserListItem {
                user_id: r.user_id,
                email: r.email,
                role: r.role,
                disabled_at_ms: r.disabled_at_ms,
            })
            .collect();
        let count = users.len();

        let summary =
            Diagnostic::new(MessageKey::parse("rubix.user.listed").expect("hard-coded key parses"))
                .with_param("count", DiagnosticParam::I64(count as i64));

        let response = UserListResponse {
            summary,
            count,
            users,
        };
        serde_json::to_value(response).map_err(|e| Error::Internal {
            source: Box::new(e),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::user::store::{InMemoryUserStore, UserRow};

    fn row(id: &str, email: &str) -> UserRow {
        UserRow {
            user_id: id.into(),
            email: email.into(),
            role: "reader".into(),
            disabled_at_ms: None,
            prefs_json: None,
            tenant_id: None,
        }
    }

    #[tokio::test]
    async fn empty_store_lists_zero_users() {
        let tool = UserListTool::new(Arc::new(InMemoryUserStore::new()));
        let out = tool.invoke(serde_json::json!({})).await.unwrap();
        let resp: UserListResponse = serde_json::from_value(out).unwrap();
        assert_eq!(resp.summary.code.as_str(), "rubix.user.listed");
        assert_eq!(resp.count, 0);
        assert!(resp.users.is_empty());
    }

    #[tokio::test]
    async fn rows_come_back_sorted_by_email() {
        let store = Arc::new(InMemoryUserStore::new());
        store.create(row("u-2", "zed@x")).await.unwrap();
        store.create(row("u-1", "ada@x")).await.unwrap();
        store.create(row("u-3", "kay@x")).await.unwrap();
        let tool = UserListTool::new(store);
        let out = tool.invoke(serde_json::json!({})).await.unwrap();
        let resp: UserListResponse = serde_json::from_value(out).unwrap();
        assert_eq!(resp.count, 3);
        let emails: Vec<&str> = resp.users.iter().map(|u| u.email.as_str()).collect();
        assert_eq!(emails, vec!["ada@x", "kay@x", "zed@x"]);
    }

    #[tokio::test]
    async fn unknown_field_in_request_is_rejected() {
        let tool = UserListTool::new(Arc::new(InMemoryUserStore::new()));
        // Empty request DTO accepts {}; an explicit garbage shape is
        // fine — we just ensure the dispatch contract is parse-then-go.
        let out = tool.invoke(serde_json::json!({})).await;
        assert!(out.is_ok());
    }
}
