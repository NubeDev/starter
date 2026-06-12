//! Datasource DTOs — a datasource is an engine input type plus a saved,
//! envelope-encrypted connection config owned by a tenant.

mod create;
mod get;
mod kinds;
mod list;
mod schema;
mod shared;
mod test;
mod update;

pub use create::CreateDatasourceRequest;
pub use get::DatasourceDetail;
pub use kinds::{DatasourceKindList, DatasourceKindSummary};
pub use list::DatasourceSummary;
pub use schema::{DatasourceSchema, SchemaColumn, SchemaRelation, SchemaTable};
pub use shared::{DatasourceKind, RedactedConnection};
pub use test::{TestConnectionRequest, TestDatasourceResponse};
pub use update::UpdateDatasourceRequest;
