//! Panel DTOs. A panel belongs to a dashboard and runs one query against a
//! datasource, rendered as a chosen visualization.

mod create;
mod shared;
mod update;

pub use create::CreatePanelRequest;
pub use shared::PanelDetail;
pub use update::UpdatePanelRequest;
