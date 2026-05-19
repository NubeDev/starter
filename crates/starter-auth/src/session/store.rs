//! DB-backed session store. Sessions live in `starter_auth_sessions`
//! so logout actually invalidates (no JWT-style "valid until expiry"
//! footgun).

/// Read/write interface to the session table. The concrete impl is
/// backed by a `starter_store_*` pool; consumers pick which.
///
/// Stubbed for v0.1 — implementation lands with the auth migrations.
pub struct SessionStore {
    _private: (),
}
