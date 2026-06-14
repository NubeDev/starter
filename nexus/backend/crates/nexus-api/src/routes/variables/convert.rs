//! Map between the variable store record and its wire DTO.
//!
//! The store keeps `kind` as its wire string; this is the single place the enum
//! and the string meet, so an unknown stored kind degrades to `Textbox` (a free
//! value) rather than failing a whole dashboard's variable list.

use nexus_spi::dto::variable::{VariableDetail, VariableKind};
use nexus_store::variable::VariableRecord;

/// Parse a stored kind string into the wire enum. An unrecognised value is
/// treated as `Textbox` — the most permissive kind — so a forward-rolled row
/// never breaks an older reader.
pub fn parse_kind(s: &str) -> VariableKind {
    match s {
        "constant" => VariableKind::Constant,
        "custom" => VariableKind::Custom,
        "query" => VariableKind::Query,
        "datasource" => VariableKind::Datasource,
        "interval" => VariableKind::Interval,
        "context" => VariableKind::Context,
        _ => VariableKind::Textbox,
    }
}

/// The wire string for a kind, used when persisting a create/update.
pub fn kind_str(kind: VariableKind) -> &'static str {
    match kind {
        VariableKind::Constant => "constant",
        VariableKind::Custom => "custom",
        VariableKind::Query => "query",
        VariableKind::Datasource => "datasource",
        VariableKind::Interval => "interval",
        VariableKind::Textbox => "textbox",
        VariableKind::Context => "context",
    }
}

/// Shape a stored record into its detail DTO.
pub fn to_detail(rec: &VariableRecord) -> VariableDetail {
    VariableDetail {
        id: rec.id,
        dashboard_id: rec.dashboard_id,
        name: rec.name.clone(),
        label: rec.label.clone(),
        kind: parse_kind(&rec.kind),
        options_config: rec.options_config.clone(),
        current: rec.current.clone(),
        multi: rec.multi,
        include_all: rec.include_all,
        hidden: rec.hidden,
        sort_order: rec.sort_order,
    }
}
