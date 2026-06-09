//! Dashboard DTOs. Grants and panel refs key on the immutable `id`; the slug is
//! a route alias resolved at the request edge.

mod create;
mod get;
mod list;
mod update;

pub use create::CreateDashboardRequest;
pub use get::DashboardDetail;
pub use list::DashboardSummary;
pub use update::UpdateDashboardRequest;
