//! `starter.warehouse.mart-read`. W14 filter validation + W11
//! envelope. The range parameter is typed `{ from, to }`; the
//! `max_buckets` cap (default 20_000) is bound at the HTTP layer
//! per M-4.

use std::str::FromStr;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use starter_flow_spi::node::{
    KindId, NodeBehavior, NodeCtx, NodeDescriptor, NodeError, SlotMap, SlotValue,
};
use starter_tags::TagQuery;

use super::runtime::WarehouseRuntime;
use crate::kinds::MART_READ;

pub const DEFAULT_MAX_BUCKETS: u32 = 20_000;

pub static DESCRIPTOR: NodeDescriptor = NodeDescriptor::new(
    MART_READ,
    "starter.warehouse.mart-read.label",
    "starter.warehouse.mart-read.summary",
    "starter.warehouse.mart-read.help",
);

pub struct MartRead {
    rt: Arc<WarehouseRuntime>,
    kind: KindId,
}

impl MartRead {
    pub fn new(rt: Arc<WarehouseRuntime>) -> Self {
        Self {
            rt,
            kind: KindId::new(MART_READ).unwrap(),
        }
    }
}

#[async_trait]
impl NodeBehavior for MartRead {
    fn kind_id(&self) -> &KindId {
        &self.kind
    }

    async fn invoke(&self, _ctx: NodeCtx<'_>, input: SlotMap) -> Result<SlotMap, NodeError> {
        let name = match input.get("name") {
            Some(SlotValue::String(s)) => s.clone(),
            _ => return Err(NodeError::InvalidInput("missing 'name' slot".into())),
        };
        let filter_str = match input.get("filter") {
            Some(SlotValue::String(s)) => s.clone(),
            _ => return Err(NodeError::InvalidInput("missing 'filter' slot".into())),
        };
        let filter =
            TagQuery::from_str(&filter_str).map_err(|e| NodeError::InvalidInput(e.to_string()))?;
        let (from, to) = parse_range(&input)?;
        let hide_unknown = matches!(input.get("hide_unknown"), Some(SlotValue::Bool(true)));
        let res = self
            .rt
            .mart_read(&name, filter, from, to, hide_unknown, DEFAULT_MAX_BUCKETS)
            .await
            .map_err(|e| match e {
                crate::nodes::runtime::RuntimeError::MartFilterUnsupportedKeys { .. } => {
                    NodeError::Domain {
                        code: "mart_filter_unsupported_keys",
                        message: e.to_string(),
                    }
                }
                other => NodeError::Backend(other.to_string()),
            })?;
        let mut out = SlotMap::new();
        out.insert(
            "result".into(),
            SlotValue::Json(serde_json::to_value(&res).unwrap()),
        );
        Ok(out)
    }
}

fn parse_range(input: &SlotMap) -> Result<(DateTime<Utc>, DateTime<Utc>), NodeError> {
    let range = match input.get("range") {
        Some(SlotValue::Json(v)) => v,
        _ => return Err(NodeError::InvalidInput("missing 'range' slot".into())),
    };
    let from = range
        .get("from")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<DateTime<Utc>>().ok())
        .ok_or_else(|| NodeError::InvalidInput("range.from missing".into()))?;
    let to = range
        .get("to")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<DateTime<Utc>>().ok())
        .ok_or_else(|| NodeError::InvalidInput("range.to missing".into()))?;
    Ok((from, to))
}
