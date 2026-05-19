//! `POST /auth/logout`. Reads the session cookie, revokes the row,
//! clears the cookie. Returns 204 even if the cookie was missing
//! (idempotent).

// TODO(ap): handler body lands with the session module.
