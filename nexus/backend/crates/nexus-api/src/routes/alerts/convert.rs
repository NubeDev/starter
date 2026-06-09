//! Map alerting store records to wire DTOs.

use nexus_spi::dto::alert::{AlertEvent, AlertRuleDetail, ChannelDetail, SilenceDetail};
use nexus_store::alert::{ChannelRecord, EventRecord, RuleRecord, SilenceRecord};

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
    }
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

pub fn channel_to_detail(c: &ChannelRecord) -> ChannelDetail {
    ChannelDetail {
        id: c.id,
        name: c.name.clone(),
        kind: c.kind.clone(),
        config: c.config.clone(),
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
