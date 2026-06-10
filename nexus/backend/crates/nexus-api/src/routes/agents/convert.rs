//! Map store agent/session records to wire DTOs.

use nexus_spi::dto::agent::{AgentDetail, AgentSummary, SessionDetail};
use nexus_store::agent::{AgentRecord, SessionRecord};

pub fn to_detail(rec: &AgentRecord) -> AgentDetail {
    AgentDetail {
        id: rec.id,
        name: rec.name.clone(),
        backend: rec.backend.clone(),
        model: rec.model.clone(),
        system_prompt: rec.system_prompt.clone(),
        config: rec.config.clone(),
    }
}

pub fn to_summary(rec: &AgentRecord) -> AgentSummary {
    AgentSummary {
        id: rec.id,
        name: rec.name.clone(),
        backend: rec.backend.clone(),
        model: rec.model.clone(),
    }
}

pub fn to_session(rec: &SessionRecord) -> SessionDetail {
    SessionDetail {
        id: rec.id,
        agent_id: rec.agent_id,
        status: rec.status.clone(),
        transcript: rec.transcript.clone(),
    }
}
