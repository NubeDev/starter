//! Shared mapping from the store's introspection result to the wire schema.
//!
//! Both the datasource schema route (`GET /datasources/:id/schema`) and the
//! nexus-DB schema route (`GET /nexus-db/schema`) return the same
//! [`DatasourceSchema`] shape, so the `SchemaInfo → DatasourceSchema` conversion
//! lives here once rather than being duplicated per handler. nexus-spi can't see
//! nexus-store's types, so the mapping can't be a `From` impl on the DTO — it's
//! this free function instead.

use nexus_spi::dto::datasource::{DatasourceSchema, SchemaColumn, SchemaRelation, SchemaTable};
use nexus_store::SchemaInfo;

/// Map the store's `SchemaInfo` (tables + FK relations) to the wire schema.
pub fn to_dto(info: SchemaInfo) -> DatasourceSchema {
    DatasourceSchema {
        tables: info
            .tables
            .into_iter()
            .map(|t| SchemaTable {
                schema: t.schema,
                name: t.name,
                columns: t
                    .columns
                    .into_iter()
                    .map(|c| SchemaColumn {
                        name: c.name,
                        data_type: c.data_type,
                    })
                    .collect(),
            })
            .collect(),
        relations: info
            .relations
            .into_iter()
            .map(|r| SchemaRelation {
                from_schema: r.from_schema,
                from_table: r.from_table,
                from_column: r.from_column,
                to_schema: r.to_schema,
                to_table: r.to_table,
                to_column: r.to_column,
            })
            .collect(),
    }
}
