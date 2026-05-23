//! `starter.warehouse.cleaner-define`. RF-6 sync→async
//! auto-promotion + entity-validate enum + sandbox freeze.

use std::sync::Arc;

use async_trait::async_trait;
use starter_flow_spi::node::{
    KindId, NodeBehavior, NodeCtx, NodeDescriptor, NodeError, SlotMap, SlotValue,
};

use super::runtime::WarehouseRuntime;
use crate::ddl::cleaner::CleanerSpec;
use crate::kinds::CLEANER_DEFINE;

pub static DESCRIPTOR: NodeDescriptor = NodeDescriptor::new(
    CLEANER_DEFINE,
    "starter.warehouse.cleaner-define.label",
    "starter.warehouse.cleaner-define.summary",
    "starter.warehouse.cleaner-define.help",
);

pub struct CleanerDefine {
    rt: Arc<WarehouseRuntime>,
    kind: KindId,
}

impl CleanerDefine {
    pub fn new(rt: Arc<WarehouseRuntime>) -> Self {
        Self { rt, kind: KindId::new(CLEANER_DEFINE).unwrap() }
    }
}

#[async_trait]
impl NodeBehavior for CleanerDefine {
    fn kind_id(&self) -> &KindId {
        &self.kind
    }

    async fn invoke(&self, _ctx: NodeCtx<'_>, input: SlotMap) -> Result<SlotMap, NodeError> {
        let spec_v = match input.get("spec") {
            Some(SlotValue::Json(v)) => v.clone(),
            _ => return Err(NodeError::InvalidInput("missing 'spec' slot".into())),
        };
        let spec: CleanerSpec = serde_json::from_value(spec_v)
            .map_err(|e| NodeError::InvalidInput(e.to_string()))?;
        let created_by = match input.get("created_by") {
            Some(SlotValue::String(s)) => s.clone(),
            _ => "user:unknown".to_string(),
        };
        let source_count = match input.get("source_row_count") {
            Some(SlotValue::Int(n)) => *n as u64,
            _ => 0,
        };
        let res = self
            .rt
            .cleaner_define(spec, &created_by, source_count)
            .await
            .map_err(|e| NodeError::Backend(e.to_string()))?;
        let mut out = SlotMap::new();
        out.insert("view_name".into(), SlotValue::String(res.view_name));
        out.insert(
            "effective_backfill".into(),
            SlotValue::String(res.effective_backfill),
        );
        out.insert(
            "auto_promoted".into(),
            SlotValue::Bool(res.auto_promoted),
        );
        Ok(out)
    }
}
