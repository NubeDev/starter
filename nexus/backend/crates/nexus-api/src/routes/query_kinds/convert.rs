//! Map store query-kind records to wire DTOs.

use nexus_spi::dto::query_kind::QueryKindDetail;
use nexus_store::query_kind::QueryKindRecord;

pub fn to_detail(rec: &QueryKindRecord) -> QueryKindDetail {
    QueryKindDetail {
        id: rec.id,
        name: rec.name.clone(),
        sql: rec.sql.clone(),
        datasource_kind: rec.datasource_kind.clone(),
        tables: rec.tables.clone(),
        params_schema: rec.params_schema.clone(),
        datasource_binding: rec.datasource_binding.clone(),
        description: rec.description.clone(),
    }
}
