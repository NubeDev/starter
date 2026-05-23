//! Persistence seams. One trait per row family (users / sessions /
//! tokens / tenants), each with its own SQLite impl behind
//! `feature = "sqlite"` and a Postgres impl behind
//! `feature = "postgres"`. The Postgres impls are landing one at a
//! time alongside their integration tests; see
//! [`crate::migration`] for the migration sources to apply first.

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

#[cfg(feature = "postgres")]
pub use user_store::PgUserStore;
