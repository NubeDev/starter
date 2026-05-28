//! `rubix.audit.policy.set` — tool dispatch.
//!
//! Upsert a single row in `changelog_kind_policy`. Reversible
//! via [`crate::audit::store::AuditPolicyReversible`] (snapshot
//! shape). Idempotent \u{2014} a second call with the same
//! `(kind, max_age_days)` returns the `rubix.audit.policy.unchanged`
//! diagnostic and skips the `ChangeDraft` so undo cannot revert
//! an unrelated edit.
//!
//! Op shape:
//! - `Op::Create` when no prior row existed (kind was implicitly
//!   unbounded). `before = null`, `after = AuditPolicyRow{...}`.
//! - `Op::Update` when a prior row existed with a different
//!   `max_age_days`. `before = AuditPolicyRow{...}` (the prior
//!   row, byte-exact incl. `updated_at_ms`), `after = new row`.

use std::sync::Arc;

use async_trait::async_trait;
use rubix_spi::dto::audit::policy_set::{
    AuditPolicyPriorSnapshot, AuditPolicySetRequest, AuditPolicySetResponse, AUDIT_POLICY_KIND,
};
use serde_json::Value;
use starter_spi::authz::ResourceRef;
use starter_spi::changelog::Op;
use starter_spi::error::{Error, Result};
use starter_spi::i18n::{Diagnostic, DiagnosticParam, MessageKey};
use starter_spi::tool::{Tool, ToolDefinition};
use starter_undo::ChangeDraft;

use crate::audit::store::{AuditPolicyRow, AuditPolicyStore};
use crate::undo::dispatch::ReversibleTool;
/// Concrete [`Tool`] for `rubix.audit.policy.set`.
pub struct AuditPolicySetTool {
    store: Arc<dyn AuditPolicyStore>,
}

impl AuditPolicySetTool {
    /// Wrap the shared store.
    pub fn new(store: Arc<dyn AuditPolicyStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl Tool for AuditPolicySetTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "rubix.audit.policy.set".to_owned(),
            description: rubix_spi::dto::audit::policy_set::DESCRIPTOR
                .purpose
                .to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "resource_kind": { "type": "string" },
                    "max_age_days": { "type": ["integer", "null"] }
                },
                "required": ["resource_kind"],
                "additionalProperties": false
            }),
        }
    }

    async fn invoke(&self, input: Value) -> Result<Value> {
        let req: AuditPolicySetRequest =
            serde_json::from_value(input).map_err(|e| Error::Invalid {
                message: format!("AuditPolicySetRequest: {e}"),
            })?;
        let kind = req.resource_kind.trim();
        if kind.is_empty() {
            return Err(Error::Invalid {
                message: "AuditPolicySetRequest.resource_kind must be non-empty".to_owned(),
            });
        }
        if let Some(days) = req.max_age_days {
            if days <= 0 {
                return Err(Error::Invalid {
                    message: format!(
                        "AuditPolicySetRequest.max_age_days must be > 0 when set, got {days}"
                    ),
                });
            }
        }

        let (prior, new) = self.store.upsert(kind, req.max_age_days).await?;
        let was_unchanged = prior
            .as_ref()
            .is_some_and(|p| p.max_age_days == new.max_age_days);

        let key = if was_unchanged {
            "rubix.audit.policy.unchanged"
        } else if new.max_age_days.is_none() {
            "rubix.audit.policy.pinned"
        } else {
            "rubix.audit.policy.set"
        };
        let summary = Diagnostic::new(MessageKey::parse(key).expect("hard-coded key parses"))
            .with_param("kind", DiagnosticParam::String(new.resource_kind.clone()))
            .with_param("at", DiagnosticParam::Timestamp(new.updated_at_ms));

        let response = AuditPolicySetResponse {
            summary,
            resource_kind: new.resource_kind,
            max_age_days: new.max_age_days,
            prior: prior.map(|p| AuditPolicyPriorSnapshot {
                max_age_days: p.max_age_days,
                updated_at_ms: p.updated_at_ms,
            }),
            was_unchanged,
            updated_at_ms: new.updated_at_ms,
        };
        serde_json::to_value(response).map_err(|e| Error::Internal {
            source: Box::new(e),
        })
    }
}

impl ReversibleTool for AuditPolicySetTool {
    fn change_for(&self, _input: &Value, output: &Value) -> Option<ChangeDraft> {
        let resp: AuditPolicySetResponse = serde_json::from_value(output.clone()).ok()?;
        if resp.was_unchanged {
            // No state change \u{2014} recording would let undo
            // silently revert a value the caller did not actually
            // flip and would also clobber the redo stack
            // (proposal \u{00a7}3.4).
            return None;
        }
        let after = AuditPolicyRow {
            resource_kind: resp.resource_kind.clone(),
            max_age_days: resp.max_age_days,
            updated_at_ms: resp.updated_at_ms,
        };
        let after_json = serde_json::to_value(&after).ok()?;
        let resource = ResourceRef {
            kind: AUDIT_POLICY_KIND.into(),
            id: Some(resp.resource_kind.clone()),
            owner: None,
            tenant: None,
        };
        match resp.prior {
            None => Some(ChangeDraft {
                resource,
                op: Op::Create,
                before: None,
                after: Some(after_json),
                resource_version: None,
                correlation: None,
            }),
            Some(prior) => {
                let before = AuditPolicyRow {
                    resource_kind: resp.resource_kind,
                    max_age_days: prior.max_age_days,
                    updated_at_ms: prior.updated_at_ms,
                };
                Some(ChangeDraft::update(
                    resource,
                    serde_json::to_value(&before).ok()?,
                    after_json,
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::store::{AuditPolicyReversible, InMemoryAuditPolicyStore};
    use starter_spi::changelog::{Actor, Change, ChangeId, GroupId, Reversible};

    fn seeded() -> Arc<InMemoryAuditPolicyStore> {
        Arc::new(InMemoryAuditPolicyStore::new())
    }

    #[tokio::test]
    async fn first_set_with_finite_curve_emits_set_diagnostic() {
        let store = seeded();
        let tool = AuditPolicySetTool::new(store);
        let out = tool
            .invoke(serde_json::json!({"resource_kind": "flow_def", "max_age_days": 30}))
            .await
            .unwrap();
        let resp: AuditPolicySetResponse = serde_json::from_value(out).unwrap();
        assert_eq!(resp.summary.code.as_str(), "rubix.audit.policy.set");
        assert!(!resp.was_unchanged);
        assert!(resp.prior.is_none());
        assert_eq!(resp.max_age_days, Some(30));
    }

    #[tokio::test]
    async fn first_set_with_null_curve_emits_pinned_diagnostic() {
        let store = seeded();
        let tool = AuditPolicySetTool::new(store);
        let out = tool
            .invoke(serde_json::json!({"resource_kind": "user"}))
            .await
            .unwrap();
        let resp: AuditPolicySetResponse = serde_json::from_value(out).unwrap();
        assert_eq!(resp.summary.code.as_str(), "rubix.audit.policy.pinned");
        assert!(resp.prior.is_none());
        assert_eq!(resp.max_age_days, None);
    }

    #[tokio::test]
    async fn second_set_with_same_value_is_unchanged_and_skips_draft() {
        let store = seeded();
        let tool = AuditPolicySetTool::new(store);
        let input = serde_json::json!({"resource_kind": "user", "max_age_days": 90});
        let _ = tool.invoke(input.clone()).await.unwrap();
        let out = tool.invoke(input.clone()).await.unwrap();
        let resp: AuditPolicySetResponse = serde_json::from_value(out.clone()).unwrap();
        assert_eq!(resp.summary.code.as_str(), "rubix.audit.policy.unchanged");
        assert!(resp.was_unchanged);
        assert!(
            tool.change_for(&input, &out).is_none(),
            "no-op set must not record a Change",
        );
    }

    #[tokio::test]
    async fn changing_curve_records_update_with_byte_exact_prior() {
        let store = seeded();
        let tool = AuditPolicySetTool::new(store);
        // First set: finite curve.
        let _ = tool
            .invoke(serde_json::json!({"resource_kind": "user", "max_age_days": 30}))
            .await
            .unwrap();
        // Tick the clock so updated_at_ms can diverge.
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        // Second set: pin to forever.
        let input = serde_json::json!({"resource_kind": "user"});
        let out = tool.invoke(input.clone()).await.unwrap();
        let resp: AuditPolicySetResponse = serde_json::from_value(out.clone()).unwrap();
        assert_eq!(resp.summary.code.as_str(), "rubix.audit.policy.pinned");
        let prior = resp.prior.clone().expect("prior expected");
        assert_eq!(prior.max_age_days, Some(30));

        let draft = tool.change_for(&input, &out).expect("draft expected");
        assert_eq!(draft.op, Op::Update);
        let before: AuditPolicyRow = serde_json::from_value(draft.before.unwrap()).unwrap();
        let after: AuditPolicyRow = serde_json::from_value(draft.after.unwrap()).unwrap();
        assert_eq!(before.max_age_days, Some(30));
        assert_eq!(after.max_age_days, None);
        // Critical: byte-exact prior updated_at_ms, not now().
        assert_eq!(before.updated_at_ms, prior.updated_at_ms);
        assert_eq!(after.updated_at_ms, resp.updated_at_ms);
    }

    #[tokio::test]
    async fn first_set_records_create_draft() {
        let store = seeded();
        let tool = AuditPolicySetTool::new(store);
        let input = serde_json::json!({"resource_kind": "flow_def", "max_age_days": 90});
        let out = tool.invoke(input.clone()).await.unwrap();
        let draft = tool.change_for(&input, &out).expect("draft expected");
        assert_eq!(draft.op, Op::Create);
        assert!(draft.before.is_none());
        let after: AuditPolicyRow = serde_json::from_value(draft.after.unwrap()).unwrap();
        assert_eq!(after.max_age_days, Some(90));
    }

    #[tokio::test]
    async fn zero_or_negative_max_age_days_is_rejected() {
        let store = seeded();
        let tool = AuditPolicySetTool::new(store);
        for bad in [0i32, -1, -90] {
            let err = tool
                .invoke(serde_json::json!({"resource_kind": "user", "max_age_days": bad}))
                .await
                .unwrap_err();
            assert!(matches!(err, Error::Invalid { .. }), "rejected {bad}");
        }
    }

    #[tokio::test]
    async fn empty_kind_is_rejected() {
        let store = seeded();
        let tool = AuditPolicySetTool::new(store);
        let err = tool
            .invoke(serde_json::json!({"resource_kind": "   "}))
            .await
            .unwrap_err();
        assert!(matches!(err, Error::Invalid { .. }));
    }

    #[tokio::test]
    async fn reversible_round_trip_restores_prior_curve_and_timestamp() {
        // Locks in the byte-exact prior-timestamp contract.
        let store = seeded();
        let tool = AuditPolicySetTool::new(store.clone());
        // Plant initial finite curve.
        let _ = tool
            .invoke(serde_json::json!({"resource_kind": "user", "max_age_days": 30}))
            .await
            .unwrap();
        let initial_ts = store.get("user").await.unwrap().unwrap().updated_at_ms;
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        // Operator pins to forever.
        let input = serde_json::json!({"resource_kind": "user"});
        let out = tool.invoke(input.clone()).await.unwrap();
        let draft = tool.change_for(&input, &out).expect("draft expected");

        let change = Change {
            id: ChangeId("c-1".into()),
            group_id: GroupId("g-1".into()),
            at: chrono::Utc::now(),
            actor: Actor::System,
            resource: draft.resource.clone(),
            op: draft.op,
            before: draft.before.clone(),
            after: draft.after.clone(),
            resource_version: None,
            correlation: None,
            patch: None,
        };
        let reversible = AuditPolicyReversible::new(store.clone());
        reversible.apply_inverse(&change).await.unwrap();
        let restored = store.get("user").await.unwrap().unwrap();
        assert_eq!(restored.max_age_days, Some(30));
        assert_eq!(restored.updated_at_ms, initial_ts);
    }

    #[tokio::test]
    async fn reversible_round_trip_undoes_create_by_deleting() {
        let store = seeded();
        let tool = AuditPolicySetTool::new(store.clone());
        let input = serde_json::json!({"resource_kind": "flow_def", "max_age_days": 90});
        let out = tool.invoke(input.clone()).await.unwrap();
        let draft = tool.change_for(&input, &out).expect("draft expected");

        let change = Change {
            id: ChangeId("c-1".into()),
            group_id: GroupId("g-1".into()),
            at: chrono::Utc::now(),
            actor: Actor::System,
            resource: draft.resource.clone(),
            op: draft.op,
            before: draft.before.clone(),
            after: draft.after.clone(),
            resource_version: None,
            correlation: None,
            patch: None,
        };
        let reversible = AuditPolicyReversible::new(store.clone());
        reversible.apply_inverse(&change).await.unwrap();
        assert!(store.get("flow_def").await.unwrap().is_none());
    }
}
