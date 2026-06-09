//! Alerting DTOs — rules, events, channels, and silences.

pub mod channel;
pub mod event;
pub mod rule;
pub mod silence;

pub use channel::{ChannelDetail, CreateChannelRequest};
pub use event::AlertEvent;
pub use rule::{AlertRuleDetail, CreateAlertRuleRequest, UpdateAlertRuleRequest};
pub use silence::{CreateSilenceRequest, SilenceDetail};
