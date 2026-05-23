//! `starter.warehouse.mart-define`. W5 idempotency + W12 manifest
//! hash + ext re-quarantine (handled inside [`WarehouseRuntime`]).

use std::sync::Arc;

use async_trait::async_trait;
use starter_flow_spi::node::{
    KindId, NodeBehavior, NodeCtx, NodeDescriptor, NodeError, SlotMap, SlotValue,
};

use super::runtime::WarehouseRuntime;
use crate::catalog::mart_spec::MartSpec;
use crate::kinds::MART_DEFINE;

pub static DESCRIPTOR: NodeDescriptor = NodeDescriptor::new(
    MART_DEFINE,
    "starter.warehouse.mart-define.label",
    "starter.warehouse.mart-define.summary",
    "starter.warehouse.mart-define.help",
);

pub struct MartDefine {
    rt: Arc<WarehouseRuntime>,
    kind: KindId,
}

impl MartDefine {
    pub fn new(rt: Arc<WarehouseRuntime>) -> Self {
        Self {
            rt,
            kind: KindId::new(MART_DEFINE).unwrap(),
        }
    }
}

#[async_trait]
impl NodeBehavior for MartDefine {
    fn kind_id(&self) -> &KindId {
        &self.kind
    }

    async fn invoke(&self, _ctx: NodeCtx<'_>, input: SlotMap) -> Result<SlotMap, NodeError> {
        let spec_v = match input.get("spec") {
            Some(SlotValue::Json(v)) => v.clone(),
            _ => return Err(NodeError::InvalidInput("missing 'spec' slot".into())),
        };
        let spec: MartSpec =
            serde_json::from_value(spec_v).map_err(|e| NodeError::InvalidInput(e.to_string()))?;
        let res = self
            .rt
            .mart_define(spec)
            .await
            .map_err(|e| NodeError::Backend(e.to_string()))?;
        let mut out = SlotMap::new();
        out.insert("name".into(), SlotValue::String(res.name));
        out.insert("status".into(), SlotValue::String(res.status));
        out.insert(
            "promoted_columns".into(),
            SlotValue::Json(serde_json::Value::Array(
                res.promoted_columns
                    .into_iter()
                    .map(serde_json::Value::String)
                    .collect(),
            )),
        );
        out.insert(
            "idempotent_noop".into(),
            SlotValue::Bool(res.idempotent_noop),
        );
        Ok(out)
    }
}
