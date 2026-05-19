//! Built-in roles. Three are enough for the common case
//! (reader/writer/admin); consumers needing more wire their own
//! `Authenticator`.

mod kind;

pub use kind::Role;
