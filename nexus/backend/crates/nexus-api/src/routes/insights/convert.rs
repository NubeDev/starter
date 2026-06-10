//! Map a stored insight record to its wire summary.

use nexus_spi::dto::insight::InsightSummary;
use nexus_store::insight::InsightRecord;

/// Shape a store record into the API summary. A plain field copy — no logic.
pub fn to_summary(rec: &InsightRecord) -> InsightSummary {
    InsightSummary {
        id: rec.id,
        name: rec.name.clone(),
        script: rec.script.clone(),
        params_schema: rec.params_schema.clone(),
    }
}
