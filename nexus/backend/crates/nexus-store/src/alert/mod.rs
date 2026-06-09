//! Tenant-scoped alerting persistence: rules + their state, the event history,
//! notification channels, and silences.
//!
//! Every per-tenant function opens a tenant-bound transaction so RLS isolates
//! the rows. The one exception is [`due::claim_due`], the scheduler's
//! cross-tenant claim, which goes through a SECURITY DEFINER function — the
//! single controlled hole a system task needs, not a blanket RLS bypass.

pub mod channel;
pub mod due;
pub mod event;
pub mod record;
pub mod rule;
pub mod silence;

pub use due::{claim_due, DueRule};
pub use record::{
    ChannelRecord, EventRecord, NewChannel, NewEvent, NewRule, NewSilence, RulePatch, RuleRecord,
    RuleState, SilenceRecord,
};
