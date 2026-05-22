//! ULID-style id generation backed by `uuid` v7 (time-sortable).
//!
//! SCOPE calls for ULIDs because they sort by creation time, which
//! pages and tails depend on. `uuid` v7 has the same monotonic
//! property and is already a workspace dep, so we use it as the
//! concrete representation. Stored as the standard hyphenated TEXT
//! form so a human reading the table can spot timestamps at a glance.

use starter_spi::changelog::{ChangeId, GroupId};

/// Fresh [`ChangeId`].
pub(crate) fn new_change_id() -> ChangeId {
    ChangeId(uuid::Uuid::now_v7().to_string())
}

/// Fresh [`GroupId`].
pub(crate) fn new_group_id() -> GroupId {
    GroupId(uuid::Uuid::now_v7().to_string())
}
