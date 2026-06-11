//! Tenant-scoped notification persistence: delivery channels, silence windows,
//! and the per-detection notify-event history.
//!
//! This is the delivery layer the deleted alert subsystem owned, re-homed under
//! detections. A detection lists `channel_ids`; when one of its findings opens or
//! resolves the runner fans that transition out to those channels (unless a
//! [`silence`] covers the detection) and appends an [`event`] row. Every function
//! is tenant-bound so RLS isolates the rows.

pub mod channel;
pub mod event;
pub mod record;
pub mod silence;

pub use record::{
    ChannelRecord, NewChannel, NewNotifyEvent, NewSilence, NotifyEventRecord, SilenceRecord,
};
