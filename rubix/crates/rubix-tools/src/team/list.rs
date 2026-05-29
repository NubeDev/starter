//! `rubix.team.list` \u{2014} tool dispatch.
//!
//! Read-only verb: queries the shared [`TeamAdminStore`], sorts
//! the rows by `name` for stable rendering, projects each row
//! to a [`TeamListItem`] (dropping the membership map, keeping
//! the count), and emits a `Diagnostic` keyed
//! `rubix.team.listed`. No [`ReversibleTool`] impl \u{2014} the
//! verb makes no state change to record.
//!
//! See the DTO module doc for the
//! `member_count`-instead-of-full-members bounding rationale.

use std::sync::Arc;

use async_trait::async_trait;
use rubix_spi::dto::team::list::{TeamListItem, TeamListRequest, TeamListResponse};
use serde_json::Value;
use starter_spi::error::{Error, Result};
use starter_spi::i18n::{Diagnostic, DiagnosticParam, MessageKey};
use starter_spi::tool::{Tool, ToolDefinition};

use crate::team::store::TeamAdminStore;

/// Concrete [`Tool`] for `rubix.team.list`.
pub struct TeamListTool {
    store: Arc<dyn TeamAdminStore>,
}

impl TeamListTool {
    /// Wrap the shared store.
    pub fn new(store: Arc<dyn TeamAdminStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl Tool for TeamListTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "rubix.team.list".to_owned(),
            description: rubix_spi::dto::team::list::DESCRIPTOR.purpose.to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        }
    }

    async fn invoke(&self, input: Value) -> Result<Value> {
        let _req: TeamListRequest = serde_json::from_value(input).map_err(|e| Error::Invalid {
            message: format!("TeamListRequest: {e}"),
        })?;

        let mut rows = self.store.list().await?;
        rows.sort_by(|a, b| a.name.cmp(&b.name));
        let teams: Vec<TeamListItem> = rows
            .into_iter()
            .map(|r| TeamListItem {
                team_id: r.team_id,
                name: r.name,
                description: r.description,
                member_count: r.members.len(),
            })
            .collect();
        let count = teams.len();

        let summary =
            Diagnostic::new(MessageKey::parse("rubix.team.listed").expect("hard-coded key parses"))
                .with_param("count", DiagnosticParam::I64(count as i64));

        let response = TeamListResponse {
            summary,
            count,
            teams,
        };
        serde_json::to_value(response).map_err(|e| Error::Internal {
            source: Box::new(e),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::team::store::{InMemoryTeamStore, TeamRow};

    fn row(id: &str, name: &str) -> TeamRow {
        TeamRow {
            team_id: id.into(),
            name: name.into(),
            description: None,
            members: BTreeMap::new(),
        }
    }

    fn row_with_members(id: &str, name: &str, members: &[(&str, i64)]) -> TeamRow {
        TeamRow {
            team_id: id.into(),
            name: name.into(),
            description: None,
            members: members.iter().map(|(u, t)| ((*u).into(), *t)).collect(),
        }
    }

    #[tokio::test]
    async fn empty_store_lists_zero_teams() {
        let tool = TeamListTool::new(Arc::new(InMemoryTeamStore::new()));
        let out = tool.invoke(serde_json::json!({})).await.unwrap();
        let resp: TeamListResponse = serde_json::from_value(out).unwrap();
        assert_eq!(resp.summary.code.as_str(), "rubix.team.listed");
        assert_eq!(resp.count, 0);
        assert!(resp.teams.is_empty());
    }

    #[tokio::test]
    async fn rows_come_back_sorted_by_name() {
        let store = Arc::new(InMemoryTeamStore::new());
        for r in [
            row("t-2", "Zenith"),
            row("t-1", "Acme"),
            row("t-3", "Kepler"),
        ] {
            store.create(r).await.expect("seed team");
        }
        let tool = TeamListTool::new(store);
        let out = tool.invoke(serde_json::json!({})).await.unwrap();
        let resp: TeamListResponse = serde_json::from_value(out).unwrap();
        assert_eq!(resp.count, 3);
        let names: Vec<&str> = resp.teams.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, vec!["Acme", "Kepler", "Zenith"]);
    }

    #[tokio::test]
    async fn member_count_reflects_membership_size() {
        let store = Arc::new(InMemoryTeamStore::new());
        store
            .create(row_with_members(
                "t-ops",
                "Ops",
                &[("u-1", 1_700_000_000_000), ("u-2", 1_700_000_000_001)],
            ))
            .await
            .expect("seed ops");
        store.create(row("t-sre", "SRE")).await.expect("seed sre");
        let tool = TeamListTool::new(store);
        let out = tool.invoke(serde_json::json!({})).await.unwrap();
        let resp: TeamListResponse = serde_json::from_value(out).unwrap();
        assert_eq!(resp.count, 2);
        let ops = resp.teams.iter().find(|t| t.team_id == "t-ops").unwrap();
        let sre = resp.teams.iter().find(|t| t.team_id == "t-sre").unwrap();
        assert_eq!(ops.member_count, 2);
        assert_eq!(sre.member_count, 0);
    }

    #[tokio::test]
    async fn description_round_trips_when_set() {
        let with_desc = TeamRow {
            team_id: "t-1".into(),
            name: "Ops".into(),
            description: Some("On-call rotation".into()),
            members: BTreeMap::new(),
        };
        let store = Arc::new(InMemoryTeamStore::new());
        store.create(with_desc).await.expect("seed with desc");
        let tool = TeamListTool::new(store);
        let out = tool.invoke(serde_json::json!({})).await.unwrap();
        let resp: TeamListResponse = serde_json::from_value(out).unwrap();
        assert_eq!(
            resp.teams[0].description.as_deref(),
            Some("On-call rotation")
        );
    }
}
