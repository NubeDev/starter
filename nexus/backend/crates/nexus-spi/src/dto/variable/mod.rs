//! Dashboard-variable DTOs (WS-02) — a variable's definition, its kind, and the
//! create/update verbs. Variables are dashboard-scoped and persisted relationally
//! (`nexus_dashboard_variables`); their *resolved* values flow into a query via
//! the WS-03 binder as `QueryVariable`, so no SQL interpolation lives here.

pub mod create;
pub mod shared;
pub mod update;

pub use create::CreateVariableRequest;
pub use shared::{VariableDetail, VariableKind};
pub use update::UpdateVariableRequest;
