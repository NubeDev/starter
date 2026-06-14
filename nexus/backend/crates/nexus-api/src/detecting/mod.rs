//! Detection runner: the scheduled analytic engine that turns a stored insight +
//! a query into persistent findings, and — for "alert-type" detections that name
//! notification channels — delivers those findings to webhook/slack/email.
//!
//! The runner runs the insight over the query frame and reconciles findings; a
//! finding opening or auto-resolving is the transition the [`notify`] layer fans
//! out to the detection's channels. This subsumes the old standalone alert
//! subsystem: a threshold "alert" is now a detection whose insight flags the
//! breaching rows and whose `channel_ids` page someone.

pub mod notify;
pub mod run;
pub mod schedule;
