//! Nav-tree DTOs (WS-13) — the navigation + access surface. A nav node mounts a
//! reusable dashboard page (with a context payload) or a static app route into a
//! nestable, access-gated tree.

pub mod create;
pub mod shared;
pub mod update;

pub use create::CreateNavNodeRequest;
pub use shared::{NavContext, NavNodeDetail, NavTarget, StaticRoute};
pub use update::UpdateNavNodeRequest;
