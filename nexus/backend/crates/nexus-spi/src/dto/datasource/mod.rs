//! Datasource DTOs — a datasource is an ArkFlow input type plus a saved,
//! envelope-encrypted connection config owned by a tenant.

mod create;
mod get;
mod list;
mod schema;
mod shared;
mod test;
mod update;

pub use create::CreateDatasourceRequest;
pub use get::DatasourceDetail;
pub use list::DatasourceSummary;
pub use schema::{DatasourceSchema, SchemaColumn, SchemaTable};
pub use shared::{DatasourceKind, RedactedConnection};
pub use test::{TestConnectionRequest, TestDatasourceResponse};
pub use update::UpdateDatasourceRequest;
