//! ULID-style id generation. See sqlite backend for rationale.

use starter_spi::changelog::{ChangeId, GroupId};

pub(crate) fn new_change_id() -> ChangeId {
    ChangeId(uuid::Uuid::now_v7().to_string())
}

pub(crate) fn new_group_id() -> GroupId {
    GroupId(uuid::Uuid::now_v7().to_string())
}
