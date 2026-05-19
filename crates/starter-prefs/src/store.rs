//! `PrefsStore` trait + sqlite implementation.
//!
//! Owns: SCOPE.md "Preferences model" + the storage entries in
//! "Crate layout". Postgres is deferred to a follow-up job per the
//! Phase 1 decision lock in SCOPE.md "Decisions"; the sqlite impl
//! lands first because the SCOPE "Crate layout" block lists
//! `starter-store-sqlite` explicitly and Postgres only by
//! implication. Empty in stage 3; the trait + sqlite impl land in
//! stage 6 of the user-prefs / i18n job.
