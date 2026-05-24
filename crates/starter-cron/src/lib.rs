//! # starter-cron
//!
//! Tiny cron primitive used by the durable scheduler in
//! `starter-server`. Wraps the [`cron`] crate so the rest of the
//! workspace only ever sees one helper:
//!
//! ```no_run
//! use chrono::Utc;
//! use starter_cron::next_fire;
//!
//! let when = next_fire(Utc::now(), "0 0 9 * * MON").unwrap();
//! # let _ = when;
//! ```
//!
//! ## Expression format
//!
//! Expressions follow the [`cron`] crate's grammar — **6 or 7 fields**
//! (`sec min hour day-of-month month day-of-week [year]`). This is
//! intentionally one field more than POSIX cron: the scheduler stores
//! second-resolution next-fire timestamps and wants the extra
//! precision.
//!
//! ## Why a wrapper crate
//!
//! Keeping the dependency local means we can swap parsers later
//! (e.g. for a timezone-aware engine) without rippling through every
//! caller — they only know about [`next_fire`] and [`CronError`].

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod error;
mod next_fire;

pub use error::CronError;
pub use next_fire::next_fire;
