//! Persistence seams. One trait per row family (users / sessions /
//! tokens), each with its own sqlite impl behind `feature = "sqlite"`.
//! Postgres impls follow the same shape and land when postgres test
//! infra exists (Phase 2 tail).

mod session_store;
mod tenant_store;
mod token_store;
mod user_store;

pub use session_store::{SessionRecord, SessionStore, SessionStoreError};
pub use tenant_store::{
    is_reserved_slug, MembershipRecord, TeamRecord, TenantRecord, TenantStore, TenantStoreError,
    RESERVED_SLUGS,
};
pub use token_store::{TokenRecord, TokenStore, TokenStoreError};
pub use user_store::{UserRecord, UserStore, UserStoreError};

#[cfg(feature = "sqlite")]
pub use session_store::SqliteSessionStore;
#[cfg(feature = "sqlite")]
pub use tenant_store::SqliteTenantStore;
#[cfg(feature = "sqlite")]
pub use token_store::SqliteTokenStore;
#[cfg(feature = "sqlite")]
pub use user_store::SqliteUserStore;
