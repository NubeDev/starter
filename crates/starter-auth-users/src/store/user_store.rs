//! `UserStore` — read/write the `starter_auth_users_users` table.

use async_trait::async_trait;

use crate::role::Role;

/// One user row, post-deserialization.
///
/// `password_hash` is `Option<String>` because a user may have been
/// created via a third-party sign-in path (OAuth, SAML, ...) and never
/// chosen a local password. `None` is **not** a wildcard — every login
/// route must reject it explicitly. See `POST /auth/login` for the
/// canonical `password_not_set` error path.
#[derive(Debug, Clone)]
pub struct UserRecord {
    /// Stable user id (the `Principal.subject`).
    pub id: String,
    /// Login email.
    pub email: String,
    /// argon2id PHC string, or `None` for a user with no local
    /// password (third-party sign-in only).
    pub password_hash: Option<String>,
    /// User's role.
    pub role: Role,
}

/// Errors specific to user persistence.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum UserStoreError {
    /// A user with the supplied email already exists.
    #[error("user already exists")]
    Conflict,
    /// Lookup found no matching row.
    #[error("user not found")]
    NotFound,
    /// Backing store failed.
    #[error("user store error: {0}")]
    Backend(String),
}

/// Persistence operations the user-management flow needs.
#[async_trait]
pub trait UserStore: Send + Sync {
    /// Insert a new user. Returns `Conflict` when the email is taken.
    ///
    /// `password_hash` is `None` for users created by a third-party
    /// sign-in path that has no local password. Backends must store
    /// `NULL` in the underlying column in that case.
    async fn create(
        &self,
        id: &str,
        email: &str,
        password_hash: Option<&str>,
        role: Role,
    ) -> Result<(), UserStoreError>;

    /// Fetch by email. `None` on miss.
    async fn find_by_email(&self, email: &str) -> Result<Option<UserRecord>, UserStoreError>;

    /// Fetch by id. `None` on miss.
    async fn find_by_id(&self, id: &str) -> Result<Option<UserRecord>, UserStoreError>;
}

#[cfg(feature = "sqlite")]
mod sqlite {
    use async_trait::async_trait;
    use sqlx::Row;
    use starter_store_sqlite::Pool;

    use super::{Role, UserRecord, UserStore, UserStoreError};

    /// sqlite-backed [`UserStore`].
    pub struct SqliteUserStore {
        pool: Pool,
    }

    impl SqliteUserStore {
        /// Wrap the pool. Run the `starter_auth_users` migrations
        /// first.
        pub fn new(pool: Pool) -> Self {
            Self { pool }
        }
    }

    fn err(e: sqlx::Error) -> UserStoreError {
        UserStoreError::Backend(e.to_string())
    }

    fn map_row(row: sqlx::sqlite::SqliteRow) -> Result<UserRecord, UserStoreError> {
        let role_str: String = row.get(3);
        let role = parse_role(&role_str)
            .ok_or_else(|| UserStoreError::Backend(format!("invalid role: {role_str}")))?;
        // `password_hash` is nullable now (migration 0002 in
        // starter-auth-oauth drops NOT NULL). `row.get::<Option<_>>` is
        // the only safe shape; reading it as `String` would panic on a
        // third-party-only user.
        let password_hash: Option<String> = row.get(2);
        Ok(UserRecord {
            id: row.get(0),
            email: row.get(1),
            password_hash,
            role,
        })
    }

    fn parse_role(s: &str) -> Option<Role> {
        match s {
            "reader" => Some(Role::Reader),
            "writer" => Some(Role::Writer),
            "admin" => Some(Role::Admin),
            _ => None,
        }
    }

    fn role_str(role: Role) -> &'static str {
        match role {
            Role::Reader => "reader",
            Role::Writer => "writer",
            Role::Admin => "admin",
        }
    }

    #[async_trait]
    impl UserStore for SqliteUserStore {
        async fn create(
            &self,
            id: &str,
            email: &str,
            password_hash: Option<&str>,
            role: Role,
        ) -> Result<(), UserStoreError> {
            let res = sqlx::query(
                "INSERT INTO starter_auth_users_users (id, email, password_hash, role) \
                 VALUES (?1, ?2, ?3, ?4)",
            )
            .bind(id)
            .bind(email)
            .bind(password_hash)
            .bind(role_str(role))
            .execute(self.pool.sqlx())
            .await;
            match res {
                Ok(_) => Ok(()),
                Err(sqlx::Error::Database(e)) if e.is_unique_violation() => {
                    Err(UserStoreError::Conflict)
                }
                Err(e) => Err(err(e)),
            }
        }

        async fn find_by_email(&self, email: &str) -> Result<Option<UserRecord>, UserStoreError> {
            let row = sqlx::query(
                "SELECT id, email, password_hash, role FROM starter_auth_users_users \
                 WHERE email = ?1 LIMIT 1",
            )
            .bind(email)
            .fetch_optional(self.pool.sqlx())
            .await
            .map_err(err)?;
            row.map(map_row).transpose()
        }

        async fn find_by_id(&self, id: &str) -> Result<Option<UserRecord>, UserStoreError> {
            let row = sqlx::query(
                "SELECT id, email, password_hash, role FROM starter_auth_users_users \
                 WHERE id = ?1 LIMIT 1",
            )
            .bind(id)
            .fetch_optional(self.pool.sqlx())
            .await
            .map_err(err)?;
            row.map(map_row).transpose()
        }
    }
}

#[cfg(feature = "sqlite")]
pub use sqlite::SqliteUserStore;
