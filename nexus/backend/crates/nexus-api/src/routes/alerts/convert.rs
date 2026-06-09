//! Map alerting store records to wire DTOs.

use nexus_spi::dto::alert::{
    AlertCondition, AlertEvent, AlertRuleDetail, ChannelDetail, SilenceDetail,
};
use nexus_store::alert::{ChannelRecord, EventRecord, RuleRecord, SilenceRecord};

use crate::alerting::notify::redact_config;

pub fn rule_to_detail(r: &RuleRecord) -> AlertRuleDetail {
    AlertRuleDetail {
        id: r.id,
        name: r.name.clone(),
        datasource_id: r.datasource_id,
        query: r.query.clone(),
        op: r.op.clone(),
        threshold: r.threshold,
        for_secs: r.for_secs,
        interval_secs: r.interval_secs,
        enabled: r.enabled,
        channel_ids: r.channel_ids.clone(),
        conditions: r
            .conditions
            .as_ref()
            .and_then(|v| serde_json::from_value::<Vec<AlertCondition>>(v.clone()).ok()),
        combinator: r.combinator.clone(),
        no_data_policy: r.no_data_policy.clone(),
        exec_error_policy: r.exec_error_policy.clone(),
        message_template: r.message_template.clone(),
    }
}

/// Serialise the request's condition list to jsonb for the store. `None` (no
/// conditions supplied) leaves the rule on its legacy single-condition path.
pub fn conditions_to_json(conditions: Option<Vec<AlertCondition>>) -> Option<serde_json::Value> {
    conditions.and_then(|c| serde_json::to_value(c).ok())
}

pub fn event_to_dto(e: &EventRecord) -> AlertEvent {
    AlertEvent {
        id: e.id,
        rule_id: e.rule_id,
        at: e.at,
        transition: e.transition.clone(),
        value: e.value,
        silenced: e.silenced,
        notified: e.notified,
        detail: e.detail.clone(),
    }
}

/// Map a channel record to its wire DTO, redacting any secret config keys (a
/// Slack webhook URL, an SMTP password) so a token is never returned to a client.
pub fn channel_to_detail(c: &ChannelRecord) -> ChannelDetail {
    ChannelDetail {
        id: c.id,
        name: c.name.clone(),
        kind: c.kind.clone(),
        config: redact_config(&c.kind, &c.config),
    }
}

pub fn silence_to_detail(s: &SilenceRecord) -> SilenceDetail {
    SilenceDetail {
        id: s.id,
        rule_id: s.rule_id,
        starts_at: s.starts_at,
        ends_at: s.ends_at,
        reason: s.reason.clone(),
    }
}
