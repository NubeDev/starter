//! Write a user's local password — the operator reset lever
//! ([`set_password`]) and the self-serve rotation
//! ([`change_password`]).
//!
//! Both funnel into one private `write` helper so the strength policy
//! and the hashing parameters cannot drift between the two paths. The
//! only difference is what each proves before writing: the operator
//! path proves authorization out-of-band (RBAC at the HTTP layer), the
//! self-serve path proves knowledge of the password being replaced.

use crate::password;
use crate::signup::validate::{self, ValidationError};
use crate::store::{UserStore, UserStoreError};

use super::AdminError;

/// Validate `new_password` against the signup policy, hash it, and
/// store it for `user_id`.
///
/// The user's own email is fed to the validator so the "password must
/// not match the email local part" rule applies here too; a missing
/// row is reported before any hashing work is done.
async fn write<U: UserStore + ?Sized>(
    store: &U,
    user_id: &str,
    email: &str,
    new_password: &str,
) -> Result<(), AdminError> {
    let min_len = validate::password_min_len_from_env();
    if let Err(e) = validate::validate_signup_input(email, new_password, min_len) {
        return Err(match e {
            // The email came out of our own store, so an
            // `InvalidEmail` here is a data problem, not caller input.
            // Surfacing it as `Validation` would blame the password.
            ValidationError::InvalidEmail(msg) => {
                AdminError::Store(format!("stored email is invalid: {msg}"))
            }
            ValidationError::WeakPassword(msg) => AdminError::Validation(msg),
        });
    }

    let hash = password::hash(new_password).map_err(|_| AdminError::HashFailed)?;
    store
        .set_password_hash(user_id, &hash)
        .await
        .map_err(store_err)
}

/// Map a store failure onto [`AdminError`], preserving the
/// "user not found" wording that HTTP callers match on to return 404.
fn store_err(e: UserStoreError) -> AdminError {
    match e {
        UserStoreError::NotFound => AdminError::Store("user not found".into()),
        UserStoreError::Backend(s) => AdminError::Store(s),
        UserStoreError::Conflict => AdminError::Store("unexpected Conflict on update".into()),
    }
}

/// Set `user_id`'s password with **no** current-password check.
///
/// This is the operator reset path: the caller must have already
/// established that the actor is allowed to reset this user (in the
/// HTTP layer, the `(users, admin)` permission lane). It is also how a
/// third-party-sign-in-only user acquires a local password for the
/// first time — a `NULL` stored hash is not an obstacle here.
///
/// Returns `AdminError::Store("user not found")` when no user has that
/// id, which callers map to 404.
pub async fn set_password<U: UserStore + ?Sized>(
    store: &U,
    user_id: &str,
    new_password: &str,
) -> Result<(), AdminError> {
    let user = store
        .find_by_id(user_id)
        .await
        .map_err(store_err)?
        .ok_or_else(|| AdminError::Store("user not found".into()))?;
    write(store, user_id, &user.email, new_password).await
}

/// Change `user_id`'s password, verifying `current_password` first.
///
/// The self-serve path. Knowledge of the existing password is required
/// so that a stolen session alone cannot rotate the credential.
///
/// A user with no local password (`password_hash IS NULL`) gets
/// [`ChangePasswordError::PasswordNotSet`] rather than a wrong-password
/// error — there is nothing to verify against, and the remedy is an
/// operator [`set_password`], not a retry.
pub async fn change_password<U: UserStore + ?Sized>(
    store: &U,
    user_id: &str,
    current_password: &str,
    new_password: &str,
) -> Result<(), ChangePasswordError> {
    let user = store
        .find_by_id(user_id)
        .await
        .map_err(|e| match e {
            UserStoreError::NotFound => ChangePasswordError::NotFound,
            other => ChangePasswordError::Store(other.to_string()),
        })?
        .ok_or(ChangePasswordError::NotFound)?;

    let existing = user
        .password_hash
        .as_deref()
        .ok_or(ChangePasswordError::PasswordNotSet)?;

    // A malformed stored hash is a store problem, not a wrong
    // password — do not let it read as `WrongPassword`, which would
    // send the user round a retry loop that can never succeed.
    let matches = password::verify(current_password, existing)
        .map_err(|e| ChangePasswordError::Store(format!("stored hash unusable: {e}")))?;
    if !matches {
        return Err(ChangePasswordError::WrongPassword);
    }

    write(store, user_id, &user.email, new_password)
        .await
        .map_err(|e| match e {
            AdminError::Validation(msg) => ChangePasswordError::Validation(msg),
            AdminError::Store(msg) if msg == "user not found" => ChangePasswordError::NotFound,
            other => ChangePasswordError::Store(other.to_string()),
        })
}

/// Failures from [`change_password`].
///
/// Callers are expected to collapse `WrongPassword` and `NotFound`
/// into one indistinguishable response — see `POST /me/password`.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ChangePasswordError {
    /// `current_password` did not match the stored hash.
    #[error("current password is incorrect")]
    WrongPassword,
    /// No user with that id.
    #[error("user not found")]
    NotFound,
    /// The user exists but has no local password (third-party
    /// sign-in only), so there is nothing to verify against.
    #[error("no local password set for this user")]
    PasswordNotSet,
    /// `new_password` failed the strength policy.
    #[error("validation error: {0}")]
    Validation(String),
    /// Hashing or persistence failed.
    #[error("store error: {0}")]
    Store(String),
}
